"""Usage API rate limit experiment."""

import argparse
import asyncio
import json
import os
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path

import httpx

API_URL = "https://api.anthropic.com/api/oauth/usage"
REQUEST_TIMEOUT = 30.0


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Anthropic usage API rate limit experiment")
    p.add_argument("--enable-burst", action="store_true", help="Enable burst phase (risky)")
    p.add_argument("--resume", action="store_true", help="Resume from checkpoint")
    p.add_argument("--output-dir", default="experiment_output", help="Output directory")
    return p.parse_args()


def read_token() -> str | None:
    """Read OAuth token. Try macOS Keychain first, then credentials file."""
    # Try Keychain (macOS)
    if sys.platform == "darwin":
        try:
            result = subprocess.run(
                ["security", "find-generic-password", "-s", "claude-ai-oauth", "-w"],
                capture_output=True, text=True, timeout=5,
            )
            if result.returncode == 0 and result.stdout.strip():
                return result.stdout.strip()
        except (subprocess.TimeoutExpired, FileNotFoundError):
            pass

    # Fallback: ~/.claude/.credentials.json
    cred_path = Path.home() / ".claude" / ".credentials.json"
    if cred_path.exists():
        data = json.loads(cred_path.read_text())
        return data.get("claudeAiOauth", {}).get("accessToken")

    return None


async def make_request(client: httpx.AsyncClient, token: str) -> dict:
    """Send one request to the usage API. Returns a result dict (never raises)."""
    ts = datetime.now(timezone.utc).isoformat()
    try:
        resp = await client.get(
            API_URL,
            headers={
                "Accept": "application/json",
                "Content-Type": "application/json",
                "User-Agent": "claude-code",
                "Authorization": f"Bearer {token}",
                "anthropic-beta": "oauth-2025-04-20",
            },
            timeout=REQUEST_TIMEOUT,
        )
        # Extract rate limit headers if present
        rl_headers = {
            k: v for k, v in resp.headers.items()
            if k.lower().startswith(("retry-after", "x-ratelimit", "ratelimit"))
        }
        return {
            "timestamp": ts,
            "status": resp.status_code,
            "rate_limit_headers": rl_headers,
            "body_preview": resp.text[:200],
        }
    except httpx.TimeoutException:
        return {"timestamp": ts, "status": 0, "error": "timeout"}
    except httpx.HTTPError as e:
        return {"timestamp": ts, "status": 0, "error": str(e)}


