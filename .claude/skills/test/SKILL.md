---
name: test
description: Use when writing E2E or integration tests for terminal UI apps built with ratatui/crossterm in Rust, or when spawning CLI subprocesses like claude from tests. Triggers on PTY testing, expectrl, Stdio::piped bugs, empty stdout from subprocesses, or CLAUDECODE nesting errors.
---

# E2E Testing TUI Apps

## Overview

Test terminal UI apps at three levels: unit (TestBackend buffer), PTY interaction (expectrl), and real subprocess integration. Each level has specific pitfalls.

## Quick Reference

| Level | Tool | Speed | What it tests |
|-------|------|-------|---------------|
| Unit render | `ratatui::TestBackend` | <1ms | Widget rendering, conditional branches |
| PTY interaction | `expectrl` + `Session::spawn(Command)` | <1s | Keystroke → state → screen output |
| Real subprocess | `expectrl` + `#[ignore]` | 30-300s | Full round-trip including API calls |

## Critical Pitfalls

### 1. Stdio::piped is REQUIRED for subprocess output capture

`Command::new("prog").spawn()` inherits parent stdio by default. `child.wait_with_output()` returns **empty buffers** unless you pipe:

```rust
// BUG: stdout/stderr are empty
let child = Command::new("claude").spawn()?;
let output = child.wait_with_output()?;
// output.stdout is EMPTY — went to parent terminal

// FIX: pipe both streams
let mut cmd = Command::new("claude");
cmd.stdout(Stdio::piped());
cmd.stderr(Stdio::piped());
let child = cmd.spawn()?;
let output = child.wait_with_output()?;
// output.stdout and output.stderr now captured
```

**Symptom:** `"JSON parse error: EOF while parsing a value at line 1 column 0"` — means you're parsing empty string because stdout was never captured.

### 2. CLI tools may write to stderr, not stdout

`claude --output-format json` writes JSON to stderr. Always check both:

```rust
let stdout = String::from_utf8_lossy(&output.stdout);
let raw = if stdout.trim_ascii().is_empty() {
    String::from_utf8_lossy(&output.stderr)
} else {
    stdout
};
```

### 3. CLAUDECODE env var blocks nested sessions

Running `claude` CLI from within a Claude Code session fails with "Nested sessions" error. Fix in tests:

```rust
let mut cmd = Command::new(bin_path);
cmd.env_remove("CLAUDECODE");  // Allow nested claude calls
let mut session = Session::spawn(cmd).unwrap();
```

### 4. expectrl works with full-screen TUI apps

Despite ANSI escape codes, `expect("text")` finds plain text within ratatui's output stream. Each 250ms redraw writes the full screen, so text appears repeatedly.

### 5. Subprocess tests need long timeouts

Claude API calls with tool use (Read, Glob) take 60-300s. Always use `#[ignore]` and generous timeouts:

```rust
#[test]
#[ignore]  // Run with: cargo test -- --ignored --test-threads=1
fn test_claude_responds() {
    let mut app = spawn_app_with_timeout(300);
    // ...
}
```

Run serial (`--test-threads=1`) to avoid rate limiting from parallel API calls.

### 6. Cannot test claude CLI from within Claude Code Bash tool

Calling `claude` from the Bash tool inside a Claude Code session hangs or produces empty output. Write `#[ignore]` tests and have the user run them from a normal terminal.

## Unit Test Pattern (TestBackend)

```rust
let mut app = App::test_default();
app.mode = Mode::Chat;
app.chat.waiting = true;  // Set the state you want to test

let backend = TestBackend::new(80, 24);
let mut terminal = Terminal::new(backend).unwrap();
terminal.draw(|f| draw(f, &mut app)).unwrap();

let buf = terminal.backend().buffer();
assert!(buffer_contains(buf, "Thinking..."));
```

**Key:** Create a side-effect-free `test_default()` constructor (no I/O, no threads).

## PTY Test Pattern (expectrl)

```rust
use expectrl::{Eof, Expect, Session};
use std::process::Command;

fn spawn_app_with_timeout(secs: u64) -> OsSession {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_my_app"));
    cmd.env_remove("CLAUDECODE");
    let mut session = Session::spawn(cmd).unwrap();
    session.set_expect_timeout(Some(Duration::from_secs(secs)));
    session
}

#[test]
fn test_keystroke_flow() {
    let mut app = spawn_app_with_timeout(5);
    app.expect("Title").unwrap();     // Wait for render
    app.send("/").unwrap();            // Send keystroke
    app.expect("Search:").unwrap();    // Verify state change
    app.send("\x1b").unwrap();         // Send Esc
    app.send("q").unwrap();            // Quit
    app.expect(Eof).unwrap();          // Verify clean exit
}
```

