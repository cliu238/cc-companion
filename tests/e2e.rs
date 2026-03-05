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
// Run with: cargo test --test e2e -- --ignored
// ---------------------------------------------------------------------------

/// Select a project via Enter, which triggers send_overview() calling
/// `claude -p "..." --output-format json`. Verify that "Thinking..." appears
/// (proving Claude was invoked) and then a response from cc-companion appears.
///
/// The overview prompt involves tool calls (Read, Glob) so Claude can take
/// 2-5 minutes. Run with: cargo test --test e2e -- --ignored --test-threads=1
#[test]
#[ignore]
fn test_claude_overview_flow() {
    let mut app = spawn_app_with_timeout(300);
    app.expect("Select Project").expect("app didn't show project select");

    // Press Enter to select the first project — triggers send_overview()
    app.send("\r").unwrap();

    // After selecting a project, app enters Chat mode and calls Claude.
    // "Thinking..." proves spawn_claude() was called.
    app.expect("Thinking...").expect("Claude was never invoked — no Thinking... indicator");

    // Wait for Claude to respond. The app renders "cc-companion:" as the
    // response label. This proves the full round-trip:
    // API call → JSON parse → message added → UI rendered.
    app.expect("cc-companion:").expect("Claude response never appeared in chat");

    app.send("q").unwrap();
    app.expect(Eof).unwrap();
}

/// Enter chat mode, type a message, send it, and verify Claude is invoked.
/// Proves the input flow: select project → wait for overview → type → send.
/// We verify "Thinking..." appears for the second call (proving spawn_claude
/// was called for our message), then cancel and quit.
#[test]
#[ignore]
fn test_claude_chat_send_triggers_call() {
    let mut app = spawn_app_with_timeout(300);
    app.expect("Select Project").unwrap();

    // Select first project — triggers overview
    app.send("\r").unwrap();
    app.expect("Chat").expect("didn't enter chat mode");

    // Wait for overview to complete
    app.expect("Thinking...").expect("overview Thinking... never shown");
    app.expect("cc-companion:").expect("overview response never arrived");

    // Enter input mode and type a message
    app.send("i").unwrap();
    app.expect("Enter=send").expect("didn't enter chat input mode");
    app.send("Say hello\r").unwrap();

    // "Thinking..." appearing proves a SECOND claude call was spawned.
    app.expect("Thinking...").expect("second Claude call was never triggered");

    // Cancel the running claude process and quit
    app.send("\x1b").unwrap();
    app.expect("Cancelled").expect("cancel didn't complete");
    app.send("q").unwrap();
    app.expect(Eof).expect("app didn't exit after cancel + quit");
}

/// Verify that the status bar shows usage information.
/// Usage is fetched in the background on App::new() — no Claude call needed.
/// Just wait for the usage API to respond (usually <5s).
#[test]
#[ignore]
fn test_usage_status_appears() {
    let mut app = spawn_app_with_timeout(30);
    app.expect("Select Project").unwrap();

    // Usage is fetched on startup. Wait for "% used" to appear in the
    // status bar. We don't need to enter chat — just need the app to
    // render after the usage fetch completes.
    // But status bar only shows in Chat mode, so enter chat first.
    app.send("\r").unwrap();
    app.expect("Chat").unwrap();

    // The usage fetch runs in parallel. It should complete within seconds.
    // The status bar should show "% used" once loaded.
    app.expect("% used").expect("usage status never appeared in status bar");

    app.send("q").unwrap();
    app.expect(Eof).unwrap();
}
