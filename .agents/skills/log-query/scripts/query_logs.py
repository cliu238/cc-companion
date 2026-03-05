#!/usr/bin/env python3
"""
Claude Code Session Log Query Tool

Queries JSONL session logs stored in ~/.claude/projects/.
Supports filtering by project, session, message type, date range,
text search, token usage analysis, and more.

Usage:
    python3 query_logs.py <command> [options]

Commands:
    projects        List all projects
    sessions        List sessions for a project (or all projects)
    messages        Query messages with filters
    tools           Show tool usage statistics
    tokens          Show token usage statistics
    errors          Show errors from tool results
    search          Full-text search across all logs
    summary         Show session summaries
    timeline        Show activity timeline
"""

import json
import os
import sys
import argparse
import glob
import re
from datetime import datetime, timezone
from collections import Counter, defaultdict
from pathlib import Path


PROJECTS_DIR = os.path.expanduser("~/.claude/projects")


def parse_timestamp(ts_str):
    """Parse ISO 8601 timestamp string to datetime."""
    if not ts_str:
        return None
    try:
        # Handle various ISO 8601 formats
        ts_str = ts_str.replace("Z", "+00:00")
        return datetime.fromisoformat(ts_str)
    except (ValueError, TypeError):
        return None


def format_timestamp(ts):
    """Format datetime for display."""
    if not ts:
        return "N/A"
    return ts.strftime("%Y-%m-%d %H:%M:%S")


def get_project_dirs():
    """Get all project directories."""
    if not os.path.isdir(PROJECTS_DIR):
        return []
    return sorted([
        d for d in os.listdir(PROJECTS_DIR)
        if os.path.isdir(os.path.join(PROJECTS_DIR, d)) and not d.startswith(".")
    ])


def project_display_name(dirname):
    """Convert directory name to readable project name."""
    # -Users-ericliu-projects5-foo -> foo
    parts = dirname.split("-")
    # Find the last meaningful segment(s)
    # Typically the pattern is -Users-user-path-projectname
    if len(parts) > 4:
        return "-".join(parts[4:])
    return dirname


def find_sessions(project_dir):
    """Find all session JSONL files in a project directory."""
    sessions = []
    full_path = os.path.join(PROJECTS_DIR, project_dir)

    # Main session files (UUID.jsonl in project root)
    for f in glob.glob(os.path.join(full_path, "*.jsonl")):
        sessions.append(f)

    return sorted(sessions)


def find_subagent_logs(project_dir, session_id):
    """Find subagent logs for a session."""
    full_path = os.path.join(PROJECTS_DIR, project_dir, session_id, "subagents")
    if not os.path.isdir(full_path):
        return []
    return sorted(glob.glob(os.path.join(full_path, "*.jsonl")))


def read_jsonl(filepath, max_lines=None):
    """Read JSONL file and yield parsed entries."""
    try:
        with open(filepath, "r", encoding="utf-8", errors="replace") as f:
            for i, line in enumerate(f):
                if max_lines and i >= max_lines:
                    break
                line = line.strip()
                if not line:
                    continue
                try:
                    yield json.loads(line)
                except json.JSONDecodeError:
                    continue
    except (IOError, OSError):
        pass


def load_session_index(project_dir):
    """Load sessions-index.json for a project."""
    index_path = os.path.join(PROJECTS_DIR, project_dir, "sessions-index.json")
    if not os.path.isfile(index_path):
        return None
    try:
        with open(index_path, "r") as f:
            return json.load(f)
    except (json.JSONDecodeError, IOError):
        return None


def extract_text_content(message):
    """Extract readable text from a message content field."""
    if not message:
        return ""
    content = message.get("content", "")
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        texts = []
        for block in content:
            if isinstance(block, dict):
                if block.get("type") == "text":
                    texts.append(block.get("text", ""))
                elif block.get("type") == "tool_use":
                    name = block.get("name", "unknown")
                    inp = block.get("input", {})
                    if isinstance(inp, dict):
                        cmd = inp.get("command", inp.get("pattern", inp.get("query", "")))
                        texts.append(f"[Tool: {name}] {cmd}")
                    else:
                        texts.append(f"[Tool: {name}]")
                elif block.get("type") == "tool_result":
                    result_content = block.get("content", "")
                    if isinstance(result_content, str):
                        texts.append(result_content[:200])
                    elif isinstance(result_content, list):
                        for sub in result_content:
                            if isinstance(sub, dict) and sub.get("type") == "text":
                                texts.append(sub.get("text", "")[:200])
                elif block.get("type") == "thinking":
                    texts.append(f"[Thinking: {block.get('thinking', '')[:100]}...]")
            elif isinstance(block, str):
                texts.append(block)
        return "\n".join(texts)
    return str(content)[:500]


