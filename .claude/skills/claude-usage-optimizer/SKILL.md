---
name: claude-usage-optimizer
description: "This skill should be used when the user asks about Claude Code usage limits, subscription plans (Pro, Max 5x, Max 20x), token budgets, rate limits, cost optimization, or how to track and reduce token consumption. Also triggers for questions about ccusage, billing blocks, or scheduling coding sessions to maximize throughput."
---

# Claude Code Usage Optimizer

## Overview

Provide guidance on Claude Code subscription plan limits, token budget management, and strategies to maximize productive usage within rate limit windows. Includes ccusage CLI reference for tracking consumption.

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
- `claude auth status` shows plan type but does not distinguish 5x vs 20x

## Rate Limit Mechanics

### 5-Hour Window
- `/usage` labels this **"Current session"** — this is the 5-hour window, NOT the Claude Code session
- Observed behavior: **discrete 5-hour blocks with fixed reset times** (e.g., 10am-3pm ET), visible via `/usage` as "Resets Xpm"
  - **Concern**: Anthropic officially describes this as a "rolling window" that begins with your first prompt; however ccusage data and `/usage` reset times show fixed block boundaries. Treating as discrete blocks for now.
- `npx ccusage@latest blocks --active` shows current block's token usage, limit, burn rate, and projections
- The `tokenLimitStatus.limit` from ccusage gives the exact 5-hour token limit for your plan
- ccusage and `/usage` may show different percentages (~4% gap) — likely due to an internal **credit-weighted** system where different models consume credits at different rates (Opus ~5x Sonnet)

### Weekly Limits
- A **separate weekly cap** exists on top of the 5-hour window — hitting either one locks you out independently
- Shared across all models and platforms (web + Code)
- Fewer than 2% of Sonnet users hit it; Opus users hit it more often
- Resets on a **fixed 7-day cycle** (visible via `/usage` as "Resets [date]")
- **Weekly token limit is undisclosed and possibly dynamic** — Anthropic does not publish exact numbers, and ccusage does NOT provide it
- Estimated weekly limits (empirical, reverse-calculated from `/usage`): **Max 5x ~300M tokens**
- To reverse-calculate: `weekly_limit = weekly_tokens_from_ccusage / (usage_percent / 100)` — more accurate at higher usage %

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
5. **Monitor with ccusage blocks** — `npx ccusage@latest blocks --active` shows remaining capacity and projections
6. **Use off-peak hours for batch work** — rate limits are per-account, but large batch jobs are better started with a full window

### Track Usage
- **In-session**: `/cost` (API users) or `/stats` (subscription users)
- **Historical**: `npx ccusage@latest daily --breakdown` for per-model costs
- **Current block**: `npx ccusage@latest blocks --active` for live quota status
- **Status line**: configure `ccusage statusline` in Claude Code hooks for continuous monitoring

For full ccusage CLI reference, see `references/ccusage-commands.md`.

## Quick Diagnosis

When hitting rate limits frequently:
1. Run `npx ccusage@latest blocks --recent` to see consumption pattern
2. Run `npx ccusage@latest session --since $(date +%Y%m%d) --order desc` to find expensive sessions
3. Check if web chat is consuming shared quota
4. Consider whether Opus usage can be replaced with Sonnet for some tasks
5. If consistently hitting limits on Max 5x, evaluate upgrading to 20x vs. switching to API billing based on usage patterns
