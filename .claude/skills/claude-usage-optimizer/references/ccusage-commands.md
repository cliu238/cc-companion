# ccusage CLI Reference

`ccusage` (via `npx ccusage@latest`) analyzes Claude Code local session data to report token usage and costs.

## Commands

### `ccusage daily` / `weekly` / `monthly`
Usage reports grouped by time period. Show cost, token counts per day/week/month.

### `ccusage session`
Usage per conversation session. Useful to identify expensive sessions.
- `-i, --id <id>` — load data for a specific session ID

### `ccusage blocks`
Usage grouped by 5-hour billing blocks. Critical for subscription plan users.
- `-a, --active` — show only the current active block with projections
- `-r, --recent` — show blocks from last 3 days
- `-t, --token-limit <limit>` — token limit for quota warnings (e.g., `500000` or `"max"`)
- `-n, --session-length <hours>` — block duration in hours (default: 5)

### `ccusage statusline`
Compact one-line output for Claude Code hooks / status bars.
- `-B, --visual-burn-rate <mode>` — burn rate display: off | emoji | text | emoji-text
- `--cost-source <source>` — auto | ccusage | cc | both
- `--context-low-threshold <N>` — green threshold % (default: 50)
- `--context-medium-threshold <N>` — yellow threshold % (default: 80)

## Common Options (all commands)
- `-s, --since <YYYYMMDD>` — filter from date
- `-u, --until <YYYYMMDD>` — filter until date
- `-j, --json` — JSON output
- `-b, --breakdown` — per-model cost breakdown
- `-o, --order <asc|desc>` — sort order
- `-i, --instances` — breakdown by project/instance
- `-p, --project <name>` — filter to specific project
- `-q, --jq <expr>` — pipe JSON through jq
- `-z, --timezone <tz>` — timezone (default: system)
- `--compact` — narrow display mode

## Useful Combos

```bash
# Current block usage with projections
npx ccusage@latest blocks --active

# Recent 3-day blocks
npx ccusage@latest blocks --recent

# Today's cost breakdown by model
npx ccusage@latest daily --since $(date +%Y%m%d) --breakdown

# This week per-session costs, expensive first
npx ccusage@latest session --since $(date -v-7d +%Y%m%d) --order desc

# Per-project breakdown this month
npx ccusage@latest monthly --instances
```
