# Fix Advisor Prompt in Pipeline Tasks — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Pipeline execution tasks should not receive the Advisor system prompt, so Claude stops refusing to execute.

**Architecture:** Add `use_advisor` field to `AutoTask`, introduce `SpawnConfig` struct to replace `spawn_claude`'s parameter list, extract `build_claude_cmd` as a testable pure function that conditionally includes `--system-prompt`.

**Tech Stack:** Rust, ratatui, crossterm, expectrl (testing), shell scripts (mock claude)

---

### Task 1: Add `use_advisor` field to `AutoTask`

**Files:**
- Modify: `src/pipeline/mod.rs:13-20`

**Step 1: Write the failing test**

Add to `src/pipeline/mod.rs` inside the existing `#[cfg(test)] mod tests`:

```rust
#[test]
fn test_issue_driven_tasks_skip_advisor() {
    let tasks = Pipeline::IssueDriven.initial_tasks("/tmp/proj", "");
    for task in &tasks {
        assert!(!task.use_advisor, "IssueDriven task '{}' should not use advisor", task.name);
    }
}

#[test]
fn test_example_tasks_use_advisor() {
    let tasks = Pipeline::Example.initial_tasks("/tmp/proj", "");
    for task in &tasks {
        assert!(task.use_advisor, "Example task '{}' should use advisor", task.name);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib test_issue_driven_tasks_skip_advisor test_example_tasks_use_advisor 2>&1 | head -30`
Expected: FAIL — `AutoTask` has no field `use_advisor`

**Step 3: Add `use_advisor` field to `AutoTask` and update all construction sites**

In `src/pipeline/mod.rs:13-20`, add the field:

```rust
#[derive(Clone)]
pub struct AutoTask {
    pub name: String,
    pub prompt: String,
    pub cwd: String,
    pub read_only: bool,
    pub resume: bool,
    pub setup: Option<String>,
    pub use_advisor: bool,
}
```

In `src/pipeline/example.rs`, add `use_advisor: true` to each of the 4 `AutoTask` literals (lines 6, 14, 22, 30). Example for the first one:

```rust
AutoTask {
    name: "code review".into(),
    prompt: "Review this project's codebase. Focus on bugs, error handling gaps, and logic issues. Be concise.".into(),
    cwd: cwd.clone(),
    read_only: true,
    resume: true,
    setup: None,
    use_advisor: true,
},
```

In `src/pipeline/issue_driven.rs`, add `use_advisor: false` to every `AutoTask` literal. There are 5 construction sites:
- `initial_tasks` function (line 10-18): 1 task
- `on_complete` function: `RUN_TESTS` (line 28-37), `IMPLEMENT` (line 53-73), `VERIFY` (line 76-89), `FINISH` (line 91-98)

Example for the first one in `initial_tasks`:

```rust
vec![AutoTask {
    name: LOAD_SKILLS.into(),
    prompt: "Load `/domain-knowledge` skills ...".into(),
    cwd: project_cwd.to_string(),
    read_only: false,
    resume: false,
    setup: None,
    use_advisor: false,
}]
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib test_issue_driven_tasks_skip_advisor test_example_tasks_use_advisor 2>&1 | tail -5`
Expected: both PASS

**Step 5: Run full test suite to check nothing broke**

Run: `cargo test --lib 2>&1 | tail -10`
Expected: all existing tests pass

**Step 6: Commit**

```bash
git add src/pipeline/mod.rs src/pipeline/example.rs src/pipeline/issue_driven.rs
git commit -m "Add use_advisor field to AutoTask (#5)"
```

---

### Task 2: Introduce `SpawnConfig` and `build_claude_cmd`

**Files:**
- Modify: `src/app/chat.rs:1-314`

**Step 1: Write the failing tests**