def get_tool_name(entry):
    """Extract tool name from a tool_use entry."""
    msg = entry.get("message", {})
    content = msg.get("content", [])
    if isinstance(content, list):
        for block in content:
            if isinstance(block, dict) and block.get("type") == "tool_use":
                return block.get("name", "unknown")
    return None


def get_token_usage(entry):
    """Extract token usage from an entry."""
    msg = entry.get("message", {})
    return msg.get("usage", None)


# ── Commands ──────────────────────────────────────────────────────────────

def cmd_projects(args):
    """List all projects."""
    dirs = get_project_dirs()
    if not dirs:
        print("No projects found.")
        return

    print(f"{'Project':<40} {'Sessions':>8} {'Size':>10}")
    print("-" * 62)

    for d in dirs:
        full_path = os.path.join(PROJECTS_DIR, d)
        # Count sessions from index first, fall back to JSONL file count
        index = load_session_index(d)
        if index and "entries" in index:
            session_count = len(index["entries"])
        else:
            session_count = len(find_sessions(d))
        # Calculate total size
        total_size = 0
        for root, _dirs, files in os.walk(full_path):
            for f in files:
                total_size += os.path.getsize(os.path.join(root, f))

        size_str = f"{total_size / 1024:.0f}K" if total_size < 1048576 else f"{total_size / 1048576:.1f}M"
        print(f"{project_display_name(d):<40} {session_count:>8} {size_str:>10}")


def cmd_sessions(args):
    """List sessions for a project."""
    dirs = get_project_dirs()

    # Filter by project if specified
    if args.project:
        dirs = [d for d in dirs if args.project.lower() in project_display_name(d).lower() or args.project.lower() in d.lower()]

    if not dirs:
        print(f"No projects matching '{args.project}'.")
        return

    for project_dir in dirs:
        index = load_session_index(project_dir)
        project_name = project_display_name(project_dir)

        if index and "entries" in index:
            entries = index["entries"]

            # Sort by modified time
            entries.sort(key=lambda e: e.get("modified", ""), reverse=True)

            if args.limit:
                entries = entries[:args.limit]

            print(f"\n=== {project_name} ({len(entries)} sessions) ===")
            print(f"{'ID (short)':<12} {'Created':<20} {'Messages':>8} {'Branch':<15} {'First Prompt'}")
            print("-" * 100)

            for entry in entries:
                sid = entry.get("sessionId", "")[:8]
                created = format_timestamp(parse_timestamp(entry.get("created", "")))
                msg_count = entry.get("messageCount", 0)
                branch = entry.get("gitBranch", "")[:14]
                first_prompt = (entry.get("firstPrompt", "") or "")[:60]
                first_prompt = first_prompt.replace("\n", " ")
                print(f"{sid:<12} {created:<20} {msg_count:>8} {branch:<15} {first_prompt}")
        else:
            # Fallback: read session files directly
            sessions = find_sessions(project_dir)
            print(f"\n=== {project_name} ({len(sessions)} sessions) ===")
            for s in sessions:
                basename = os.path.basename(s)
                size = os.path.getsize(s)
                mtime = datetime.fromtimestamp(os.path.getmtime(s))
                print(f"  {basename[:12]}... {format_timestamp(mtime)} {size/1024:.0f}K")


def cmd_messages(args):
    """Query messages with filters."""
    dirs = get_project_dirs()

    if args.project:
        dirs = [d for d in dirs if args.project.lower() in project_display_name(d).lower() or args.project.lower() in d.lower()]

    if args.session:
        # Find specific session
        for project_dir in dirs:
            sessions = find_sessions(project_dir)
            for s in sessions:
                if args.session in os.path.basename(s):
                    _query_session_messages(s, args, project_display_name(project_dir))
                    return
        print(f"Session '{args.session}' not found.")
        return

    # Query all matching sessions
    count = 0
    for project_dir in dirs:
        sessions = find_sessions(project_dir)
        for s in sessions:
            count += _query_session_messages(s, args, project_display_name(project_dir))
            if args.limit and count >= args.limit:
                return


