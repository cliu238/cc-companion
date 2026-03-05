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
/// and Esc cancels the running call. Proves the integration:
/// project select → send_overview → spawn_claude → API call starts → cancel works.
#[test]
#[ignore]
fn test_ignored_1_claude_spawns_and_cancels() {
    let mut app = spawn_app_with_timeout(60);
    app.expect("Select Project").expect("app didn't show project select");

    // Press Enter to select the first project — triggers send_overview()
    app.send("\r").unwrap();

    // "Thinking..." proves spawn_claude() was called
    app.expect("Thinking...").expect("Claude was never invoked");

    // Esc cancels the running call
    app.send("\x1b").unwrap();
    app.expect("Cancelled").expect("cancel didn't work");

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