Add at bottom of `src/app/chat.rs`, inside a new `#[cfg(test)] mod tests` block:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> SpawnConfig {
        SpawnConfig {
            msg: "hello".into(),
            resume_session: None,
            read_only: false,
            cwd: None,
            model: None,
            setup: None,
            system_prompt: None,
            gateway: None,
        }
    }

    #[test]
    fn test_build_cmd_no_system_prompt() {
        let config = base_config();
        let cmd = build_claude_cmd(&config);
        let args: Vec<_> = cmd.get_args().collect();
        assert!(!args.contains(&"--system-prompt".as_ref()),
            "should not have --system-prompt when system_prompt is None");
        assert!(args.contains(&"-p".as_ref()));
        assert!(args.contains(&"hello".as_ref()));
    }

    #[test]
    fn test_build_cmd_with_system_prompt() {
        let mut config = base_config();
        config.system_prompt = Some("You are an advisor.".into());
        let cmd = build_claude_cmd(&config);
        let args: Vec<_> = cmd.get_args().collect();
        assert!(args.contains(&"--system-prompt".as_ref()));
        assert!(args.contains(&"You are an advisor.".as_ref()));
    }

    #[test]
    fn test_build_cmd_read_only() {
        let mut config = base_config();
        config.read_only = true;
        let cmd = build_claude_cmd(&config);
        let args: Vec<_> = cmd.get_args().collect();
        assert!(args.contains(&"--disallowedTools".as_ref()));
    }

    #[test]
    fn test_build_cmd_not_read_only() {
        let config = base_config();
        let cmd = build_claude_cmd(&config);
        let args: Vec<_> = cmd.get_args().collect();
        assert!(!args.contains(&"--disallowedTools".as_ref()));
    }

    #[test]
    fn test_build_cmd_with_resume() {
        let mut config = base_config();
        config.resume_session = Some("sess-123".into());
        let cmd = build_claude_cmd(&config);
        let args: Vec<_> = cmd.get_args().collect();
        assert!(args.contains(&"--resume".as_ref()));
        assert!(args.contains(&"sess-123".as_ref()));
    }

    #[test]
    fn test_build_cmd_with_model() {
        let mut config = base_config();
        config.model = Some("claude-sonnet-4-6".into());
        let cmd = build_claude_cmd(&config);
        let args: Vec<_> = cmd.get_args().collect();
        assert!(args.contains(&"--model".as_ref()));
        assert!(args.contains(&"claude-sonnet-4-6".as_ref()));
    }

    #[test]
    fn test_build_cmd_with_cwd() {
        let mut config = base_config();
        config.cwd = Some("/tmp/proj".into());
        let cmd = build_claude_cmd(&config);
        let args: Vec<_> = cmd.get_args().collect();
        assert!(args.contains(&"--add-dir".as_ref()));
        assert!(args.contains(&"/tmp/proj".as_ref()));
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib chat::tests 2>&1 | head -20`
Expected: FAIL — `SpawnConfig` and `build_claude_cmd` don't exist yet

**Step 3: Implement `SpawnConfig`, `GatewayConfig`, and `build_claude_cmd`**

Add these structs and function at the top of `src/app/chat.rs` (after the existing imports):

```rust
pub(crate) struct GatewayConfig {
    pub url: String,
    pub headers: Option<String>,
}

pub(crate) struct SpawnConfig {
    pub msg: String,
    pub resume_session: Option<String>,
    pub read_only: bool,
    pub cwd: Option<String>,
    pub model: Option<String>,
    pub setup: Option<String>,
    pub system_prompt: Option<String>,
    pub gateway: Option<GatewayConfig>,
}

pub(crate) fn build_claude_cmd(config: &SpawnConfig) -> Command {
    let mut cmd = Command::new("claude");
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    if let Some(dir) = &config.cwd {
        if !dir.is_empty() {
            cmd.current_dir(dir);
            cmd.arg("--add-dir").arg(dir);
        }
    }
    if let Some(gw) = &config.gateway {
        cmd.env("ANTHROPIC_BASE_URL", &gw.url);
        if let Some(h) = &gw.headers {
            cmd.env("ANTHROPIC_CUSTOM_HEADERS", h);
        }
    } else {
        cmd.env_remove("ANTHROPIC_BASE_URL");
        cmd.env_remove("ANTHROPIC_CUSTOM_HEADERS");
    }
    if let Some(m) = &config.model {
        cmd.arg("--model").arg(m);
    }
    if let Some(sp) = &config.system_prompt {
        cmd.arg("--system-prompt").arg(sp);
    }
    cmd.arg("-p").arg(&config.msg);
    cmd.arg("--output-format").arg("json");
    cmd.arg("--permission-mode").arg("dontAsk");
    if config.read_only {
        cmd.arg("--disallowedTools")
            .arg("Write,Edit,MultiEdit,TodoWrite");
    }
    if let Some(id) = &config.resume_session {
        cmd.arg("--resume").arg(id);
    }
    cmd
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib chat::tests 2>&1 | tail -10`
Expected: all 7 tests PASS

**Step 5: Commit**

```bash
git add src/app/chat.rs
git commit -m "Add SpawnConfig and build_claude_cmd pure function (#5)"
```

---

### Task 3: Refactor `spawn_claude` to use `SpawnConfig`

**Files:**
- Modify: `src/app/chat.rs:222-313` (`spawn_claude` method)
- Modify: `src/app/chat.rs:213-217` (`send_chat_message`)

**Step 1: Refactor `spawn_claude` signature**

Change `spawn_claude` from:
```rust
pub(crate) fn spawn_claude(&mut self, msg: String, resume: bool, read_only: bool, cwd: Option<&str>, model: Option<&str>, setup: Option<String>)
```
to:
```rust
pub(crate) fn spawn_claude(&mut self, config: SpawnConfig)
```

Replace the closure body to use `build_claude_cmd`:

```rust
pub(crate) fn spawn_claude(&mut self, config: SpawnConfig) {
    self.chat.error = None;
    self.chat.waiting = true;
    self.chat.waiting_since = Some(Instant::now());
    self.chat.child_pid.store(0, Ordering::Relaxed);

    let session_id = if config.resume_session.is_some() {
        config.resume_session.clone()
    } else {
        None
    };
    // Merge resume_session with live session_id for resuming
    let config = SpawnConfig {
        resume_session: if config.resume_session.is_some() {
            self.chat.session_id.clone()
        } else {
            None
        },
        ..config
    };
    let pid_handle = Arc::clone(&self.chat.child_pid);

    let (tx, rx) = mpsc::channel();
    self.chat.response_rx = Some(rx);

    thread::spawn(move || {
        if let Some(setup_cmd) = &config.setup {
            let _ = Command::new("sh").arg("-c").arg(setup_cmd).output();
        }
        let cmd = build_claude_cmd(&config);

        let result = match cmd.spawn() {
            // ... same spawn/wait/parse logic as before ...
        };
        let _ = tx.send(result);
    });
}
```

Note: The `resume_session` handling needs care. Currently `spawn_claude` checks a local `resume` bool and reads `self.chat.session_id`. The new pattern: callers pass `resume_session: None` for new sessions. For resume, the **caller** decides whether to pass `self.chat.session_id` — this is cleaner since `SpawnConfig` is self-contained.

**Step 2: Update `send_chat_message`**

```rust
fn send_chat_message(&mut self, msg: String) {
    self.chat.messages.push(("user".into(), msg.clone()));
    let system_prompt = format!("{}{}", BASE_SYSTEM_PROMPT, self.chat.tone.suffix());
    let cwd = if self.cwd.as_os_str().is_empty() { None } else { Some(self.cwd.display().to_string()) };
    self.spawn_claude(SpawnConfig {
        msg,
        resume_session: self.chat.session_id.clone(),
        read_only: true,
        cwd,
        model: None,
        setup: None,
        system_prompt: Some(system_prompt),
        gateway: if self.gateway_enabled {
            self.gateway_url.as_ref().map(|url| GatewayConfig {
                url: url.clone(),
                headers: self.gateway_headers.clone(),
            })
        } else {
            None
        },
    });
}
```

**Step 3: Run full test suite**

Run: `cargo test --lib 2>&1 | tail -10`
Expected: FAIL — callers in `mod.rs` and `chat.rs` still use old signature

(This is expected — Task 4 fixes them.)

**Step 4: Commit (WIP, won't compile yet)**

Don't commit yet — complete Task 4 first, then commit together.

---

### Task 4: Update all callsites in `app/mod.rs` and `app/chat.rs`

**Files:**
- Modify: `src/app/mod.rs:334-346` (`tick()` auto-task launch)
- Modify: `src/app/chat.rs:82-96` (manual task Enter key)

**Step 1: Update `tick()` auto-task launch**

In `src/app/mod.rs:334-346`, change:

```rust
// Old:
self.spawn_claude(task.prompt, task.resume, task.read_only, Some(&cwd), None, task.setup);

// New:
let system_prompt = if task.use_advisor {
    Some(BASE_SYSTEM_PROMPT.to_string())
} else {
    None
};
let gateway = if self.gateway_enabled {
    self.gateway_url.as_ref().map(|url| chat::GatewayConfig {
        url: url.clone(),
        headers: self.gateway_headers.clone(),
    })
} else {
    None
};
self.spawn_claude(chat::SpawnConfig {
    msg: task.prompt,
    resume_session: if task.resume { self.chat.session_id.clone() } else { None },
    read_only: task.read_only,
    cwd: Some(cwd),
    model: None,
    setup: task.setup,
    system_prompt,
    gateway,
});
```

**Step 2: Update manual task (Enter key in task panel)**

In `src/app/chat.rs:82-96`, apply the same pattern — build `SpawnConfig` from the `task`, using `task.use_advisor` to decide `system_prompt`.

**Step 3: Extract a helper to reduce duplication**

Both callsites (tick auto-task and manual Enter) build `SpawnConfig` from `AutoTask` identically. Add a helper method on `App`:

```rust
fn config_for_task(&self, task: &crate::pipeline::AutoTask) -> SpawnConfig {
    let cwd = if task.cwd.is_empty() {
        if self.cwd.as_os_str().is_empty() { None } else { Some(self.cwd.display().to_string()) }
    } else {
        Some(task.cwd.clone())
    };
    let system_prompt = if task.use_advisor {
        Some(BASE_SYSTEM_PROMPT.to_string())
    } else {
        None
    };
    let gateway = if self.gateway_enabled {
        self.gateway_url.as_ref().map(|url| GatewayConfig {
            url: url.clone(),
            headers: self.gateway_headers.clone(),
        })
    } else {
        None
    };
    SpawnConfig {
        msg: task.prompt.clone(),
        resume_session: if task.resume { self.chat.session_id.clone() } else { None },
        read_only: task.read_only,
        cwd,
        model: None,
        setup: task.setup.clone(),
        system_prompt,
        gateway,
    }
}
```

Then both callsites become:
```rust
let config = self.config_for_task(&task);
self.spawn_claude(config);
```

**Step 4: Run full test suite**

Run: `cargo test 2>&1 | tail -10`
Expected: all tests PASS, compiles clean

**Step 5: Commit Tasks 3+4 together**

```bash
git add src/app/chat.rs src/app/mod.rs
git commit -m "Refactor spawn_claude to use SpawnConfig, skip advisor for pipeline tasks (#5)"
```

---

### Task 5: Create mock claude script

**Files:**
- Create: `tests/fixtures/mock_claude`

**Step 1: Create the mock script**

```bash
#!/bin/bash
# Mock claude CLI for testing. Writes received args to $MOCK_CLAUDE_ARGS_FILE
# and returns a valid JSON response.

if [ -n "$MOCK_CLAUDE_ARGS_FILE" ]; then
    echo "$@" > "$MOCK_CLAUDE_ARGS_FILE"
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

cat <<ENDJSON
[
  {"type":"init","session_id":"$SESSION_ID"},
  {"type":"result","session_id":"$SESSION_ID","result":"Mock response to: $MSG"}
]
ENDJSON
```

**Step 2: Make it executable**

Run: `chmod +x tests/fixtures/mock_claude`

**Step 3: Test manually**

Run: `MOCK_CLAUDE_ARGS_FILE=/tmp/test_args ./tests/fixtures/mock_claude -p "hello" --output-format json`
Expected: valid JSON output AND `/tmp/test_args` contains the args

**Step 4: Commit**

```bash
git add tests/fixtures/mock_claude
git commit -m "Add mock claude script for CI-safe E2E tests (#5)"
```

---

### Task 6: Add mock CLI PTY tests

**Files:**
- Modify: `tests/e2e.rs`

**Step 1: Add mock-based spawn helper**

Add near the top of `tests/e2e.rs`, after the existing `spawn_app` functions:

```rust
/// Spawn the app with `claude` overridden by mock script.
/// Returns (session, args_file_path) — read args_file after test to verify CLI args.
fn spawn_app_with_mock_claude(timeout_secs: u64) -> (OsSession, std::path::PathBuf) {
    let bin = env!("CARGO_BIN_EXE_cc-companion");
    let mock_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let args_file = std::env::temp_dir().join(format!("mock_claude_args_{}", std::process::id()));

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
```

**Step 2: Write test — chat message includes `--system-prompt`**

```rust
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

    // Wait for mock response
    app.expect("Mock response").expect("mock claude didn't respond");

    // Verify args
    let args = std::fs::read_to_string(&args_file).unwrap_or_default();
    assert!(args.contains("--system-prompt"), "chat should include --system-prompt, got: {}", args);

    app.send("q").unwrap();
    app.expect(Eof).unwrap();
    let _ = std::fs::remove_file(&args_file);
}
```

**Step 3: Write test — pipeline task skips `--system-prompt`**

```rust
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
    app.send("\r").unwrap();
    // IssueDriven asks for goal text — submit empty
    std::thread::sleep(Duration::from_millis(300));
    app.send("\r").unwrap();

    std::thread::sleep(Duration::from_millis(500));

    // Run the first pending task manually with Enter
    // Navigate to the pending task (past done=0, running=0)
    app.send("\r").unwrap();

    // Wait for mock response
    std::thread::sleep(Duration::from_secs(3));
    app.expect("Mock response").expect("mock claude didn't respond to pipeline task");

    // Verify args — should NOT have --system-prompt
    let args = std::fs::read_to_string(&args_file).unwrap_or_default();
    assert!(!args.contains("--system-prompt"),
        "pipeline task should NOT include --system-prompt, got: {}", args);

    app.send("\x1b").unwrap();
    std::thread::sleep(Duration::from_millis(300));
    app.send("q").unwrap();
    app.expect(Eof).unwrap();
    let _ = std::fs::remove_file(&args_file);
}
```

**Step 4: Run mock tests**

Run: `cargo test --test e2e test_mock 2>&1 | tail -15`
Expected: both PASS

**Step 5: Commit**

```bash
git add tests/e2e.rs
git commit -m "Add mock CLI E2E tests verifying system prompt behavior (#5)"
```

---

### Task 7: Add real E2E test (`#[ignore]`)

**Files:**
- Modify: `tests/e2e.rs`

**Step 1: Add ignored test for pipeline execution**

```rust
/// Test: Pipeline task executes successfully without Advisor prompt blocking it.
/// Proves the fix works end-to-end with real Claude.
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

    step!(t, "waiting for response (up to 120s)");
    app.expect("cc-companion:").expect("pipeline task got no response — Claude may have refused");

    step!(t, "quitting");
    app.send("\x1b").unwrap();
    std::thread::sleep(Duration::from_millis(300));
    app.send("q").unwrap();
    app.expect(Eof).unwrap();
    step!(t, "done");
}
```

**Step 2: Commit**

```bash
git add tests/e2e.rs
git commit -m "Add ignored E2E test for pipeline task execution (#5)"
```

---

### Task 8: Update test skill

**Files:**
- Modify: `.claude/skills/test/SKILL.md`

**Step 1: Add Mock CLI Testing section**

Append after the existing "Common Mistakes" table at the end of the file:

```markdown
## Mock CLI Testing

For testing CLI subprocess behavior without real API calls, use a mock script that mimics the CLI's output format.

### Pattern: mock script + PATH override

1. Create `tests/fixtures/mock_claude` (executable shell script):
   - Reads args, writes them to `$MOCK_CLAUDE_ARGS_FILE` for later assertion
   - Returns valid JSON in the expected output format

2. In PTY test, override PATH so `claude` resolves to the mock:

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

3. After the interaction, read `args_file` to verify which CLI flags were passed:

```rust
let args = std::fs::read_to_string(&args_file).unwrap_or_default();
assert!(args.contains("--system-prompt"), "expected --system-prompt in args");
```

### Key points
- Mock script must output the exact JSON format the app expects (`[{"type":"init",...},{"type":"result",...}]`)
- Clean up `args_file` at end of test
- Use `std::process::id()` in filename to avoid collisions in parallel test runs
```

**Step 2: Commit**

```bash
git add .claude/skills/test/SKILL.md
git commit -m "Add Mock CLI Testing section to test skill (#5)"
```

---

### Task 9: Final verification

**Step 1: Run full test suite**

Run: `cargo test 2>&1 | tail -20`
Expected: all tests PASS (unit + mock E2E, ignored tests skipped)

**Step 2: Run clippy**

Run: `cargo clippy 2>&1 | tail -20`
Expected: no warnings

**Step 3: Verify ignored tests list**

Run: `cargo test --test e2e -- --ignored --list 2>&1`
Expected: shows `test_ignored_4_pipeline_task_executes` in the list

**Step 4: Final commit if any fixups needed, then done**
