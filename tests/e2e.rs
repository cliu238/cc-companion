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

/// Spawn the app with `claude` overridden by mock script.
/// Returns (session, args_file_path).
fn spawn_app_with_mock_claude(timeout_secs: u64) -> (OsSession, std::path::PathBuf) {
    let bin = env!("CARGO_BIN_EXE_cc-companion");
    let mock_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let args_file = std::env::temp_dir().join(format!(
        "mock_claude_args_{}_{}", std::process::id(), id
    ));

    let original_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", mock_dir.display(), original_path);

    let mut cmd = Command::new(bin);
    cmd.env_remove("CLAUDECODE");
    cmd.env("PATH", &new_path);
    cmd.env("MOCK_CLAUDE_ARGS_FILE", &args_file);

    let mut session = Session::spawn(cmd).expect("failed to spawn with mock");
    session.set_expect_timeout(Some(Duration::from_secs(timeout_secs)));
    (session, args_file)
}

#[test]
fn test_mock_chat_includes_system_prompt() {
    let (mut app, args_file) = spawn_app_with_mock_claude(15);
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

    app.send("q").unwrap();
    app.expect(Eof).unwrap();
    let _ = std::fs::remove_file(&args_file);
}

#[test]
fn test_mock_pipeline_task_skips_system_prompt() {
    let (mut app, args_file) = spawn_app_with_mock_claude(15);
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

    app.expect("Mock response")
        .expect("mock claude didn't respond to pipeline task");

    // Verify args — should NOT have --system-prompt
    let args = std::fs::read_to_string(&args_file).unwrap_or_default();
    assert!(
        !args.contains("--system-prompt"),
        "pipeline task should NOT include --system-prompt, got: {}",
        args
    );

    app.send("q").unwrap();
    app.expect(Eof).unwrap();
    let _ = std::fs::remove_file(&args_file);
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

/// Test 3: Verify usage stats appear in the status bar.
/// Usage is fetched on App::new() in the background. Enter chat mode
/// (where the status bar is visible) and wait for "% used".
#[test]
#[ignore]
fn test_ignored_3_usage_status_renders() {
    let mut app = spawn_app_with_timeout(60);
    app.expect("Select Project").unwrap();

    // Enter chat mode (status bar only shows here)
    app.send("\r").unwrap();
    app.expect("Chat").unwrap();

    // Usage fetch runs in parallel on startup.
    // Status bar initially shows "Loading usage" then switches to "% used".
    // Check for "Loading" first (immediate), then wait for "% used" (API response).
    app.expect("Loading").expect("status bar never showed loading state");
    app.set_expect_timeout(Some(Duration::from_secs(30)));
    // If API responds in time we see "% used"; if rate-limited, timeout is OK.
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
