---
name: claude-usage-optimizer
description: "This skill should be used when the user asks about Claude Code usage limits, subscription plans (Pro, Max 5x, Max 20x), token budgets, rate limits, cost optimization, or how to track and reduce token consumption. Also triggers for questions about ccusage, billing blocks, or scheduling coding sessions to maximize throughput."
---

# Claude Code Usage Optimizer

> **Staleness warning**: Specific numbers (plan prices, token limits, message estimates) were last verified Feb 2026. Anthropic may change these without notice. Always cross-check against `claude.com/pricing` and `/usage` output for current values.

## Overview

Provide guidance on Claude Code subscription plan limits, token budget management, and strategies to maximize productive usage within rate limit windows.

## How cc-companion Tracks Usage

cc-companion uses the **Anthropic OAuth Usage API** (`GET https://api.anthropic.com/api/oauth/usage`) to fetch the exact same utilization percentages shown by `/usage` in Claude Code. This replaced the previous ccusage-based approach.

- OAuth token is read from macOS Keychain (`security find-generic-password -s "Claude Code-credentials" -w`)
- Requires the `anthropic-beta: oauth-2025-04-20` header
- Returns `five_hour.utilization` (0-100%) and `seven_day.utilization` (0-100%) — already percentage values
- `five_hour.resets_at` provides the countdown timer
- Refreshes every 60 seconds
- Status line format: `⏱ {countdown} | {pct}% used | 7d: {weekly_pct}%`

For API details, see the `claude-usage-api` skill.

## Plan Comparison

| | Pro ($20/mo) | Max 5x ($100/mo) | Max 20x ($200/mo) |
|---|---|---|---|
| **Usage multiplier** | 1x baseline | 5x Pro | 20x Pro |
| **Context window** | 200K tokens | 200K tokens | 200K tokens |
| **Est. messages/5h** | 10-40 | 50-225 | 200-900 |
| **Daily capacity** | Light use | Full-day coding | Heavy multi-instance |
| **Weekly limit** | Yes (shared) | Yes (shared) | Yes (rarely hit) |

Key facts:
- All plans share a **200K token context window** (same across models)
- Limits are **shared between claude.ai web and Claude Code** — web chat usage eats into Code quota
- If `ANTHROPIC_API_KEY` env var is set, Claude Code uses **API billing** instead of subscription (pay-per-token, no rate limits)

## Rate Limit Mechanics

### 5-Hour Window
- `/usage` labels this **"Current session"** — this is the 5-hour window, NOT the Claude Code session
- The OAuth API returns the exact utilization percentage and reset time
- Observed behavior: **discrete 5-hour blocks with fixed reset times** (e.g., 10am-3pm ET)

### Weekly Limits
- A **separate weekly cap** exists on top of the 5-hour window — hitting either one locks you out independently
- Shared across all models and platforms (web + Code)
- Fewer than 2% of Sonnet users hit it; Opus users hit it more often
- Resets on a **fixed 7-day cycle** (visible via `/usage` as "Resets [date]")

### When Limits Are Hit
- Claude Code shows a countdown timer until capacity returns
- Options: wait for window to slide, upgrade plan tier, enable "extra usage" (overage billing), or switch to API key billing

## Optimization Strategies

### Maximize Token Efficiency
1. **Use `/clear` between unrelated tasks** — stale context inflates every subsequent message cost
2. **Use Sonnet for routine tasks, reserve Opus for complex reasoning** — switch with `/model`
3. **Write specific prompts** — "add validation to login() in auth.ts" not "improve the codebase"
4. **Use plan mode (Shift+Tab)** before complex implementations — prevents expensive re-work
5. **Delegate verbose ops to subagents** — test output, log analysis stay in subagent context
6. **Reduce extended thinking budget** — default 31,999 tokens; lower with `MAX_THINKING_TOKENS=8000` for simpler tasks
7. **Disable unused MCP servers** — each adds tool definitions to every message's context
8. **Keep CLAUDE.md under ~500 lines** — move specialized workflows into skills (loaded on-demand)
9. **Use `/compact` with focus instructions** — e.g., `/compact Focus on code changes and test results`

### Schedule Sessions Strategically
1. **Batch related work** — keep one session for related tasks to benefit from prompt caching; `/clear` only when switching topics
2. **Front-load Opus work** — do complex architecture/planning first while quota is full, then switch to Sonnet for implementation
3. **Stagger multi-instance use** — running 2+ Claude Code instances simultaneously burns quota much faster
4. **Avoid web chat during heavy Code sessions** — they share the same quota pool
5. **Use off-peak hours for batch work** — rate limits are per-account, but large batch jobs are better started with a full window

### Track Usage
- **In-session**: `/usage` shows current 5h and 7d utilization
- **cc-companion status bar**: live percentage updated every 60s via OAuth API
- **Historical**: `npx ccusage@latest daily --breakdown` for per-model costs (ccusage is still useful for historical analysis)

## Quick Diagnosis

When hitting rate limits frequently:
1. Check `/usage` or cc-companion status bar for current 5h and 7d percentages
2. Check if web chat is consuming shared quota
3. Consider whether Opus usage can be replaced with Sonnet for some tasks
4. If consistently hitting limits on Max 5x, evaluate upgrading to 20x vs. switching to API billing based on usage patterns
