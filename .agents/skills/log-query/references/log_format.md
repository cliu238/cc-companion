# Claude Code Session Log Format Reference

## Directory Structure

```
~/.claude/projects/
├── -Users-{user}-{path}-{project}/          # Project directory
│   ├── sessions-index.json                   # Session metadata index
│   ├── {session-uuid}.jsonl                  # Main session log (JSONL)
│   └── {session-uuid}/                       # Session data directory
│       ├── subagents/
│       │   └── agent-{id}.jsonl              # Subagent conversation logs
│       ├── tool-results/
│       │   └── toolu_{id}.txt                # Large tool output files
│       └── session-memory/
│           └── summary.md                    # Session summary document
```

## sessions-index.json

```json
{
  "version": 1,
  "entries": [
    {
      "sessionId": "uuid",
      "fullPath": "/absolute/path/to/session.jsonl",
      "fileMtime": 1769798251378,
      "firstPrompt": "User's first message text",
      "summary": "AI-generated session summary",
      "messageCount": 35,
      "created": "2026-02-05T18:17:03.182Z",
      "modified": "2026-02-05T18:17:03.179Z",
      "gitBranch": "master",
      "projectPath": "/Users/user/projects/project-name",
      "isSidechain": false
    }
  ],
  "originalPath": "/Users/user/projects/project-name"
}
```

## JSONL Entry Types

Each line in a `.jsonl` file is a JSON object. Common fields across all types:

| Field | Description |
|-------|-------------|
| `type` | Entry type: user, assistant, progress, file-history-snapshot, system, thinking, hook_progress |
| `parentUuid` | UUID of parent message (null for root) |
| `uuid` | Unique entry identifier |
| `sessionId` | Session UUID |
| `timestamp` | ISO 8601 timestamp |
| `isSidechain` | true for subagent threads |
| `userType` | "external" for user |
| `cwd` | Working directory |
| `version` | Claude Code version |
| `gitBranch` | Current git branch |

### User Message

```json
{
  "type": "user",
  "message": {
    "role": "user",
    "content": "text or content blocks array"
  },
  "permissionMode": "bypassPermissions"
}
```

### Assistant Message (text response)

```json
{
  "type": "assistant",
  "message": {
    "model": "claude-opus-4-5-20251101",
    "role": "assistant",
    "content": [{"type": "text", "text": "response"}],
    "stop_reason": "end_turn",
    "usage": {
      "input_tokens": 3,
      "output_tokens": 9,
      "cache_creation_input_tokens": 10175,
      "cache_read_input_tokens": 15548
    }
  }
}
```

### Tool Use (within assistant message)

```json
{
  "type": "assistant",
  "message": {
    "content": [{
      "type": "tool_use",
      "id": "toolu_01FHcR2...",
      "name": "Bash",
      "input": {"command": "ls -la", "description": "List files"}
    }]
  }
}
```

Tool names include: Bash, Read, Write, Edit, Grep, Glob, Task, WebFetch, WebSearch, etc.

### Tool Result (within user message)

```json
{
  "type": "user",
  "message": {
    "content": [{
      "type": "tool_result",
      "content": "output text",
      "is_error": false,
      "tool_use_id": "toolu_01FHcR2..."
    }]
  }
}
```

### Token Usage Fields

Found in `message.usage` of assistant entries:

| Field | Description |
|-------|-------------|
| `input_tokens` | Tokens in the input |
| `output_tokens` | Tokens in the output |
| `cache_creation_input_tokens` | Tokens written to cache |
| `cache_read_input_tokens` | Tokens read from cache |

## File Size Ranges

- Session JSONL: 1K - 700K
- Subagent JSONL: 1K - 500K
- Tool result TXT: 1K - 3.5M
- Session memory MD: 200 - 1000 lines
