---
name: log-query
description: This skill should be used when the user wants to query, search, analyze, or explore Claude Code session logs stored in ~/.claude/projects/. It handles questions about past sessions, tool usage, token consumption, error investigation, activity timelines, and full-text search across conversation history. Triggers on requests like "show my recent sessions", "search logs for X", "how many tokens did I use", "what errors happened", "show tool usage stats", or any question about past Claude Code activity.
---

# Log Query

## Overview

Query and analyze Claude Code session logs stored in `~/.claude/projects/`. The logs are in JSONL format and contain full conversation history, tool calls, token usage, errors, and session metadata across all projects.

## Quick Start

Run the query script at `scripts/query_logs.py` (relative to this skill directory) using Python 3. No external dependencies required.

```bash
python3 <skill-dir>/scripts/query_logs.py <command> [options]
```

## Commands

### List Projects

Show all projects with session counts and total log size.

```bash
python3 scripts/query_logs.py projects
```

### List Sessions

Show sessions for a project with timestamps, message counts, and first prompts.

```bash
python3 scripts/query_logs.py sessions -p <project-name>
python3 scripts/query_logs.py sessions -p comsa -l 10
```

Options: `-p/--project` (substring match), `-l/--limit`

### Query Messages

View conversation messages with flexible filtering.

```bash
# All user messages from a project
python3 scripts/query_logs.py messages -p ava --user-only

# Messages containing specific text
python3 scripts/query_logs.py messages -g "error" -l 20

# Messages from a specific session
python3 scripts/query_logs.py messages -s 45c09283

# Messages in a date range
python3 scripts/query_logs.py messages --after 2026-02-01 --before 2026-02-05

# Only assistant responses
python3 scripts/query_logs.py messages -p comsa --assistant-only -l 10
```

Options: `-p/--project`, `-s/--session` (ID prefix), `-t/--type`, `--user-only`, `--assistant-only`, `--after/--before` (YYYY-MM-DD), `-g/--grep` (text filter), `-l/--limit`, `-w/--max-width`, `--all-types`

### Tool Usage Statistics

Show which tools were used and how frequently.

```bash
python3 scripts/query_logs.py tools
python3 scripts/query_logs.py tools -p comsa --by-project
```

Options: `-p/--project`, `-l/--limit`, `--by-project`

### Token Usage Statistics

Analyze token consumption across sessions and models.

```bash
python3 scripts/query_logs.py tokens
python3 scripts/query_logs.py tokens -p ava --by-session
```

Options: `-p/--project`, `--by-session`, `-l/--limit`

### Show Errors

List tool execution errors across sessions.

```bash
python3 scripts/query_logs.py errors
python3 scripts/query_logs.py errors -p comsa -l 20
```

Options: `-p/--project`, `-l/--limit`

### Full-Text Search

Search across all conversation content with optional regex.

```bash
python3 scripts/query_logs.py search "database migration"
python3 scripts/query_logs.py search "error|exception" -r -p comsa
python3 scripts/query_logs.py search "TODO" --include-subagents
```

Options: `-p/--project`, `-r/--regex`, `-l/--limit`, `-w/--max-width`, `--include-subagents`

### Session Summaries

View AI-generated session summaries from the session index.

```bash
python3 scripts/query_logs.py summary
python3 scripts/query_logs.py summary -p comsa -l 5
```

Options: `-p/--project`, `-l/--limit`, `--all` (include sessions without summaries)

### Activity Timeline

Show when sessions were created, grouped by time period.

```bash
python3 scripts/query_logs.py timeline
python3 scripts/query_logs.py timeline -g hour
python3 scripts/query_logs.py timeline -p comsa -g month
```

Options: `-p/--project`, `-g/--granularity` (hour/day/month)

## Advanced Usage

### Combining with Shell Tools

For queries not covered by the script, read JSONL files directly using `jq` or grep.

```bash
# Count entries by type in a session
cat ~/.claude/projects/<project>/<session>.jsonl | python3 -c "
import sys, json; from collections import Counter;
c = Counter(json.loads(l).get('type','') for l in sys.stdin);
print('\n'.join(f'{k}: {v}' for k,v in c.most_common()))
"

# Extract all tool names used in a session
grep '"tool_use"' ~/.claude/projects/<project>/<session>.jsonl | python3 -c "
import sys, json
for l in sys.stdin:
    d = json.loads(l)
    for b in d.get('message',{}).get('content',[]):
        if isinstance(b,dict) and b.get('type')=='tool_use':
            print(b['name'], b.get('input',{}).get('command','')[:80])
"
```

### Reading Raw Logs

To inspect raw JSONL entries, use the Read tool on specific session files. Session files are at `~/.claude/projects/<project-dir>/<session-uuid>.jsonl`. Use `sessions-index.json` in each project directory to find session IDs and metadata.

## Resources

### scripts/

- `query_logs.py` - Main query script. Pure Python 3, no external dependencies. Run with `python3`.

### references/

- `log_format.md` - Detailed documentation of the JSONL log format, directory structure, entry types, and field descriptions. Read this file when deeper understanding of the log schema is needed.
