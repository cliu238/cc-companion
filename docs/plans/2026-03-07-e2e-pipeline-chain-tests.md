# E2E Pipeline Chain Tests Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add deep E2E tests that verify the full IssueDriven pipeline chain, keybinding interaction flows, and real-API round-trips.

**Architecture:** Enhance the mock `claude` bash script to be stateful (detect pipeline step from `-p` arg, track call count). Write PTY tests using `expectrl` that drive the TUI through the full pipeline chain and verify each step chains correctly. Add keybinding flow tests for cancel, new session, and tone. Add a real-API `#[ignore]` test for multi-step pipeline execution.

**Tech Stack:** Rust, expectrl (PTY), bash (mock script), ratatui TestBackend (unit)

---

### Task 1: Stateful Mock Claude Script

**Files:**
- Modify: `tests/fixtures/mock_claude`

**Step 1: Read the current mock script to understand its structure**

Already read. Current script: writes `$@` to `$MOCK_CLAUDE_ARGS_FILE`, extracts `-p` message, returns fixed JSON with `"Mock response to: $MSG"`.

**Step 2: Write the enhanced mock script**

Replace the mock script with a stateful version that:
- Tracks call count via `$MOCK_CLAUDE_CALL_COUNT` env var pointing to a counter file
- Detects pipeline step keywords in the `-p` arg
- Returns step-specific output text that `on_complete()` uses to chain
- On the 6th call (second cycle's `load-skills`), returns `ISSUES_EMPTY` to halt

```bash
#!/bin/bash
# Mock claude CLI for testing. Stateful: detects pipeline step from -p arg,
# tracks call count, returns step-specific responses.

# Write all args for assertion
if [ -n "$MOCK_CLAUDE_ARGS_FILE" ]; then
    echo "$@" >> "$MOCK_CLAUDE_ARGS_FILE"
fi

# Track call count
COUNT=0
if [ -n "$MOCK_CLAUDE_CALL_COUNT" ]; then
    if [ -f "$MOCK_CLAUDE_CALL_COUNT" ]; then
        COUNT=$(cat "$MOCK_CLAUDE_CALL_COUNT")
    fi
    COUNT=$((COUNT + 1))
    echo "$COUNT" > "$MOCK_CLAUDE_CALL_COUNT"
fi

# Extract the message from -p argument
MSG=""
while [ $# -gt 0 ]; do
    case "$1" in
        -p) MSG="$2"; shift 2 ;;
        *) shift ;;
    esac
done

SESSION_ID="mock-session-001"

# Detect pipeline step and return step-specific result
RESULT="Mock response to: $MSG"
case "$MSG" in
    *"Load"*"/domain-knowledge"*|*"load-skills"*|*"Load"*"skills"*)
        if [ "$COUNT" -ge 6 ]; then
            RESULT="ISSUES_EMPTY"
        else
            RESULT="Skills loaded successfully"
        fi
        ;;
    *"test"*"skill"*|*"run-tests"*|*"Run"*"test"*|*"run all tests"*)
        RESULT="39 passed; 0 failed"
        ;;
    *"verification-before-completion"*"GitHub issue"*|*"implement-issue"*|*"fetch the next"*)
        RESULT="PR #42 created for issue #7"
        ;;
    *"verification-before-completion"*"full test suite"*|*"verify"*)
        RESULT="All 42 tests pass"
        ;;
    *"finishing-a-development-branch"*|*"finish"*)
        RESULT="Branch merged and cleaned up"
        ;;
esac

cat <<ENDJSON
[
  {"type":"init","session_id":"$SESSION_ID"},
  {"type":"result","session_id":"$SESSION_ID","result":"$RESULT"}
]
ENDJSON
```

Key design decisions:
- `>>` (append) for args file so all calls are recorded, not just the last one
- Pattern matching on prompt substrings that appear in `issue_driven.rs` prompts
- Call count threshold of 6 = after 5 steps complete, the 6th call (cycle 2 `load-skills`) returns `ISSUES_EMPTY`

**Step 3: Verify the script is executable**

Run: `chmod +x tests/fixtures/mock_claude && echo "ok"`
Expected: `ok`

**Step 4: Commit**

```bash
git add tests/fixtures/mock_claude
git commit -m "feat: make mock_claude stateful for pipeline chain testing"
```

---

### Task 2: Full Pipeline Chain PTY Test (mock-based)

**Files:**
- Modify: `tests/e2e.rs`

**Step 1: Write the full chain test**

This test drives the TUI through all 5 IssueDriven steps using manual execution (Enter in task panel). After each step completes, it re-opens the task panel and verifies the next step was enqueued.

The key challenge: after running a task (Enter in task panel), the task panel closes. We need to close the panel (Esc), wait for the mock to complete and `tick()` to process `on_complete()`, then re-open the panel (X) to see the next task.

What to `expect()` after each step:
- Task panel title shows `"Issue-Driven"` and pending count changes
- After running a task, chat shows the result text (e.g., `"Skills loaded"`)
- After `on_complete()`, new task name appears in pending list (e.g., `"run-tests"`)

```rust
#[test]
fn test_mock_pipeline_full_chain() {
    let t = Instant::now();
    let (mut app, args_file) = spawn_app_with_mock_claude(30);

    // Also set up call count file
    let count_file = std::env::temp_dir().join(format!(
        "mock_claude_count_{}",
        std::process::id()
    ));
    // Need to respawn with MOCK_CLAUDE_CALL_COUNT set — refactor spawn helper

    step!(t, "selecting project");
    app.expect("Select Project").unwrap();
    app.send("\r").unwrap();
    app.expect("i=type").unwrap();
    std::thread::sleep(Duration::from_millis(500));

    step!(t, "opening task panel and switching to IssueDriven");
    app.send("X").unwrap();
    app.expect("p=pipeline").unwrap();
    app.send("p").unwrap();
    std::thread::sleep(Duration::from_millis(300));
    app.send("j").unwrap();
    std::thread::sleep(Duration::from_millis(200));
    app.send("\r").unwrap();
    // Goal input — submit empty
    app.expect("Esc=cancel").unwrap();
    app.send("\r").unwrap();
    app.expect("d=delete").unwrap();

    // --- Step 1: load-skills ---
    step!(t, "running load-skills");
    app.expect("load-skills").unwrap();
    app.send("\r").unwrap();  // Run the pending task
    // Close panel to see chat
    std::thread::sleep(Duration::from_millis(500));
    app.send("\x1b").unwrap();
    // Wait for mock response
    app.expect("Skills loaded").expect("load-skills didn't complete");

    // Re-open panel to verify run-tests was enqueued
    step!(t, "verifying run-tests enqueued");
    std::thread::sleep(Duration::from_millis(500));
    app.send("X").unwrap();
    app.expect("run-tests").expect("run-tests not enqueued after load-skills");

    // --- Step 2: run-tests ---
    step!(t, "running run-tests");
    // Navigate to the pending task (past done items)
    // After load-skills completes: done=[load-skills], pending=[run-tests]
    // selected_idx may be on done item, press j to move to pending
    app.send("j").unwrap();
    std::thread::sleep(Duration::from_millis(200));
    app.send("\r").unwrap();
    std::thread::sleep(Duration::from_millis(500));
    app.send("\x1b").unwrap();
    app.expect("39 passed").expect("run-tests didn't complete");

    // Re-open panel to verify implement-issue was enqueued
    step!(t, "verifying implement-issue enqueued");
    std::thread::sleep(Duration::from_millis(500));
    app.send("X").unwrap();
    app.expect("implement-issue").expect("implement-issue not enqueued after run-tests");

    // --- Step 3: implement-issue ---
    step!(t, "running implement-issue");
    // done=[load-skills, run-tests], pending=[implement-issue] → need to navigate past done
    app.send("j").unwrap();
    std::thread::sleep(Duration::from_millis(100));
    app.send("j").unwrap();
    std::thread::sleep(Duration::from_millis(200));
    app.send("\r").unwrap();
    std::thread::sleep(Duration::from_millis(500));
    app.send("\x1b").unwrap();
    app.expect("PR #42").expect("implement-issue didn't complete");

    // --- Step 4: verify ---
    step!(t, "verifying verify enqueued and running");
    std::thread::sleep(Duration::from_millis(500));
    app.send("X").unwrap();
    app.expect("verify").expect("verify not enqueued");
    // Navigate past 3 done items to pending
    app.send("j").unwrap();
    std::thread::sleep(Duration::from_millis(100));
    app.send("j").unwrap();
    std::thread::sleep(Duration::from_millis(100));
    app.send("j").unwrap();
    std::thread::sleep(Duration::from_millis(200));
    app.send("\r").unwrap();
    std::thread::sleep(Duration::from_millis(500));
    app.send("\x1b").unwrap();
    app.expect("All 42 tests").expect("verify didn't complete");

    // --- Step 5: finish ---
    step!(t, "verifying finish enqueued and running");
    std::thread::sleep(Duration::from_millis(500));
    app.send("X").unwrap();
    app.expect("finish").expect("finish not enqueued");
    // Navigate past 4 done items
    app.send("j").unwrap();
    std::thread::sleep(Duration::from_millis(100));
    app.send("j").unwrap();
    std::thread::sleep(Duration::from_millis(100));
    app.send("j").unwrap();
    std::thread::sleep(Duration::from_millis(100));
    app.send("j").unwrap();
    std::thread::sleep(Duration::from_millis(200));
    app.send("\r").unwrap();
    std::thread::sleep(Duration::from_millis(500));
    app.send("\x1b").unwrap();
    app.expect("Branch merged").expect("finish didn't complete");

    // --- Verify cycle: pipeline should have restarted with new load-skills ---
    step!(t, "verifying pipeline cycled back");
    std::thread::sleep(Duration::from_millis(500));
    app.send("X").unwrap();
    // After finish completes and pipeline cycles, new load-skills should be pending
    // The title should show pending count > 0
    app.expect("1 pending").expect("pipeline didn't cycle back");

    step!(t, "quitting");
    app.send("\x1b").unwrap();
    std::thread::sleep(Duration::from_millis(300));
    app.send("q").unwrap();
    app.expect(Eof).unwrap();
    step!(t, "done");

    // Verify all 5 steps were called
    let args = std::fs::read_to_string(&args_file).unwrap_or_default();
    let call_count = args.lines().count();
    assert!(call_count >= 5, "expected at least 5 mock calls, got {}", call_count);

    let _ = std::fs::remove_file(&args_file);
    let _ = std::fs::remove_file(&count_file);
}
```

**Step 2: Refactor `spawn_app_with_mock_claude` to also set `MOCK_CLAUDE_CALL_COUNT`**

Update the helper to return the count file path too:

```rust
fn spawn_app_with_mock_claude(timeout_secs: u64) -> (OsSession, std::path::PathBuf, std::path::PathBuf) {
    let bin = env!("CARGO_BIN_EXE_cc-companion");
    let mock_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let args_file = std::env::temp_dir().join(format!(
        "mock_claude_args_{}_{}", std::process::id(), id
    ));
    let count_file = std::env::temp_dir().join(format!(
        "mock_claude_count_{}_{}", std::process::id(), id
    ));

    let original_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", mock_dir.display(), original_path);

    let mut cmd = Command::new(bin);
    cmd.env_remove("CLAUDECODE");
    cmd.env("PATH", &new_path);
    cmd.env("MOCK_CLAUDE_ARGS_FILE", &args_file);
    cmd.env("MOCK_CLAUDE_CALL_COUNT", &count_file);

    let mut session = Session::spawn(cmd).expect("failed to spawn with mock");
    session.set_expect_timeout(Some(Duration::from_secs(timeout_secs)));
    (session, args_file, count_file)
}
```

**Important:** Update existing tests `test_mock_chat_includes_system_prompt` and `test_mock_pipeline_task_skips_system_prompt` to destructure the new 3-tuple return.

**Step 3: Run existing tests to verify refactor didn't break anything**

Run: `cargo test --test e2e -- --test-threads=1 2>&1 | tail -20`
Expected: All 10 non-ignored tests pass

**Step 4: Run the new chain test**

Run: `cargo test --test e2e test_mock_pipeline_full_chain -- --test-threads=1 --nocapture 2>&1`
Expected: PASS with step timing output showing all 5 steps complete

**Step 5: Commit**

```bash
git add tests/e2e.rs
git commit -m "feat: add full pipeline chain E2E test with stateful mock"
```

---

### Task 3: Cancel Running Task Test

**Files:**
- Modify: `tests/e2e.rs`

**Step 1: Write the cancel test**

This test sends a chat message (which triggers a mock `claude` call), then presses Esc to cancel, and verifies `[Cancelled]` appears.

Challenge: the mock returns instantly, so we need a way to make it slow. Add a `MOCK_CLAUDE_DELAY` env var to the mock script that sleeps before responding.

**Step 1a: Add delay support to mock script**

Add at the top of the mock script, after arg parsing:

```bash
# Optional delay for testing cancel
if [ -n "$MOCK_CLAUDE_DELAY" ]; then
    sleep "$MOCK_CLAUDE_DELAY"
fi
```

**Step 1b: Write spawn helper with delay**

```rust
fn spawn_app_with_slow_mock(timeout_secs: u64, delay_secs: u64) -> (OsSession, std::path::PathBuf, std::path::PathBuf) {
    let bin = env!("CARGO_BIN_EXE_cc-companion");
    let mock_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let args_file = std::env::temp_dir().join(format!(
        "mock_claude_args_{}_{}", std::process::id(), id
    ));
    let count_file = std::env::temp_dir().join(format!(
        "mock_claude_count_{}_{}", std::process::id(), id
    ));

    let original_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", mock_dir.display(), original_path);

    let mut cmd = Command::new(bin);
    cmd.env_remove("CLAUDECODE");
    cmd.env("PATH", &new_path);
    cmd.env("MOCK_CLAUDE_ARGS_FILE", &args_file);
    cmd.env("MOCK_CLAUDE_CALL_COUNT", &count_file);
    cmd.env("MOCK_CLAUDE_DELAY", delay_secs.to_string());

    let mut session = Session::spawn(cmd).expect("failed to spawn with slow mock");
    session.set_expect_timeout(Some(Duration::from_secs(timeout_secs)));
    (session, args_file, count_file)
}
```

**Step 1c: Write the test**

```rust
#[test]
fn test_cancel_running_task() {
    let (mut app, args_file, count_file) = spawn_app_with_slow_mock(15, 10);
    app.expect("Select Project").unwrap();
    app.send("\r").unwrap();
    app.expect("i=type").unwrap();
    std::thread::sleep(Duration::from_millis(500));

    // Enter input mode and send a message
    app.send("i").unwrap();
    app.expect("Alt+Enter").unwrap();
    app.send("hello\r").unwrap();

    // Wait for Thinking... to appear (proves claude is running)
    app.expect("Thinking").expect("chat didn't start thinking");

    // Cancel with Esc
    std::thread::sleep(Duration::from_millis(500));
    app.send("\x1b").unwrap();
    app.expect("Cancelled").expect("cancel message not shown");

    app.send("q").unwrap();
    app.expect(Eof).unwrap();

    let _ = std::fs::remove_file(&args_file);
    let _ = std::fs::remove_file(&count_file);
}
```

**Step 2: Run the test**

Run: `cargo test --test e2e test_cancel_running_task -- --test-threads=1 --nocapture`
Expected: PASS

**Step 3: Commit**

```bash
git add tests/fixtures/mock_claude tests/e2e.rs
git commit -m "feat: add cancel running task E2E test"
```

---

### Task 4: New Chat Session Test

**Files:**
- Modify: `tests/e2e.rs`

**Step 1: Write the test**

Press `n` to clear the chat session after sending a mock message, verify old messages are gone.

```rust
#[test]
fn test_new_chat_session_clears_messages() {
    let (mut app, args_file, count_file) = spawn_app_with_mock_claude(15);
    app.expect("Select Project").unwrap();
    app.send("\r").unwrap();
    app.expect("i=type").unwrap();
    std::thread::sleep(Duration::from_millis(500));

    // Send a message
    app.send("i").unwrap();
    app.expect("Alt+Enter").unwrap();
    app.send("hello\r").unwrap();
    app.expect("Mock response").expect("mock didn't respond");

    // Press 'n' to start new session
    app.send("n").unwrap();
    std::thread::sleep(Duration::from_millis(500));

    // The chat should now show "new session" in title bar
    app.expect("new session").expect("new session not shown after 'n'");

    app.send("q").unwrap();
    app.expect(Eof).unwrap();
    let _ = std::fs::remove_file(&args_file);
    let _ = std::fs::remove_file(&count_file);
}
```

**Step 2: Run the test**

Run: `cargo test --test e2e test_new_chat_session -- --test-threads=1 --nocapture`
Expected: PASS

**Step 3: Commit**

```bash
git add tests/e2e.rs
git commit -m "feat: add new chat session E2E test"
```

---

### Task 5: Real-API Multi-Step Pipeline Test (`#[ignore]`)

**Files:**
- Modify: `tests/e2e.rs`

**Step 1: Write the test**

This test runs the first 2 steps of the IssueDriven pipeline with real Claude. It uses `#[ignore]` and generous timeouts.

```rust
#[test]
#[ignore]
fn test_ignored_5_pipeline_chain_two_steps() {
    let t = Instant::now();
    step!(t, "spawning app");
    let mut app = spawn_app_with_timeout(300);

    step!(t, "selecting project");
    app.expect("Select Project").unwrap();
    app.send("\r").unwrap();
    app.expect("i=type").unwrap();
    std::thread::sleep(Duration::from_secs(1));

    step!(t, "opening task panel and switching to IssueDriven");
    app.send("X").unwrap();
    app.expect("p=pipeline").unwrap();
    app.send("p").unwrap();
    std::thread::sleep(Duration::from_millis(500));
    app.send("j").unwrap();
    std::thread::sleep(Duration::from_millis(200));
    app.send("\r").unwrap();
    app.expect("Esc=cancel").unwrap();
    app.send("\r").unwrap(); // submit empty goal
    app.expect("d=delete").unwrap();

    // --- Step 1: load-skills ---
    step!(t, "running load-skills");
    app.expect("load-skills").unwrap();
    app.send("\r").unwrap();
    // Close panel
    std::thread::sleep(Duration::from_millis(500));
    app.send("\x1b").unwrap();

    step!(t, "waiting for load-skills to complete (up to 120s)");
    app.set_expect_timeout(Some(Duration::from_secs(120)));
    app.expect("cc-companion:").expect("load-skills got no response");

    // Re-open panel to verify run-tests was enqueued
    step!(t, "verifying run-tests enqueued");
    std::thread::sleep(Duration::from_secs(1));
    app.send("X").unwrap();
    app.expect("run-tests").expect("run-tests not enqueued after load-skills");

    // --- Step 2: run-tests ---
    step!(t, "running run-tests");
    app.send("j").unwrap();
    std::thread::sleep(Duration::from_millis(200));
    app.send("\r").unwrap();
    std::thread::sleep(Duration::from_millis(500));
    app.send("\x1b").unwrap();

    step!(t, "waiting for run-tests to complete (up to 300s)");
    app.set_expect_timeout(Some(Duration::from_secs(300)));
    app.expect("cc-companion:").expect("run-tests got no response");

    // Verify implement-issue was enqueued
    step!(t, "verifying implement-issue enqueued");
    std::thread::sleep(Duration::from_secs(1));
    app.send("X").unwrap();
    app.expect("implement-issue").expect("implement-issue not enqueued after run-tests");

    step!(t, "quitting");
    app.send("\x1b").unwrap();
    std::thread::sleep(Duration::from_millis(300));
    app.send("q").unwrap();
    app.expect(Eof).unwrap();
    step!(t, "done");
}
```

**Step 2: Run the test (manually from terminal, not from Claude Code)**

Run: `cargo test --test e2e test_ignored_5 -- --ignored --test-threads=1 --nocapture`
Expected: PASS (takes 1-5 minutes depending on API speed)

**Step 3: Commit**

```bash
git add tests/e2e.rs
git commit -m "feat: add real-API multi-step pipeline E2E test"
```

---

### Task 6: Final Verification

**Step 1: Run all non-ignored E2E tests**

Run: `cargo test --test e2e -- --test-threads=1 2>&1 | tail -20`
Expected: All tests pass (including the new chain, cancel, and new-session tests)

**Step 2: Run all unit tests**

Run: `cargo test --lib 2>&1 | tail -10`
Expected: All unit tests still pass

**Step 3: Final commit (if any fixups needed)**

```bash
git add -A
git commit -m "fix: test adjustments from final verification"
```
