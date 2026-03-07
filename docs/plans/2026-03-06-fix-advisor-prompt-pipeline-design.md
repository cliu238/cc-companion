# Fix: Pipeline tasks use Advisor system prompt (Issue #5)

## Problem

`spawn_claude` always injects the Advisor system prompt via `--system-prompt`, including for pipeline execution tasks. The Advisor prompt says "Never write or modify files. Read-only." and "Forbidden: Write, Edit, MultiEdit, TodoWrite", causing Claude to refuse execution tasks even when `read_only: false`.

## Approach: SpawnConfig + conditional system prompt

### Data Structure Changes

**`AutoTask`** — add `use_advisor: bool`:
```rust
pub struct AutoTask {
    pub name: String,
    pub prompt: String,
    pub cwd: String,
    pub read_only: bool,
    pub resume: bool,
    pub setup: Option<String>,
    pub use_advisor: bool,  // NEW
}
```

**New `SpawnConfig`** — replaces `spawn_claude`'s 7-param signature:
```rust
pub(crate) struct SpawnConfig {
    pub msg: String,
    pub resume_session: Option<String>,
    pub read_only: bool,
    pub cwd: Option<String>,
    pub model: Option<String>,
    pub setup: Option<String>,
    pub system_prompt: Option<String>,   // None = skip --system-prompt
    pub gateway: Option<GatewayConfig>,
}

pub(crate) struct GatewayConfig {
    pub url: String,
    pub headers: Option<String>,
}
```

### Core Logic

**`build_claude_cmd(config: &SpawnConfig) -> Command`** — pure function extracted from `spawn_claude` closure:
- `system_prompt: Some(...)` → `cmd.arg("--system-prompt").arg(sp)`
- `system_prompt: None` → no `--system-prompt` arg, Claude Code uses its built-in default
- All other args unchanged

**`spawn_claude`** simplified to: build `SpawnConfig` → call `build_claude_cmd` → spawn thread.

### Callsite Behavior

| Callsite | `system_prompt` |
|----------|----------------|
| `send_chat_message` (user chat) | `Some(advisor_prompt + tone_suffix)` |
| `tick()` auto-task, `use_advisor: true` | `Some(advisor_prompt)` |
| `tick()` auto-task, `use_advisor: false` | `None` |
| Manual task (Enter key in panel) | Same logic as auto-task |

### Pipeline Task Settings

- **`issue_driven.rs`**: all tasks → `use_advisor: false`
- **`example.rs`**: all tasks → `use_advisor: true` (preserves current behavior)

## Testing Strategy

### Layer 1: Unit tests (CI, `src/app/chat.rs`)
- `build_claude_cmd` with `system_prompt: Some(...)` → args contain `--system-prompt`
- `build_claude_cmd` with `system_prompt: None` → args do NOT contain `--system-prompt`
- `read_only: true` → contains `--disallowedTools`
- Gateway on/off → correct env vars

### Layer 2: Mock CLI + PTY tests (CI, `tests/e2e.rs`)
- `tests/fixtures/mock_claude` shell script mimics claude JSON output
- PTY test overrides `PATH` to point `claude` at mock script
- Scenarios:
  - Task panel → run pipeline task → verify response received
  - Mock writes received args to tmpfile → test reads and verifies `--system-prompt` presence/absence

### Layer 3: Real E2E (`#[ignore]`, manual)
- Extend existing `test_ignored_1_claude_round_trip` pattern
- New: execute pipeline task from task panel, verify Claude does not refuse
- Run: `cargo test --test e2e -- --ignored --test-threads=1`

### Skill update
- `.claude/skills/test/SKILL.md`: add "Mock CLI Testing" section

## Files Changed

| File | Change |
|------|--------|
| `src/pipeline/mod.rs` | `AutoTask` add `use_advisor: bool` |
| `src/pipeline/issue_driven.rs` | All tasks: `use_advisor: false` |
| `src/pipeline/example.rs` | All tasks: `use_advisor: true` |
| `src/app/chat.rs` | New `SpawnConfig`, `GatewayConfig`, `build_claude_cmd()`; refactor `spawn_claude` |
| `src/app/mod.rs` | `tick()` and manual task adapt to `SpawnConfig` |
| `tests/e2e.rs` | Mock CLI PTY tests |
| `tests/fixtures/mock_claude` | New mock script |
| `.claude/skills/test/SKILL.md` | Add Mock CLI Testing section |

## Not Changed

`ui.rs`, `main.rs`, `data.rs`, `system_prompt.md`
