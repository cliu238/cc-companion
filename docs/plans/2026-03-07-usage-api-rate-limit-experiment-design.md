# Usage API Rate Limit Experiment Design

## Problem

`fetch_oauth_usage()` in `src/app/mod.rs` polls `https://api.anthropic.com/api/oauth/usage` every 5 minutes. The API frequently returns `429 Rate Limited`, but exact limits are unknown (undocumented API). The 5-minute interval was chosen as a guess after 60s caused too many 429s.

Five questions remain unanswered:
1. Rate limit threshold — how many requests per window before 429?
2. Window type — per-minute? per-hour? sliding?
3. Response headers — does the API return `Retry-After` or `X-RateLimit-*`?
4. Safe polling interval — what's the actual minimum?
5. Recovery behavior — how long after 429 until requests succeed again?

## Solution

A standalone Python script (outside cc-companion's Rust codebase) that runs a controlled experiment against the usage API, progressing from conservative to aggressive request patterns.

### Safety-First Design

The biggest risk is not the experiment failing — it's **IP ban or account suspension** from triggering Anthropic's security systems. The entire design prioritizes safety over completeness.

**Fuse (circuit breaker) rules:**
- `403` or `401` → terminate immediately, print warning
- 2 consecutive `429`s → pause 10 minutes
- 5 total `429`s → terminate, output collected data
- Any unexpected status (5xx, connection refused) → terminate

### Experiment Phases (conservative → aggressive)

**Phase 1: Steady-state** (safest, run first)
- Try intervals: 10min, 5min, 2min, 1min (in that order)
- 5 requests per interval
- Goal: find the minimum interval that never triggers 429
- If 10min triggers 429, something is very wrong → terminate

**Phase 1b: Endurance** (validate Phase 1 result)
- Use the minimum safe interval from Phase 1
- Send 20 requests at that interval (e.g., 2min interval → ~40 minutes)
- Goal: confirm the interval is truly stable, not just within a token bucket capacity
- For each request, log rolling stats: requests in last 1min, 5min, 10min
- If 429 appears, record which request number triggered it → infers bucket capacity
- If all 20 succeed → the interval is genuinely safe

**Rate limit model inference** (from endurance data):
- 429 always at request N regardless of timing → **token bucket** (capacity = N)
- 429 near fixed time boundaries (e.g., :00 of each minute) → **fixed window**
- 429 correlates with "requests in last N minutes" → **sliding window**
- Same parameters yield different results at different times → **adaptive** (needs separate runs to confirm)

**Phase 2: Recovery** (moderate risk)
- Using the minimum safe interval from Phase 1, intentionally trigger one 429 (by sending 2 rapid requests)
- Then probe recovery: wait 30s, 1min, 2min, 5min... until success
- Goal: find minimum recovery time after a 429
- Max duration: 30 minutes, then record "inconclusive"

**Phase 3: Burst** (highest risk, opt-in only)
- Only runs with `--enable-burst` flag
- Send requests at 1s intervals until first 429
- Goal: find exact threshold (e.g., "5 requests per minute")
- Hard cap: stop after 10 requests regardless
- Immediately enter 10-min cooldown after

### Long-Running Reliability

The experiment may run for hours (steady-state alone: 4 intervals x 5 requests x 10 min = 200 min worst case). Protections:

1. **Request timeout** — 30s per HTTP request, no hanging on network issues
2. **Per-phase time cap** — each phase has a max duration, logs "inconclusive" on timeout
3. **Append-only log** — each request result written immediately to file (not buffered in memory)
4. **Checkpoint file** — JSON recording current phase/step, enables resume after crash via `--resume`
5. **Periodic summary** — print phase summary after each phase completes, so partial runs still yield data

### Tech Stack

- Python 3.12+, managed with `uv`
- `httpx` for HTTP requests (async, built-in timeout support)
- `asyncio` for timing control
- Reads OAuth token from `~/.claude/.credentials.json`
- Log output: one line per request (timestamp, status, headers, body snippet)
- Checkpoint: `checkpoint.json` in output directory

### Output

```
experiment_output/
  log.txt           # append-only, one line per request
  checkpoint.json   # current phase/step for resume
  summary.txt       # generated after completion or interruption
```

Summary includes: requests sent, successes, 429s, observed headers, recommended polling interval, and inferred rate limit model.

### Cross-Session Validation

A single experiment run cannot detect time-of-day adaptive behavior. To validate:
- Summary records experiment start/end timestamps
- Users should run at different times (e.g., daytime vs. late night) and compare results
- If results differ significantly → adaptive model, recommend using the most conservative result

### File Location

`scripts/usage_rate_limit_experiment/` in the cc-companion repo, with its own `pyproject.toml` (uv project).

## Non-Goals

- Modifying cc-companion's Rust code (that comes after the experiment)
- Testing other Anthropic API endpoints
- Load testing or stress testing
