use std::process::Command;
use std::time::Duration;

use expectrl::session::OsSession;
use expectrl::{Eof, Expect, Session};

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

// ---------------------------------------------------------------------------
// True E2E tests that call Claude Code headless.
// These require a valid OAuth token and network access.
// Run with: cargo test --test e2e -- --ignored --test-threads=1
//
// Tests are numbered to control execution order (alphabetical).
// ---------------------------------------------------------------------------

/// Test 1: Select a project → verify Claude is called ("Thinking..." appears)
/// and a response renders ("cc-companion:" label). Proves the full round-trip:
/// spawn_claude → API call → JSON parse → message added → UI rendered.
#[test]
#[ignore]
fn test_ignored_1_claude_overview_responds() {
    let mut app = spawn_app_with_timeout(300);
    app.expect("Select Project").expect("app didn't show project select");

    // Press Enter to select the first project — triggers send_overview()
    app.send("\r").unwrap();

    // "Thinking..." proves spawn_claude() was called
    app.expect("Thinking...").expect("Claude was never invoked");

    // "cc-companion:" proves the response was parsed and rendered
    app.expect("cc-companion:").expect("Claude response never appeared");

    app.send("q").unwrap();
    app.expect(Eof).unwrap();
}

/// Test 2: After overview, type a message and verify a second Claude call
/// is triggered. We only check that "Thinking..." appears for our message
/// (proving spawn_claude was called), then cancel and quit.
#[test]
#[ignore]
fn test_ignored_2_chat_input_triggers_claude() {
    let mut app = spawn_app_with_timeout(300);
    app.expect("Select Project").unwrap();

    // Select first project — triggers overview
    app.send("\r").unwrap();

    // Wait for overview to complete (may take 60-180s)
    app.expect("Thinking...").unwrap();
    app.expect("cc-companion:").expect("overview never completed");

    // Enter input mode, type a message, send it
    app.send("i").unwrap();
    app.expect("Enter=send").expect("didn't enter input mode");
    app.send("Say hello\r").unwrap();

    // "Thinking..." proves a SECOND claude call was spawned
    app.expect("Thinking...").expect("second Claude call never triggered");

    // Cancel and quit
    app.send("\x1b").unwrap();
    app.expect("Cancelled").expect("cancel failed");
    app.send("q").unwrap();
    app.expect(Eof).unwrap();
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

    // Usage fetch runs in parallel on startup — should complete in seconds.
    // The status bar shows either "% used" (loaded) or "Loading usage" (pending/failed).
    // Use a short timeout: if API is rate-limited, "Loading usage" appears immediately.
    app.set_expect_timeout(Some(Duration::from_secs(15)));
    let loaded = app.expect("% used");
    if loaded.is_err() {
        // API may be rate-limited — verify loading state is shown instead
        app.expect("Loading usage").expect("neither usage nor loading state shown");
    }

    app.send("q").unwrap();
    app.expect(Eof).unwrap();
}