class Logger:
    """Append-only logger that writes each request result to file immediately."""

    def __init__(self, output_dir: str):
        self.dir = Path(output_dir)
        self.dir.mkdir(parents=True, exist_ok=True)
        self.log_path = self.dir / "log.txt"
        self.entries: list[dict] = []

    def reload(self):
        """Reload entries from log file (for --resume)."""
        if self.log_path.exists():
            for line in self.log_path.read_text().splitlines():
                if line.strip():
                    self.entries.append(json.loads(line))
            print(f"  Reloaded {len(self.entries)} entries from {self.log_path}")

    def log(self, phase: str, entry: dict):
        """Log a request result. Writes to file immediately."""
        entry = {**entry, "phase": phase}
        self.entries.append(entry)
        line = json.dumps(entry)
        with open(self.log_path, "a") as f:
            f.write(line + "\n")
        # Also print to console
        status = entry.get("status", "?")
        rl = entry.get("rate_limit_headers", {})
        rl_str = f" headers={rl}" if rl else ""
        print(f"  [{entry['timestamp']}] HTTP {status}{rl_str}")

    def phase_summary(self, phase: str) -> dict:
        """Return summary stats for a phase."""
        phase_entries = [e for e in self.entries if e.get("phase") == phase]
        statuses = [e.get("status", 0) for e in phase_entries]
        return {
            "phase": phase,
            "total": len(phase_entries),
            "success": statuses.count(200),
            "rate_limited": statuses.count(429),
            "errors": len([s for s in statuses if s not in (200, 429)]),
            "headers_seen": [
                e.get("rate_limit_headers", {})
                for e in phase_entries
                if e.get("rate_limit_headers")
            ],
        }

    def rolling_stats(self, window_seconds: float) -> dict:
        """Count requests in the last `window_seconds` based on logged timestamps."""
        now = datetime.now(timezone.utc)
        count = 0
        for e in reversed(self.entries):
            ts = datetime.fromisoformat(e["timestamp"])
            if (now - ts).total_seconds() <= window_seconds:
                count += 1
            else:
                break
        return {"window_s": window_seconds, "count": count}

    def infer_model(self) -> str:
        """Analyze endurance data to infer the rate limit model."""
        endurance = [e for e in self.entries if e.get("phase") == "endurance"]
        if not endurance:
            return "unknown (no endurance data)"

        four29s = [e for e in endurance if e.get("status") == 429]
        if not four29s:
            return "no 429 observed in endurance — interval is safe"

        nums = [e.get("request_num") for e in four29s if e.get("request_num")]
        if nums and len(set(nums)) == 1:
            return f"token_bucket (capacity ~{nums[0] - 1}, 429 always at request #{nums[0]})"

        times = []
        for e in four29s:
            ts = datetime.fromisoformat(e["timestamp"])
            times.append(ts.second)
        if times and max(times) - min(times) <= 5:
            return f"fixed_window (429s cluster near second {int(sum(times)/len(times))})"

        rolling_counts = [e.get("rolling_1min", 0) for e in four29s]
        if rolling_counts and len(set(rolling_counts)) == 1:
            return f"sliding_window (429 when requests_in_1min >= {rolling_counts[0]})"

        return "inconclusive (429 pattern doesn't match known models)"

    def write_summary(self):
        """Write final summary to summary.txt."""
        summary_path = self.dir / "summary.txt"
        phases = sorted(set(e.get("phase", "") for e in self.entries))
        lines = ["=== Usage API Rate Limit Experiment Summary ===\n"]
        if self.entries:
            lines.append(f"Experiment start: {self.entries[0].get('timestamp', '?')}")
            lines.append(f"Experiment end:   {self.entries[-1].get('timestamp', '?')}")
        for phase in phases:
            s = self.phase_summary(phase)
            lines.append(f"\n--- {phase} ---")
            lines.append(f"  Requests: {s['total']}")
            lines.append(f"  200 OK:   {s['success']}")
            lines.append(f"  429:      {s['rate_limited']}")
            lines.append(f"  Errors:   {s['errors']}")
            if s["headers_seen"]:
                lines.append(f"  Rate limit headers observed: {s['headers_seen']}")
        model = self.infer_model()
        lines.append(f"\n--- Rate Limit Model Inference ---")
        lines.append(f"  {model}")
        lines.append(f"\nNote: Run at different times of day to check for adaptive behavior.")
        summary_path.write_text("\n".join(lines) + "\n")
        print(f"\nSummary written to {summary_path}")


class CircuitBreaker:
    """Safety fuse that terminates experiment on dangerous signals."""

    def __init__(self):
        self.consecutive_429s = 0
        self.total_429s = 0
        self.tripped = False
        self.trip_reason = ""

    def record(self, status: int) -> str | None:
        """Record a status code. Returns action: None, 'pause', or 'terminate'."""
        if status in (401, 403):
            self.tripped = True
            self.trip_reason = f"HTTP {status} — possible account issue, terminating immediately"
            return "terminate"

        if status >= 500 or status == 0:
            self.tripped = True
            self.trip_reason = f"HTTP {status} — server error or no response, terminating"
            return "terminate"

        if status == 429:
            self.consecutive_429s += 1
            self.total_429s += 1
            if self.total_429s >= 5:
                self.tripped = True
                self.trip_reason = f"Total 429s reached {self.total_429s}, terminating"
                return "terminate"
            if self.consecutive_429s >= 2:
                return "pause"
            return None

        # Success — reset consecutive counter
        self.consecutive_429s = 0
        return None


