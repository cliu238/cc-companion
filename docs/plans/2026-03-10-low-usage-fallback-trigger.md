# Low-Usage Fallback Trigger Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Pipeline auto-executes when usage is below safe caps, instead of only near reset windows.

**Architecture:** Add a third "fallback" trigger to `should_launch()` that fires when both 5h and 7d usage are below conservative thresholds. Fix the status bar to show "idle" instead of "??" when 5h `resets_at` is `None`.

**Tech Stack:** Rust, chrono, ratatui

---

### Task 1: Add failing tests for low-usage fallback trigger

**Files:**
- Modify: `src/pipeline/mod.rs:69-148` (test module)

**Step 1: Write failing tests**

Add these tests after the existing `test_should_launch_too_high_pct` test (line 118):

```rust
#[test]
fn test_should_launch_fallback_low_usage() {
    let mut sched = Scheduler::new(Pipeline::Example, "/tmp", "");
    sched.enabled = true;
    // Low usage, but reset is far away (2 hours for 5h, 2 days for 7d)
    // Neither near-reset trigger fires, but fallback should
    let usage = make_usage(20.0, 120, 30.0, 2 * 24 * 60);
    assert!(sched.should_launch(&usage, false), "fallback should fire when usage is low");
}

#[test]
fn test_should_launch_fallback_blocked_by_high_5h() {
    let mut sched = Scheduler::new(Pipeline::Example, "/tmp", "");
    sched.enabled = true;
    // 5h usage above fallback cap, 7d low, reset far away
    let usage = make_usage(85.0, 120, 30.0, 2 * 24 * 60);
    assert!(!sched.should_launch(&usage, false), "fallback must not fire when 5h usage high");
}

#[test]
fn test_should_launch_fallback_blocked_by_high_7d() {
    let mut sched = Scheduler::new(Pipeline::Example, "/tmp", "");
    sched.enabled = true;
    // 5h low, 7d usage above fallback cap, reset far away
    let usage = make_usage(20.0, 120, 92.0, 2 * 24 * 60);
    assert!(!sched.should_launch(&usage, false), "fallback must not fire when 7d usage high");
}

#[test]
fn test_should_launch_fallback_none_resets_at() {
    let mut sched = Scheduler::new(Pipeline::Example, "/tmp", "");
    sched.enabled = true;
    // resets_at is None (0% usage, API returns null) — fallback should still fire
    let usage = UsageStatus {
        five_hour_pct: 0.0,
        five_hour_resets_at: None,
        seven_day_pct: 22.0,
        seven_day_resets_at: Some(Utc::now() + Duration::minutes(2 * 24 * 60)),
        seven_day_sonnet_pct: Some(4.0),
        last_fetched: Utc::now(),
    };
    assert!(sched.should_launch(&usage, false), "fallback should fire even when resets_at is None");
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p cc-companion --lib pipeline::tests -- --nocapture 2>&1 | tail -20`
Expected: 4 new tests FAIL (fallback logic doesn't exist yet)

**Step 3: Commit**

```bash
git add src/pipeline/mod.rs
git commit -m "test: add failing tests for low-usage fallback trigger (#7)"
```

---

### Task 2: Implement fallback trigger in `should_launch()`

**Files:**
- Modify: `src/pipeline/mod.rs:7-10` (constants) and `src/pipeline/mod.rs:220-244` (`should_launch`)

**Step 1: Add fallback constants after existing constants (line 10)**

```rust
const FALLBACK_MAX_5H_PCT: f64 = 80.0;
const FALLBACK_MAX_7D_PCT: f64 = 90.0;
```

**Step 2: Add fallback trigger to `should_launch()` (after line 241, before the final return)**

Replace the return statement at line 243:

```rust
        five_hour_trigger || seven_day_trigger
```

with:

```rust
        // Fallback: run when usage is comfortably below caps,
        // even if reset windows are far away.
        let fallback_trigger =
            usage.five_hour_pct < FALLBACK_MAX_5H_PCT
            && usage.seven_day_pct < FALLBACK_MAX_7D_PCT;

        five_hour_trigger || seven_day_trigger || fallback_trigger
```

**Step 3: Run tests to verify they pass**

Run: `cargo test -p cc-companion --lib pipeline::tests -- --nocapture 2>&1 | tail -20`
Expected: ALL tests pass (including 4 new ones)

**Step 4: Commit**

```bash
git add src/pipeline/mod.rs
git commit -m "feat: add low-usage fallback trigger to pipeline scheduler (#7)"
```

---

### Task 3: Display "idle" instead of "??" in status bar

**Files:**
- Modify: `src/ui.rs:506`

**Step 1: Change the None branch**

Replace line 506:

```rust
        None => "??".to_string(),
```

with:

```rust
        None => "idle".to_string(),
```

**Step 2: Verify build compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src/ui.rs
git commit -m "fix: show 'idle' instead of '??' when 5h reset time unavailable (#7)"
```

---

### Task 4: Force-refresh usage after task completion

**Files:**
- Modify: `src/app/mod.rs` (in the task completion handler, around line 288-296)

**Context:** After a pipeline task completes, the cached usage data may be stale (task consumed tokens). Force a fresh API fetch before the scheduler decides whether to launch the next task.

**Step 1: Write failing test**

Add a test that verifies after `complete_running` is called, a usage refresh is triggered (or at minimum, the cached usage is invalidated).

**Step 2: Implement force-refresh**

After the `self.tasks.scheduler.complete_running(output, &cwd)` call (around line 292), invalidate the usage cache so the next tick triggers a fresh fetch. The simplest approach: set `self.usage_status = None` or set `last_fetched` to epoch so the interval check triggers immediately.

**Step 3: Run tests to verify**

Run: `cargo test -p cc-companion 2>&1 | tail -20`
Expected: ALL tests pass

**Step 4: Commit**

```bash
git add src/app/mod.rs
git commit -m "feat: force-refresh usage data after pipeline task completion (#7)"
```

---

### Task 5: Verify all tests pass

**Step 1: Run full test suite**

Run: `cargo test -p cc-companion 2>&1 | tail -20`
Expected: ALL tests pass, no regressions

**Step 2: Final commit if any cleanup needed**
