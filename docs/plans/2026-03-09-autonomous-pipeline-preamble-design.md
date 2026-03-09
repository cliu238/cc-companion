# Autonomous Pipeline Preamble

**Issue:** #7 — Pipeline tasks should auto-complete without waiting for user confirmation

## Problem

Pipeline tasks spawned via `claude` CLI don't know they're running unattended. Claude's default conversational behavior causes it to present findings and wait for user feedback, stalling the pipeline.

## Solution

Add a shared `AUTONOMOUS_PREAMBLE` constant in `src/pipeline/mod.rs` prepended to all task prompts at dispatch time.

### Preamble

```
You are running in an automated pipeline with no human operator.
Complete the task fully and autonomously. Do not ask questions or wait for confirmation.
Make decisions yourself. When done, output your final result and stop.
If you are blocked and cannot proceed, output FAILED=<reason> and stop.
```

### Application Point

Prepend in `Scheduler::next_task()` and `Scheduler::run_task()` before returning the task. Applies uniformly to all pipeline tasks regardless of pipeline type.

### Files Changed

- `src/pipeline/mod.rs` — add constant, prepend in `next_task()`/`run_task()`

### Tests

- Unit test: dispatched task prompt contains preamble
- Existing tests unaffected (they call `initial_tasks`/`on_complete` directly)
