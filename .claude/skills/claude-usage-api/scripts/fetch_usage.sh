#!/usr/bin/env bash
# Fetch Claude Code OAuth usage limits from the Anthropic API.
# Requires: macOS Keychain with "Claude Code-credentials" entry (auto-created by Claude Code login).
#
# Usage:
#   ./fetch_usage.sh          # JSON output
#   ./fetch_usage.sh --short  # One-line summary: "5h: 6% (resets 5:00p) | 7d: 35%"

set -euo pipefail

# --- Retrieve OAuth token from macOS Keychain ---
CREDS_JSON=$(security find-generic-password -s "Claude Code-credentials" -w 2>/dev/null) || {
  echo "Error: Could not read 'Claude Code-credentials' from Keychain." >&2
  echo "Make sure you are logged into Claude Code (claude auth login)." >&2
  exit 1
}

ACCESS_TOKEN=$(echo "$CREDS_JSON" | python3 -c "import sys,json; print(json.load(sys.stdin)['claudeAiOauth']['accessToken'])" 2>/dev/null) || {
  echo "Error: Could not parse accessToken from credentials." >&2
  exit 1
}

# --- Fetch usage data ---
RESPONSE=$(curl -sS --fail-with-body \
  -H "Accept: application/json" \
  -H "Content-Type: application/json" \
  -H "User-Agent: claude-code/2.0.32" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "anthropic-beta: oauth-2025-04-20" \
  "https://api.anthropic.com/api/oauth/usage" 2>/dev/null) || {
  echo "Error: API request failed." >&2
  exit 1
}

# --- Output ---
if [[ "${1:-}" == "--short" ]]; then
  echo "$RESPONSE" | python3 -c "
import sys, json
from datetime import datetime, timezone

d = json.load(sys.stdin)
parts = []

for label, key in [('5h', 'five_hour'), ('7d', 'seven_day')]:
    obj = d.get(key)
    if obj:
        pct = obj.get('utilization', 0)
        resets = obj.get('resets_at')
        if resets:
            t = datetime.fromisoformat(resets).astimezone()
            reset_str = f' (resets {t.strftime(\"%-I:%M%p\").lower()})'
        else:
            reset_str = ''
        parts.append(f'{label}: {pct:.0f}%{reset_str}')

print(' | '.join(parts) if parts else 'No usage data')
"
else
  echo "$RESPONSE" | python3 -m json.tool
fi