def _query_session_messages(filepath, args, project_name=""):
    """Query messages from a single session file."""
    count = 0
    for entry in read_jsonl(filepath):
        entry_type = entry.get("type", "")

        # Filter by type
        if args.type and entry_type != args.type:
            continue

        # Filter by date range
        ts = parse_timestamp(entry.get("timestamp", ""))
        if args.after and ts:
            after_dt = parse_timestamp(args.after + "T00:00:00Z")
            if after_dt and ts < after_dt:
                continue
        if args.before and ts:
            before_dt = parse_timestamp(args.before + "T23:59:59Z")
            if before_dt and ts > before_dt:
                continue

        # Filter by text content
        text = extract_text_content(entry.get("message", {}))
        if args.grep and args.grep.lower() not in text.lower():
            continue

        # Filter: only user messages
        if args.user_only and entry_type != "user":
            continue

        # Filter: only assistant messages
        if args.assistant_only and entry_type != "assistant":
            continue

        # Skip progress and file-history-snapshot by default
        if entry_type in ("progress", "file-history-snapshot") and not args.all_types:
            continue

        # Print the message
        ts_str = format_timestamp(ts) if ts else "N/A"
        sid = entry.get("sessionId", "")[:8]

        # Truncate text for display
        display_text = text.replace("\n", "\\n")
        if len(display_text) > args.max_width:
            display_text = display_text[:args.max_width] + "..."

        role = entry.get("message", {}).get("role", entry_type)
        model = entry.get("message", {}).get("model", "")
        model_short = model.split("-")[1] if "-" in model else ""

        prefix = f"[{ts_str}] [{project_name}:{sid}]"
        if model_short:
            prefix += f" [{model_short}]"
        print(f"{prefix} ({role}) {display_text}")

        count += 1
        if args.limit and count >= args.limit:
            return count

    return count


def cmd_tools(args):
    """Show tool usage statistics."""
    dirs = get_project_dirs()

    if args.project:
        dirs = [d for d in dirs if args.project.lower() in project_display_name(d).lower() or args.project.lower() in d.lower()]

    tool_counts = Counter()
    tool_errors = Counter()
    tool_by_project = defaultdict(Counter)

    for project_dir in dirs:
        project_name = project_display_name(project_dir)
        sessions = find_sessions(project_dir)
        for s in sessions:
            for entry in read_jsonl(s):
                tool_name = get_tool_name(entry)
                if tool_name:
                    tool_counts[tool_name] += 1
                    tool_by_project[project_name][tool_name] += 1

                # Check tool results for errors
                msg = entry.get("message", {})
                content = msg.get("content", [])
                if isinstance(content, list):
                    for block in content:
                        if isinstance(block, dict) and block.get("type") == "tool_result":
                            if block.get("is_error"):
                                # Find tool name from linked tool_use
                                tool_errors[block.get("tool_use_id", "unknown")] += 1

    if not tool_counts:
        print("No tool usage found.")
        return

    print(f"\n{'Tool':<30} {'Uses':>8}")
    print("-" * 42)
    for tool, count in tool_counts.most_common(args.limit or 30):
        print(f"{tool:<30} {count:>8}")

    if args.by_project:
        print("\n\n=== Tool Usage by Project ===")
        for project, tools in sorted(tool_by_project.items()):
            print(f"\n--- {project} ---")
            for tool, count in tools.most_common(10):
                print(f"  {tool:<28} {count:>6}")


