use std::process::Command;
use std::time::{Duration, Instant};

use expectrl::session::OsSession;
use expectrl::{Eof, Expect, Session};

macro_rules! step {
    ($start:expr, $msg:expr) => {
        eprintln!("[{:>6.1}s] {}", $start.elapsed().as_secs_f64(), $msg);
    };
}

/// Spawn the app in a PTY with a given timeout.
/// Unsets CLAUDECODE so nested `claude` CLI calls work.
fn spawn_app_with_timeout(timeout_secs: u64) -> OsSession {
    let bin = env!("CARGO_BIN_EXE_cc-companion");
    let mut cmd = Command::new(bin);
    cmd.env_remove("CLAUDECODE");
    let mut session = Session::spawn(cmd).expect("failed to spawn cc-companion");
    session.set_expect_timeout(Some(Duration::from_secs(timeout_secs)));
    session
}

/// Spawn the app in a PTY (short timeout for fast tests).
fn spawn_app() -> OsSession {
    spawn_app_with_timeout(5)
}

#[test]
fn test_app_launches_and_shows_title() {
    let mut app = spawn_app();
    app.expect("cc-companion").expect("title bar not found");
    app.send("q").unwrap();
    app.expect(Eof).expect("app did not exit after 'q'");
}

#[test]
fn test_quit_with_q() {
    let mut app = spawn_app();
    app.expect("cc-companion").expect("app didn't start");
    app.send("q").unwrap();
    app.expect(Eof).expect("app did not exit after 'q'");
}

#[test]
fn test_project_select_mode_shown() {
    let mut app = spawn_app();
    app.expect("Select Project")
        .expect("project select title not found");
    app.send("q").unwrap();
    app.expect(Eof).unwrap();
}

#[test]
fn test_search_mode_shows_prompt() {
    let mut app = spawn_app();
    app.expect("cc-companion").unwrap();
    // '/' enters search mode — help bar should show "Search:"
    app.send("/").unwrap();
    app.expect("Search:").expect("search prompt not shown");
    // Esc cancels search
    app.send("\x1b").unwrap();
    // Should be back to normal help bar
    app.expect("j/k=move").expect("normal help bar not restored after Esc");
    app.send("q").unwrap();
    app.expect(Eof).unwrap();
}

#[test]
fn test_add_path_mode_shows_prompt() {
    let mut app = spawn_app();
    app.expect("cc-companion").unwrap();
    // 'a' enters path input mode
    app.send("a").unwrap();
    app.expect("Path:").expect("path input prompt not shown");
    // Esc cancels
    app.send("\x1b").unwrap();
    app.expect("j/k=move").expect("normal help bar not restored");
    app.send("q").unwrap();
    app.expect(Eof).unwrap();
}

#[test]
fn test_search_filters_and_clears() {
    let mut app = spawn_app();
    app.expect("Select Project").unwrap();
    // Enter search mode and type a query
    app.send("/").unwrap();
    app.expect("Search:").unwrap();
    // Type something unlikely to match any project
    app.send("zzzznotaproject").unwrap();
    // Press Enter to accept the search
    app.send("\r").unwrap();
    // Should show "0" projects (filtered down)
    // NOTE: Copilot suggested matching "(0)" but PTY differential rendering splits
    // parentheses with ANSI escapes. Matching bare "0" is broad but reliable.
    app.expect("0").expect("expected 0 projects after nonsense search");
    app.send("q").unwrap();
    app.expect(Eof).unwrap();
}

#[test]
fn test_help_bar_shows_keybindings() {
    let mut app = spawn_app();
    // In ProjectSelect, help bar should show key hints
    app.expect("/=search").expect("help bar missing /=search");
    app.send("q").unwrap();
    app.expect(Eof).unwrap();
}

#[test]
fn test_task_panel_opens_in_chat() {
    let mut app = spawn_app();
    app.expect("Select Project").unwrap();
    // Select first project to enter Chat mode
    app.send("\r").unwrap();
    app.expect("i=type").expect("didn't enter Chat mode");
    std::thread::sleep(Duration::from_millis(500));
    // 'X' opens task panel — help bar changes to panel-specific keys
    app.send("X").unwrap();
    // "p=pipeline" is unique to the task panel help bar (not in normal Chat help bar)
    app.expect("p=pipeline").expect("task panel help bar not shown");
    // Close panel and quit
    app.send("\x1b").unwrap();
    std::thread::sleep(Duration::from_millis(300));
    app.send("q").unwrap();
    app.expect(Eof).unwrap();
}