class Checkpoint:
    """Tracks experiment progress for resume after crash."""

    def __init__(self, output_dir: str):
        self.path = Path(output_dir) / "checkpoint.json"
        self.state: dict = {"phase": "steady_state", "step": 0, "completed_phases": []}

    def load(self) -> bool:
        """Load checkpoint. Returns True if found."""
        if self.path.exists():
            self.state = json.loads(self.path.read_text())
            return True
        return False

    def save(self):
        # Atomic write: temp file + rename to avoid corruption on crash
        tmp = self.path.with_suffix(".tmp")
        tmp.write_text(json.dumps(self.state, indent=2) + "\n")
        os.rename(tmp, self.path)

    def set_phase(self, phase: str, step: int = 0):
        self.state["phase"] = phase
        self.state["step"] = step
        self.save()

    def complete_phase(self, phase: str):
        if phase not in self.state["completed_phases"]:
            self.state["completed_phases"].append(phase)
        self.save()

    def is_completed(self, phase: str) -> bool:
        return phase in self.state["completed_phases"]


# Phase constants
STEADY_STATE_INTERVALS = [600, 300, 120, 60]  # seconds: 10min, 5min, 2min, 1min
STEADY_STATE_REQUESTS_PER_INTERVAL = 5
STEADY_STATE_MAX_DURATION = 4 * 3600  # 4 hours hard cap
PAUSE_DURATION = 600  # 10 minutes


async def phase_steady_state(
    client: httpx.AsyncClient,
    token: str,
    logger: Logger,
    breaker: CircuitBreaker,
    checkpoint: Checkpoint,
) -> int | None:
    """Run steady-state phase. Returns minimum safe interval in seconds, or None."""
    phase = "steady_state"
    print(f"\n{'='*50}")
    print(f"Phase 1: Steady-State")
    print(f"Testing intervals: {[f'{s}s' for s in STEADY_STATE_INTERVALS]}")
    print(f"{'='*50}")

    start = asyncio.get_event_loop().time()
    min_safe_interval = None

    for interval in STEADY_STATE_INTERVALS:
        if breaker.tripped:
            break

        elapsed = asyncio.get_event_loop().time() - start
        if elapsed > STEADY_STATE_MAX_DURATION:
            print(f"  Phase time cap reached ({STEADY_STATE_MAX_DURATION}s)")
            break

        print(f"\n  Testing interval: {interval}s ({interval // 60}min)")
        all_ok = True

        for i in range(STEADY_STATE_REQUESTS_PER_INTERVAL):
            if i > 0:
                print(f"  Waiting {interval}s...")
                await asyncio.sleep(interval)

            result = await make_request(client, token)
            logger.log(phase, {**result, "interval": interval, "request_num": i + 1})

            action = breaker.record(result.get("status", 0))
            if action == "terminate":
                print(f"  FUSE TRIPPED: {breaker.trip_reason}")
                break
            if action == "pause":
                print(f"  2 consecutive 429s — pausing {PAUSE_DURATION}s")
                await asyncio.sleep(PAUSE_DURATION)
                all_ok = False
                break

            if result.get("status") == 429:
                all_ok = False
                print(f"  429 at interval {interval}s — this interval is too aggressive")
                break

        if all_ok and not breaker.tripped:
            min_safe_interval = interval
            print(f"  ✓ Interval {interval}s is safe ({STEADY_STATE_REQUESTS_PER_INTERVAL}/{STEADY_STATE_REQUESTS_PER_INTERVAL} succeeded)")
        elif not all_ok:
            print(f"  ✗ Interval {interval}s triggered rate limit — stopping here")
            break

    summary = logger.phase_summary(phase)
    print(f"\n  Phase 1 summary: {summary['total']} requests, {summary['success']} OK, {summary['rate_limited']} 429s")
    if min_safe_interval:
        print(f"  Minimum safe interval found: {min_safe_interval}s")
    else:
        print(f"  No safe interval found!")

    checkpoint.complete_phase(phase)
    return min_safe_interval


