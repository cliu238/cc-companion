"""Usage API rate limit experiment."""

import argparse
import asyncio
import json
import subprocess
import sys
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