### 7. Ratatui differential rendering breaks expect() matching

Ratatui only redraws cells that changed between frames. If the new text occupies the same terminal region as old text, `expect()` may never see it in the PTY stream — the bytes are simply not emitted.

**Example:** Help bar changes from `"i=type x=task..."` to `"Enter=send Alt+Enter=newline..."`. If both strings render at the same screen position, ratatui may emit only the **changed characters**, not the full new string.

**Fix:** Match on a substring that is **unique to the new state AND not a substring of the old state**:

```rust
// BAD: "Enter=send" might not appear in PTY stream due to diff rendering
app.expect("Enter=send").unwrap();

// GOOD: "Alt+Enter" is unique to ChatInput help bar and wasn't in old text
app.expect("Alt+Enter").unwrap();
```

**General rule:** After a mode change, `expect()` a string that:
1. Only appears in the new mode's rendering
2. Was NOT present (even partially) in the previous frame
3. Occupies screen cells that previously had different content

### 8. Add sleep between mode transitions

After `expect()` confirms a screen state, the app may still be processing. Add a short delay before sending the next keystroke:

```rust
app.expect("i=type").unwrap();           // Chat mode rendered
std::thread::sleep(Duration::from_secs(1)); // Let app settle
app.send("i").unwrap();                  // Now enter input mode
```

### 9. Use step!() timing macro for debugging slow tests

Add timestamps to identify which step is slow or stuck:

```rust
macro_rules! step {
    ($start:expr, $msg:expr) => {
        eprintln!("[{:>6.1}s] {}", $start.elapsed().as_secs_f64(), $msg);
    };
}

// Run with: cargo test -- --ignored --test-threads=1 --nocapture
```

## Common Mistakes

| Mistake | Fix |
|---------|-----|
| Forgetting `Stdio::piped()` | Always pipe when using `wait_with_output()` |
| Checking only stdout | Check stderr too — many CLIs write there |
| Short timeouts for API calls | 300s for tool-using prompts |
| Running ignored tests in CI | Use `#[ignore]` + `--ignored` for API tests |
| Parallel API test runs | Use `--test-threads=1` to avoid rate limits |
| Testing claude from Claude Code Bash | Write ignored tests, run from normal terminal |
| `expect()` for text in same screen region | Use unique substring not in previous frame |
| Sending keys immediately after `expect()` | Add `sleep()` between mode transitions |

## Mock CLI Testing

For testing CLI subprocess behavior without real API calls, use a mock script that mimics the CLI's output format.

### Pattern: mock script + PATH override

1. Create `tests/fixtures/mock_claude` (executable shell script):
   - Reads args, writes them to `$MOCK_CLAUDE_ARGS_FILE` for later assertion
   - Returns valid JSON in the expected output format

2. Create a symlink `tests/fixtures/claude -> mock_claude` so PATH override works.

3. In PTY test, override PATH so `claude` resolves to the mock:

```rust
fn spawn_app_with_mock_claude(timeout_secs: u64) -> (OsSession, PathBuf) {
    let mock_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let args_file = std::env::temp_dir().join(format!("mock_args_{}", std::process::id()));
    let path = format!("{}:{}", mock_dir.display(), std::env::var("PATH").unwrap_or_default());

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_my_app"));
    cmd.env("PATH", &path);
    cmd.env("MOCK_CLAUDE_ARGS_FILE", &args_file);
    cmd.env_remove("CLAUDECODE");

    let mut session = Session::spawn(cmd).unwrap();
    session.set_expect_timeout(Some(Duration::from_secs(timeout_secs)));
    (session, args_file)
}
```

4. After the interaction, read `args_file` to verify which CLI flags were passed:

```rust
let args = std::fs::read_to_string(&args_file).unwrap_or_default();
assert!(args.contains("--system-prompt"), "expected --system-prompt in args");
```

### Key points
- Mock script must output the exact JSON format the app expects (`[{"type":"init",...},{"type":"result",...}]`)
- Clean up `args_file` at end of test
- Use unique filename (PID or atomic counter) to avoid collisions in parallel test runs
- Create symlink `tests/fixtures/claude -> mock_claude` since the app invokes `claude` by name
