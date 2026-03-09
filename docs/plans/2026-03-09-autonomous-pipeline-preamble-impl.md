# Autonomous Pipeline Preamble Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Prepend an autonomous-mode preamble to all pipeline task prompts so Claude completes tasks without waiting for user input.

**Architecture:** Add a `AUTONOMOUS_PREAMBLE` constant in `src/pipeline/mod.rs`. Prepend it to `task.prompt` in `Scheduler::next_task()` and `Scheduler::run_task()` before returning.

**Tech Stack:** Rust

---

### Task 1: Write failing tests for preamble prepending

**Files:**
- Modify: `src/pipeline/mod.rs` (test section, ~line 63-169)

**Step 1: Write the failing tests**

Add these tests at the end of the `mod tests` block in `src/pipeline/mod.rs`:

```rust
#[test]
fn test_next_task_prepends_preamble() {
    let mut sched = Scheduler::new(Pipeline::Example, "/tmp", "");
    sched.enabled = true;
    assert!(!sched.tasks.is_empty());
    let task = sched.next_task();
    assert!(task.prompt.starts_with("You are running in an automated pipeline"),
        "next_task() should prepend autonomous preamble to prompt");
}

#[test]
fn test_run_task_prepends_preamble() {
    let mut sched = Scheduler::new(Pipeline::Example, "/tmp", "");
    sched.enabled = true;
    assert!(!sched.tasks.is_empty());
    let task = sched.run_task(0);
    assert!(task.prompt.starts_with("You are running in an automated pipeline"),
        "run_task() should prepend autonomous preamble to prompt");
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib pipeline::tests::test_next_task_prepends_preamble pipeline::tests::test_run_task_prepends_preamble -- --nocapture`

Expected: FAIL — prompts don't start with preamble yet.

**Step 3: Commit failing tests**

```bash
git add src/pipeline/mod.rs
git commit -m "test: add failing tests for autonomous preamble (#7)"
```

---

### Task 2: Add preamble constant and prepend logic

**Files:**
- Modify: `src/pipeline/mod.rs:7-11` (add constant after existing constants)
- Modify: `src/pipeline/mod.rs:220-232` (`next_task()` and `run_task()` methods)

**Step 1: Add the constant**

After the existing `TRIGGER_*` constants (line 10), add:

```rust
const AUTONOMOUS_PREAMBLE: &str = "\
You are running in an automated pipeline with no human operator.\n\
Complete the task fully and autonomously. Do not ask questions or wait for confirmation.\n\
Make decisions yourself. When done, output your final result and stop.\n\
If you are blocked and cannot proceed, output FAILED=<reason> and stop.\n\n";
```

**Step 2: Modify `next_task()` to prepend**

Change `next_task()` from:

```rust
pub fn next_task(&mut self) -> AutoTask {
    let task = self.tasks.remove(0);
    self.running = Some(task.name.clone());
    self.running_resume = task.resume;
    task
}
```

To:

```rust
pub fn next_task(&mut self) -> AutoTask {
    let mut task = self.tasks.remove(0);
    self.running = Some(task.name.clone());
    self.running_resume = task.resume;
    task.prompt = format!("{}{}", AUTONOMOUS_PREAMBLE, task.prompt);
    task
}
```

**Step 3: Modify `run_task()` to prepend**

Change `run_task()` from:

```rust
pub fn run_task(&mut self, idx: usize) -> AutoTask {
    let task = self.tasks.remove(idx);
    self.running = Some(task.name.clone());
    self.running_resume = task.resume;
    task
}
```

To:

```rust
pub fn run_task(&mut self, idx: usize) -> AutoTask {
    let mut task = self.tasks.remove(idx);
    self.running = Some(task.name.clone());
    self.running_resume = task.resume;
    task.prompt = format!("{}{}", AUTONOMOUS_PREAMBLE, task.prompt);
    task
}
```

**Step 4: Run all tests**

Run: `cargo test --lib`

Expected: ALL PASS — including the two new preamble tests and all existing tests.

**Step 5: Commit**

```bash
git add src/pipeline/mod.rs
git commit -m "feat: prepend autonomous preamble to pipeline task prompts (#7)"
```
