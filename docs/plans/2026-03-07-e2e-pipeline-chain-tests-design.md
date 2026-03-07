# E2E Pipeline Chain Tests Design

## Problem

Current E2E tests are shallow — they verify mode transitions and help bar text but don't test:
1. Full pipeline chaining: `tick()` → `on_complete()` → enqueue → `spawn_claude()` loop
2. The complete IssueDriven 5-step chain running end-to-end
3. Halt signals (`ISSUES_EMPTY`, `FAILED=`) stopping the chain
4. Key interaction flows (cancel, toggle auto-scheduler, switch pipeline mid-run)

## Solution

Three layers of new tests:

### Layer A: Stateful Mock Pipeline Chain Test

Enhance `tests/fixtures/mock_claude` to be stateful — detect which pipeline step is running from the `-p` prompt arg and return step-specific responses (including halt signals like `ISSUES_EMPTY` on the second cycle).

PTY test flow:
1. Enter chat → open task panel (X) → switch to IssueDriven (p, j, Enter) → submit empty goal (Enter)
2. Manually run step 1 `load-skills` (Enter) → verify "Mock response" appears in chat
3. Verify `on_complete()` enqueued step 2 `run-tests` in task panel
4. Re-open task panel, manually run step 2 → verify step 3 `implement-issue` appears
5. Continue through all 5 steps: `load-skills` → `run-tests` → `implement-issue` → `verify` → `finish`
6. Verify pipeline cycles back, then second cycle halts with `ISSUES_EMPTY`

Mock script behavior:
- Parse `-p` arg for task name keywords (`load-skills`, `run-tests`, etc.)
- Return step-specific result text (e.g., "39 passed; 0 failed" for run-tests)
- On second `load-skills` call (cycle 2), return `ISSUES_EMPTY` to test halt
- Use `$MOCK_CLAUDE_CALL_COUNT` file to track invocation count

### Layer B: Keybinding Flow Tests (mock-based)

Cover interaction flows not currently tested:
- **Cancel running task**: send Esc during `waiting` → verify `[Cancelled]` appears
- **Toggle auto-scheduler**: press `a` → verify scheduler enabled/disabled state
- **New chat session**: press `n` → verify messages cleared
- **Tone cycling**: press `t` → verify tone label changes

### Layer C: Real-API Chained Test (`#[ignore]`)

Extend `test_ignored_4_pipeline_task_executes` to run at least 2-3 pipeline steps with real Claude, verifying each step completes and the next is enqueued. Generous timeouts (300s per step). Run with `--ignored --test-threads=1`.

## Files to Change

1. `tests/fixtures/mock_claude` — add stateful step detection and call counting
2. `tests/e2e.rs` — add new test functions for all three layers

## Key Constraints

- Mock script must remain a simple bash script (no external deps)
- PTY tests use `expectrl` patterns from the test skill (unique substrings, sleep between transitions)
- Real-API tests use `#[ignore]` and `step!()` timing macro
- Cannot spawn `claude` from within Claude Code Bash tool (tests run manually)