def cmd_tokens(args):
    """Show token usage statistics."""
    dirs = get_project_dirs()

    if args.project:
        dirs = [d for d in dirs if args.project.lower() in project_display_name(d).lower() or args.project.lower() in d.lower()]

    total_input = 0
    total_output = 0
    total_cache_create = 0
    total_cache_read = 0
    model_usage = defaultdict(lambda: {"input": 0, "output": 0, "cache_create": 0, "cache_read": 0, "calls": 0})
    session_tokens = []

    for project_dir in dirs:
        project_name = project_display_name(project_dir)
        sessions = find_sessions(project_dir)
        for s in sessions:
            session_input = 0
            session_output = 0
            for entry in read_jsonl(s):
                usage = get_token_usage(entry)
                if usage:
                    inp = usage.get("input_tokens", 0)
                    out = usage.get("output_tokens", 0)
                    cc = usage.get("cache_creation_input_tokens", 0)
                    cr = usage.get("cache_read_input_tokens", 0)
                    total_input += inp
                    total_output += out
                    total_cache_create += cc
                    total_cache_read += cr
                    session_input += inp
                    session_output += out

                    model = entry.get("message", {}).get("model", "unknown")
                    model_usage[model]["input"] += inp
                    model_usage[model]["output"] += out
                    model_usage[model]["cache_create"] += cc
                    model_usage[model]["cache_read"] += cr
                    model_usage[model]["calls"] += 1

            if session_input > 0:
                sid = os.path.basename(s)[:8]
                session_tokens.append((project_name, sid, session_input, session_output))

    print("=== Token Usage Summary ===\n")
    print(f"Total input tokens:              {total_input:>12,}")
    print(f"Total output tokens:             {total_output:>12,}")
    print(f"Total cache creation tokens:     {total_cache_create:>12,}")
    print(f"Total cache read tokens:         {total_cache_read:>12,}")
    print(f"Total tokens (input+output):     {total_input + total_output:>12,}")

    if model_usage:
        print(f"\n{'Model':<40} {'Calls':>8} {'Input':>12} {'Output':>10}")
        print("-" * 74)
        for model, stats in sorted(model_usage.items(), key=lambda x: x[1]["input"], reverse=True):
            print(f"{model:<40} {stats['calls']:>8} {stats['input']:>12,} {stats['output']:>10,}")

    if args.by_session and session_tokens:
        print(f"\n\n=== Top Sessions by Token Usage ===")
        session_tokens.sort(key=lambda x: x[2] + x[3], reverse=True)
        print(f"{'Project':<30} {'Session':<10} {'Input':>12} {'Output':>10}")
        print("-" * 66)
        for project, sid, inp, out in session_tokens[:args.limit or 20]:
            print(f"{project:<30} {sid:<10} {inp:>12,} {out:>10,}")


def cmd_errors(args):
    """Show errors from tool results."""
    dirs = get_project_dirs()

    if args.project:
        dirs = [d for d in dirs if args.project.lower() in project_display_name(d).lower() or args.project.lower() in d.lower()]

    error_count = 0
    for project_dir in dirs:
        project_name = project_display_name(project_dir)
        sessions = find_sessions(project_dir)
        for s in sessions:
            for entry in read_jsonl(s):
                msg = entry.get("message", {})
                content = msg.get("content", [])
                if isinstance(content, list):
                    for block in content:
                        if isinstance(block, dict) and block.get("type") == "tool_result" and block.get("is_error"):
                            ts = format_timestamp(parse_timestamp(entry.get("timestamp", "")))
                            sid = entry.get("sessionId", "")[:8]
                            error_text = block.get("content", "")
                            if isinstance(error_text, str):
                                error_text = error_text[:200].replace("\n", "\\n")
                            elif isinstance(error_text, list):
                                texts = [b.get("text", "")[:200] for b in error_text if isinstance(b, dict)]
                                error_text = " | ".join(texts)

                            print(f"[{ts}] [{project_name}:{sid}] {error_text}")
                            error_count += 1

                            if args.limit and error_count >= args.limit:
                                return

    if error_count == 0:
        print("No errors found.")
    else:
        print(f"\nTotal errors: {error_count}")