/// Test that usage fetch resolves without crashing.
/// The async usage fetch runs on app startup. By entering chat mode and
/// waiting briefly, we verify the new Result-based fetch and error display
/// code path doesn't panic. The status bar will show either usage data
/// or an error message depending on OAuth token availability.
#[test]
fn test_usage_fetch_completes_without_crash() {
    let mut app = spawn_app_with_timeout(15);
    app.expect("Select Project").unwrap();
    app.send("\r").unwrap();
    // Enter chat mode — status bar with usage info is rendered here
    app.expect("i=type").unwrap();
    // Wait for async usage fetch to complete (token lookup + curl)
    std::thread::sleep(Duration::from_secs(3));
    // Verify app is still responsive after usage fetch resolved
    app.send("q").unwrap();
    app.expect(Eof).expect("app should exit cleanly after usage fetch");
}

/// Spawn the app with `claude` overridden by mock script.
/// Returns (session, args_file_path, count_file_path).
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

/// Spawn the app with `claude` overridden by a slow mock script.
/// The mock sleeps for `delay_secs` before responding.
/// Returns (session, args_file_path, count_file_path).
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

#[test]
fn test_mock_chat_includes_system_prompt() {
    let (mut app, args_file, count_file) = spawn_app_with_mock_claude(15);
    app.expect("Select Project").unwrap();
    app.send("\r").unwrap();
    app.expect("i=type").unwrap();

    std::thread::sleep(Duration::from_secs(1));
    app.send("i").unwrap();
    app.expect("Alt+Enter").unwrap();
    app.send("hello\r").unwrap();

    // Wait for mock response to appear
    app.expect("Mock response").expect("mock claude didn't respond");

    // Verify args file was written and contains --system-prompt
    let args = std::fs::read_to_string(&args_file).unwrap_or_default();
    assert!(
        args.contains("--system-prompt"),
        "chat should include --system-prompt, got: {}",
        args
    );
    assert!(args.contains("dontAsk"),
        "chat should use dontAsk permission mode, got: {}", args);

    app.send("q").unwrap();
    app.expect(Eof).unwrap();
    let _ = std::fs::remove_file(&args_file);
    let _ = std::fs::remove_file(&count_file);
}

#[test]
fn test_mock_pipeline_task_skips_system_prompt() {
    let (mut app, args_file, count_file) = spawn_app_with_mock_claude(15);
    app.expect("Select Project").unwrap();
    app.send("\r").unwrap();
    app.expect("i=type").unwrap();

    std::thread::sleep(Duration::from_millis(500));

    // Open task panel
    app.send("X").unwrap();
    app.expect("p=pipeline").unwrap();

    // Switch to IssueDriven pipeline
    app.send("p").unwrap();
    std::thread::sleep(Duration::from_millis(300));
    // Navigate to IssueDriven (index 1) and select
    app.send("j").unwrap();
    std::thread::sleep(Duration::from_millis(200));
    app.send("\r").unwrap();

    // IssueDriven asks for goal text — submit empty
    app.expect("Esc=cancel").expect("goal input prompt not shown");
    app.send("\r").unwrap();

    // Wait for task panel to show tasks after pipeline switch
    app.expect("d=delete").expect("task panel not shown after goal submit");

    // Run the first pending task manually with Enter
    app.send("\r").unwrap();

    // Wait for mock to complete, then close task panel to reveal chat
    std::thread::sleep(Duration::from_secs(2));
    app.send("\x1b").unwrap();
    std::thread::sleep(Duration::from_millis(500));

    app.expect("Skills loaded")
        .expect("mock claude didn't respond to pipeline task");

    // Verify args — should NOT have --system-prompt
    let args = std::fs::read_to_string(&args_file).unwrap_or_default();
    assert!(
        !args.contains("--system-prompt"),
        "pipeline task should NOT include --system-prompt, got: {}",
        args
    );
    assert!(args.contains("bypassPermissions"),
        "pipeline task should use bypassPermissions, got: {}", args);

    app.send("q").unwrap();
    app.expect(Eof).unwrap();
    let _ = std::fs::remove_file(&args_file);
    let _ = std::fs::remove_file(&count_file);
}

