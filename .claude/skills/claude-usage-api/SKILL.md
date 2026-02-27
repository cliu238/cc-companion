---
name: claude-usage-api
description: "This skill should be used when the user wants to programmatically query Claude Code's OAuth usage API to check rate limit utilization, build statusline integrations, or write scripts/tools that monitor the 5-hour and 7-day usage windows. Triggers on requests involving the Anthropic OAuth usage endpoint, retrieving usage data from the API, displaying usage in terminal statuslines (tmux, starship, etc.), or building automation around Claude Code rate limit data."
---

# Claude Usage API

## Overview

Provide guidance on programmatically querying the Anthropic OAuth usage API (`/api/oauth/usage`) to retrieve real-time rate limit utilization for Claude Code subscriptions. This enables statusline integrations, monitoring dashboards, and automation scripts.

## Quick Start

To fetch current usage from the command line, run the bundled script:

```bash
# Full JSON output
scripts/fetch_usage.sh

# One-line summary (e.g., "5h: 6% (resets 5:00pm) | 7d: 35%")
scripts/fetch_usage.sh --short
```

## Core Workflow

### 1. Retrieve OAuth Token

Claude Code stores credentials in macOS Keychain under `Claude Code-credentials`:

```bash
security find-generic-password -s "Claude Code-credentials" -w
```

This returns JSON containing `claudeAiOauth.accessToken` — the Bearer token for API requests.

### 2. Call the Usage API

```bash
curl -s \
  -H "Accept: application/json" \
  -H "Content-Type: application/json" \
  -H "User-Agent: claude-code/2.0.32" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "anthropic-beta: oauth-2025-04-20" \
  "https://api.anthropic.com/api/oauth/usage"
```

The `anthropic-beta: oauth-2025-04-20` header is required.

### 3. Parse the Response

The response contains `five_hour` and `seven_day` objects, each with:
- `utilization` — percentage used (0-100), matching what `/usage` shows in Claude Code
- `resets_at` — ISO 8601 timestamp for when the window resets

## Common Integration Patterns

### Terminal Statusline (tmux, starship, etc.)

Use `scripts/fetch_usage.sh --short` as a tmux status-right command or starship custom module. Cache the result (e.g., 60s TTL) to avoid excessive API calls.

tmux example:
```
set -g status-right '#(~/.claude/skills/claude-usage-api/scripts/fetch_usage.sh --short)'
set -g status-interval 60
```

### Claude Code Hooks (statusline config)

Configure in `.claude/settings.json` as a PreToolUse or PostToolUse hook to display usage in the Claude Code status area.

### Programmatic Monitoring

For TypeScript/JavaScript implementations, see `references/api_details.md` for type definitions and fetch patterns.

## Important Notes

- This is an **undocumented internal API** — Anthropic may change it without notice
- The OAuth token has an expiry (`expiresAt` field); re-authenticate with `claude auth login` if requests return 401
- macOS only for Keychain retrieval; Linux credential storage differs (check `~/.claude/` for alternatives)
- Rate-limit the polling frequency (every 60s is reasonable) to avoid being flagged

For full API response schema, headers, TypeScript types, and error handling, see `references/api_details.md`.