def cmd_search(args):
    """Full-text search across all logs."""
    dirs = get_project_dirs()

    if args.project:
        dirs = [d for d in dirs if args.project.lower() in project_display_name(d).lower() or args.project.lower() in d.lower()]

    query = args.query.lower()
    if args.regex:
        try:
            pattern = re.compile(args.query, re.IGNORECASE)
        except re.error as e:
            print(f"Invalid regex: {e}")
            return
    else:
        pattern = None

    count = 0
    for project_dir in dirs:
        project_name = project_display_name(project_dir)
        sessions = find_sessions(project_dir)

        # Search main session files
        for s in sessions:
            for entry in read_jsonl(s):
                text = extract_text_content(entry.get("message", {}))
                match = False
                if pattern:
                    match = bool(pattern.search(text))
                else:
                    match = query in text.lower()

                if match:
                    ts = format_timestamp(parse_timestamp(entry.get("timestamp", "")))
                    sid = entry.get("sessionId", "")[:8]
                    role = entry.get("message", {}).get("role", entry.get("type", ""))

                    # Show context around match
                    display_text = text.replace("\n", "\\n")
                    if len(display_text) > args.max_width:
                        # Try to center the match
                        if not pattern:
                            idx = display_text.lower().find(query)
                        else:
                            m = pattern.search(display_text)
                            idx = m.start() if m else 0
                        start = max(0, idx - args.max_width // 2)
                        display_text = "..." + display_text[start:start + args.max_width] + "..."

                    print(f"[{ts}] [{project_name}:{sid}] ({role}) {display_text}")
                    count += 1
                    if args.limit and count >= args.limit:
                        print(f"\n(Showing first {args.limit} results. Use --limit to adjust.)")
                        return

        # Optionally search subagent logs
        if args.include_subagents:
            for s in sessions:
                session_id = os.path.basename(s).replace(".jsonl", "")
                for sub_log in find_subagent_logs(project_dir, session_id):
                    for entry in read_jsonl(sub_log):
                        text = extract_text_content(entry.get("message", {}))
                        match = pattern.search(text) if pattern else query in text.lower()
                        if match:
                            ts = format_timestamp(parse_timestamp(entry.get("timestamp", "")))
                            agent = os.path.basename(sub_log)[:12]
                            role = entry.get("message", {}).get("role", entry.get("type", ""))
                            display_text = text.replace("\n", "\\n")[:args.max_width]
                            print(f"[{ts}] [{project_name}:sub:{agent}] ({role}) {display_text}")
                            count += 1
                            if args.limit and count >= args.limit:
                                return

    print(f"\nTotal matches: {count}")


def cmd_summary(args):
    """Show session summaries."""
    dirs = get_project_dirs()

    if args.project:
        dirs = [d for d in dirs if args.project.lower() in project_display_name(d).lower() or args.project.lower() in d.lower()]

    for project_dir in dirs:
        project_name = project_display_name(project_dir)
        index = load_session_index(project_dir)

        if not index or "entries" not in index:
            continue

        entries = index["entries"]
        entries.sort(key=lambda e: e.get("modified", ""), reverse=True)

        if args.limit:
            entries = entries[:args.limit]

        for entry in entries:
            summary = entry.get("summary", "")
            if not summary and not args.all:
                continue

            sid = entry.get("sessionId", "")[:8]
            created = format_timestamp(parse_timestamp(entry.get("created", "")))
            first_prompt = (entry.get("firstPrompt", "") or "")[:80].replace("\n", " ")

            print(f"\n{'='*80}")
            print(f"Session: {sid}  |  Created: {created}  |  Project: {project_name}")
            print(f"Branch: {entry.get('gitBranch', 'N/A')}  |  Messages: {entry.get('messageCount', 0)}")
            print(f"Prompt: {first_prompt}")
            if summary:
                print(f"\nSummary: {summary}")
            print(f"{'='*80}")


def cmd_timeline(args):
    """Show activity timeline."""
    dirs = get_project_dirs()

    if args.project:
        dirs = [d for d in dirs if args.project.lower() in project_display_name(d).lower() or args.project.lower() in d.lower()]

    activity = defaultdict(lambda: {"sessions": 0, "messages": 0, "projects": set()})

    for project_dir in dirs:
        project_name = project_display_name(project_dir)
        index = load_session_index(project_dir)

        if index and "entries" in index:
            for entry in index["entries"]:
                created = parse_timestamp(entry.get("created", ""))
                if created:
                    if args.granularity == "hour":
                        key = created.strftime("%Y-%m-%d %H:00")
                    elif args.granularity == "day":
                        key = created.strftime("%Y-%m-%d")
                    else:
                        key = created.strftime("%Y-%m")
                    activity[key]["sessions"] += 1
                    activity[key]["messages"] += entry.get("messageCount", 0)
                    activity[key]["projects"].add(project_name)

    if not activity:
        print("No activity found.")
        return

    print(f"{'Period':<20} {'Sessions':>8} {'Messages':>10} {'Projects'}")
    print("-" * 70)
    for period in sorted(activity.keys(), reverse=True):
        data = activity[period]
        projects = ", ".join(sorted(data["projects"]))[:40]
        print(f"{period:<20} {data['sessions']:>8} {data['messages']:>10} {projects}")


def main():
    parser = argparse.ArgumentParser(
        description="Query Claude Code session logs",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__
    )
    subparsers = parser.add_subparsers(dest="command", help="Command to run")

    # projects
    p_projects = subparsers.add_parser("projects", help="List all projects")

    # sessions
    p_sessions = subparsers.add_parser("sessions", help="List sessions")
    p_sessions.add_argument("-p", "--project", help="Filter by project name (substring match)")
    p_sessions.add_argument("-l", "--limit", type=int, help="Limit number of sessions shown")

    # messages
    p_messages = subparsers.add_parser("messages", help="Query messages")
    p_messages.add_argument("-p", "--project", help="Filter by project name")
    p_messages.add_argument("-s", "--session", help="Filter by session ID (prefix match)")
    p_messages.add_argument("-t", "--type", choices=["user", "assistant", "system", "thinking", "progress", "file-history-snapshot", "hook_progress"], help="Filter by message type")
    p_messages.add_argument("--user-only", action="store_true", help="Show only user messages")
    p_messages.add_argument("--assistant-only", action="store_true", help="Show only assistant messages")
    p_messages.add_argument("--after", help="Show messages after date (YYYY-MM-DD)")
    p_messages.add_argument("--before", help="Show messages before date (YYYY-MM-DD)")
    p_messages.add_argument("-g", "--grep", help="Filter by text content (case-insensitive)")
    p_messages.add_argument("-l", "--limit", type=int, default=50, help="Limit results (default: 50)")
    p_messages.add_argument("-w", "--max-width", type=int, default=200, help="Max display width (default: 200)")
    p_messages.add_argument("--all-types", action="store_true", help="Include progress and snapshot entries")

    # tools
    p_tools = subparsers.add_parser("tools", help="Tool usage statistics")
    p_tools.add_argument("-p", "--project", help="Filter by project name")
    p_tools.add_argument("-l", "--limit", type=int, help="Limit number of tools shown")
    p_tools.add_argument("--by-project", action="store_true", help="Break down by project")

    # tokens
    p_tokens = subparsers.add_parser("tokens", help="Token usage statistics")
    p_tokens.add_argument("-p", "--project", help="Filter by project name")
    p_tokens.add_argument("--by-session", action="store_true", help="Show per-session breakdown")
    p_tokens.add_argument("-l", "--limit", type=int, help="Limit sessions shown")

    # errors
    p_errors = subparsers.add_parser("errors", help="Show tool errors")
    p_errors.add_argument("-p", "--project", help="Filter by project name")
    p_errors.add_argument("-l", "--limit", type=int, default=50, help="Limit results")

    # search
    p_search = subparsers.add_parser("search", help="Full-text search")
    p_search.add_argument("query", help="Search query")
    p_search.add_argument("-p", "--project", help="Filter by project name")
    p_search.add_argument("-r", "--regex", action="store_true", help="Use regex pattern")
    p_search.add_argument("-l", "--limit", type=int, default=30, help="Limit results")
    p_search.add_argument("-w", "--max-width", type=int, default=200, help="Max display width")
    p_search.add_argument("--include-subagents", action="store_true", help="Also search subagent logs")

    # summary
    p_summary = subparsers.add_parser("summary", help="Show session summaries")
    p_summary.add_argument("-p", "--project", help="Filter by project name")
    p_summary.add_argument("-l", "--limit", type=int, help="Limit sessions")
    p_summary.add_argument("--all", action="store_true", help="Include sessions without summaries")

    # timeline
    p_timeline = subparsers.add_parser("timeline", help="Activity timeline")
    p_timeline.add_argument("-p", "--project", help="Filter by project name")
    p_timeline.add_argument("-g", "--granularity", choices=["hour", "day", "month"], default="day", help="Time granularity")

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        return

    commands = {
        "projects": cmd_projects,
        "sessions": cmd_sessions,
        "messages": cmd_messages,
        "tools": cmd_tools,
        "tokens": cmd_tokens,
        "errors": cmd_errors,
        "search": cmd_search,
        "summary": cmd_summary,
        "timeline": cmd_timeline,
    }

    commands[args.command](args)


if __name__ == "__main__":
    main()