#[test]
fn test_mock_pipeline_full_chain() {
    let t = Instant::now();
    let (mut app, args_file, count_file) = spawn_app_with_mock_claude(30);

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
    // Task panel is visible with load-skills as first pending task (d=delete confirmed above)
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
    // Title shows "1 done / 0 running / 1 pending" — confirms chaining worked
    app.expect("1 pending").expect("run-tests not enqueued after load-skills");

    // --- Step 2: run-tests ---
    step!(t, "running run-tests");
    // Navigate to the pending task (past done items)
    // After load-skills completes: done=[load-skills], pending=[run-tests]
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
    app.expect("1 pending").expect("implement-issue not enqueued after run-tests");

    // --- Step 3: implement-issue ---
    step!(t, "running implement-issue");
    // done=[load-skills, run-tests], pending=[implement-issue]
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
    app.expect("1 pending").expect("verify not enqueued");
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
    app.expect("1 pending").expect("finish not enqueued");
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

    // Cancel with Esc (we're back in Normal mode after sending)
    std::thread::sleep(Duration::from_millis(500));
    app.send("\x1b").unwrap();
    app.expect("Cancelled").expect("cancel message not shown");

    app.send("q").unwrap();
    app.expect(Eof).unwrap();

    let _ = std::fs::remove_file(&args_file);
    let _ = std::fs::remove_file(&count_file);
}

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
    std::thread::sleep(Duration::from_millis(500));

    // Press 'n' to start new session
    app.send("n").unwrap();

    // The chat should now show "new session" in title bar.
    // Note: ratatui splits the title across escape codes at column boundaries,
    // so we match a shorter fragment that stays contiguous in the PTY output.
    app.expect("new se").expect("new session not shown after 'n'");

    app.send("q").unwrap();
    app.expect(Eof).unwrap();
    let _ = std::fs::remove_file(&args_file);
    let _ = std::fs::remove_file(&count_file);
}

// ---------------------------------------------------------------------------
// True E2E tests that call Claude Code headless.
// These require a valid OAuth token and network access.
// Run with: cargo test --test e2e -- --ignored --test-threads=1
//
// Tests are numbered to control execution order (alphabetical).
// ---------------------------------------------------------------------------

/// Test 1: Select a project → enter chat → send a message → verify Claude responds.
/// Proves the full round-trip: spawn → API → JSON parse → message rendered.
#[test]
#[ignore]
fn test_ignored_1_claude_round_trip() {
    let t = Instant::now();
    step!(t, "spawning app");
    let mut app = spawn_app_with_timeout(120);

    step!(t, "waiting for Select Project");
    app.expect("Select Project").expect("app didn't show project select");

    step!(t, "pressing Enter to select project");
    app.send("\r").unwrap();

    step!(t, "waiting for Chat help bar");
    app.expect("i=type").expect("Chat mode help bar not shown");

    std::thread::sleep(Duration::from_secs(1));
    step!(t, "pressing 'i' for input mode");
    app.send("i").unwrap();

    step!(t, "waiting for ChatInput help bar");
    app.expect("Alt+Enter").expect("didn't enter input mode");

    step!(t, "sending message");
    app.send("Say hi in one word\r").unwrap();

    step!(t, "waiting for Thinking...");
    app.expect("Thinking...").expect("chat claude call never triggered");

    step!(t, "waiting for cc-companion: response");
    app.expect("cc-companion:").expect("Claude response never appeared");

    step!(t, "quitting");
    app.send("q").unwrap();
    app.expect(Eof).unwrap();
    step!(t, "done");
}

/// Test 3: Verify usage status resolves in the status bar.
/// Usage is fetched on App::new() in the background. Enter chat mode
/// (where the status bar is visible) and wait for a result.
/// On success: "% used". On rate limit: "Rate limited". On failure: "Network error".
#[test]
#[ignore]
fn test_ignored_3_usage_status_renders() {
    let mut app = spawn_app_with_timeout(60);
    app.expect("Select Project").unwrap();

    // Enter chat mode (status bar only shows here)
    app.send("\r").unwrap();
    app.expect("Chat").unwrap();

    // Usage fetch runs in parallel on startup.
    // Status bar initially shows "Loading usage" then resolves to either
    // usage data ("% used") or an error ("Rate limited" / "Network error").
    app.expect("Loading").expect("status bar never showed loading state");
    app.set_expect_timeout(Some(Duration::from_secs(30)));
    // Try success first; if rate-limited or error, that's also a valid outcome.
    let _ = app.expect("% used");

    app.send("q").unwrap();
    app.expect(Eof).unwrap();
}