ENDURANCE_REQUESTS = 20
ENDURANCE_MAX_DURATION = 2 * 3600  # 2 hours hard cap


async def phase_endurance(
    client: httpx.AsyncClient,
    token: str,
    logger: Logger,
    breaker: CircuitBreaker,
    checkpoint: Checkpoint,
    interval: int,
) -> dict:
    """Run endurance phase at the given interval. Returns analysis dict."""
    phase = "endurance"
    print(f"\n{'='*50}")
    print(f"Phase 1b: Endurance")
    print(f"Sending {ENDURANCE_REQUESTS} requests at {interval}s intervals")
    print(f"Estimated duration: {ENDURANCE_REQUESTS * interval // 60} minutes")
    print(f"{'='*50}")

    start = asyncio.get_event_loop().time()
    first_429_at = None

    for i in range(ENDURANCE_REQUESTS):
        if breaker.tripped:
            break

        elapsed = asyncio.get_event_loop().time() - start
        if elapsed > ENDURANCE_MAX_DURATION:
            print(f"  Phase time cap reached ({ENDURANCE_MAX_DURATION}s)")
            break

        if i > 0:
            print(f"  Waiting {interval}s... ({i}/{ENDURANCE_REQUESTS})")
            await asyncio.sleep(interval)

        result = await make_request(client, token)

        # Add rolling stats for model inference
        rolling_1m = logger.rolling_stats(60)["count"]
        rolling_5m = logger.rolling_stats(300)["count"]
        rolling_10m = logger.rolling_stats(600)["count"]

        logger.log(phase, {
            **result,
            "request_num": i + 1,
            "interval": interval,
            "rolling_1min": rolling_1m,
            "rolling_5min": rolling_5m,
            "rolling_10min": rolling_10m,
        })

        action = breaker.record(result.get("status", 0))
        if action == "terminate":
            print(f"  FUSE TRIPPED: {breaker.trip_reason}")
            break
        if action == "pause":
            print(f"  2 consecutive 429s — pausing {PAUSE_DURATION}s")
            await asyncio.sleep(PAUSE_DURATION)

        if result.get("status") == 429 and first_429_at is None:
            first_429_at = i + 1
            print(f"  First 429 at request #{i + 1} — interval {interval}s is NOT fully safe")
            # Don't break — keep going to collect more data points for model inference
            # But increase interval to avoid burning through circuit breaker
            interval = max(interval, 300)
            print(f"  Backing off to {interval}s for remaining requests")

    summary = logger.phase_summary(phase)
    model = logger.infer_model()
    print(f"\n  Phase 1b summary: {summary['total']} requests, {summary['success']} OK, {summary['rate_limited']} 429s")
    print(f"  Inferred model: {model}")

    if first_429_at:
        print(f"  WARNING: 429 appeared at request #{first_429_at} — Phase 1 result was overfit")
    else:
        print(f"  All {summary['total']} requests succeeded — interval {interval}s is confirmed safe")

    checkpoint.complete_phase(phase)
    return {
        "first_429_at": first_429_at,
        "total_requests": summary["total"],
        "model": model,
    }


async def run(args: argparse.Namespace) -> None:
    print(f"Output dir: {args.output_dir}")
    print("TODO: implement phases")


def main():
    args = parse_args()
    asyncio.run(run(args))


def _test_read_token():
    token = read_token()
    assert token is not None, "No token found"
    assert len(token) > 20, f"Token too short: {len(token)}"
    print(f"Token found: {token[:8]}...{token[-4:]}")


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "test":
        _test_read_token()
        sys.exit(0)
    main()