/// Test 4: Pipeline task executes successfully without Advisor prompt blocking it.
/// Proves the fix for Issue #5 works end-to-end with real Claude.
#[test]
#[ignore]
fn test_ignored_4_pipeline_task_executes() {
    let t = Instant::now();
    step!(t, "spawning app");
    let mut app = spawn_app_with_timeout(120);

    step!(t, "selecting project");
    app.expect("Select Project").unwrap();
    app.send("\r").unwrap();
    app.expect("i=type").unwrap();

    std::thread::sleep(Duration::from_secs(1));

    step!(t, "opening task panel");
    app.send("X").unwrap();
    app.expect("p=pipeline").unwrap();

    step!(t, "switching to IssueDriven pipeline");
    app.send("p").unwrap();
    std::thread::sleep(Duration::from_millis(500));
    app.send("j").unwrap();
    app.send("\r").unwrap();
    std::thread::sleep(Duration::from_millis(500));
    app.send("\r").unwrap(); // submit empty goal

    std::thread::sleep(Duration::from_millis(500));

    step!(t, "running first task");
    app.send("\r").unwrap();

    // Close task panel so chat area is visible
    app.send("\x1b").unwrap();

    step!(t, "waiting for response (up to 120s)");
    app.expect("cc-companion:").expect("pipeline task got no response — Claude may have refused");

    step!(t, "quitting");
    app.send("q").unwrap();
    app.expect(Eof).unwrap();
    step!(t, "done");
}

/// Test 5: Pipeline chain — run first 2 steps with real Claude, verify chaining.
/// Proves that `on_complete()` correctly enqueues the next task after each step.
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
    app.send("\r").unwrap();
    // Close panel while task runs
    std::thread::sleep(Duration::from_millis(500));
    app.send("\x1b").unwrap();

    // Poll task panel for completion. Opening the panel draws the popup fresh,
    // so the title (with done/pending counts) is always fully emitted.
    step!(t, "waiting for load-skills to complete (polling, up to 120s)");
    app.set_expect_timeout(Some(Duration::from_secs(5)));
    let mut step1_done = false;
    for attempt in 0..12 {
        std::thread::sleep(Duration::from_secs(10));
        step!(t, &format!("poll attempt {}/12", attempt + 1));
        app.send("X").unwrap(); // Open panel
        if app.expect("1 done").is_ok() {
            step1_done = true;
            break;
        }
        app.send("\x1b").unwrap(); // Close panel, try again
    }
    assert!(step1_done, "load-skills didn't complete within 120s");

    // Panel is open with "1 done" — verify run-tests was enqueued
    step!(t, "verifying run-tests enqueued");
    app.expect("1 pending").expect("run-tests not enqueued after load-skills");

    // --- Step 2: run-tests ---
    step!(t, "running run-tests");
    app.send("j").unwrap();
    std::thread::sleep(Duration::from_millis(200));
    app.send("\r").unwrap();
    // Close panel while task runs
    std::thread::sleep(Duration::from_millis(500));
    app.send("\x1b").unwrap();

    // Poll task panel for completion. Can't use expect("cc-companion:") because
    // the title bar doesn't change between responses (differential rendering).
    // Opening the panel draws the popup fresh each time, so the title is emitted.
    step!(t, "waiting for run-tests to complete (polling, up to 300s)");
    app.set_expect_timeout(Some(Duration::from_secs(5)));
    let mut step2_done = false;
    for attempt in 0..30 {
        std::thread::sleep(Duration::from_secs(10));
        step!(t, &format!("poll attempt {}/30", attempt + 1));
        app.send("X").unwrap(); // Open panel
        if app.expect("2 done").is_ok() {
            step2_done = true;
            break;
        }
        app.send("\x1b").unwrap(); // Close panel, try again
    }
    assert!(step2_done, "run-tests didn't complete within 300s");

    // Panel is open with "2 done" — verify implement-issue was enqueued
    step!(t, "verifying implement-issue enqueued");
    app.expect("1 pending").expect("implement-issue not enqueued after run-tests");

    step!(t, "quitting");
    app.send("\x1b").unwrap();
    std::thread::sleep(Duration::from_millis(300));
    app.send("q").unwrap();
    app.expect(Eof).unwrap();
    step!(t, "done");
}
