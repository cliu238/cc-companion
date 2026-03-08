# CLAUDE.md

## Code Style

> **Note:** Use best RUST language practice.
> **Note:** always use SIMPLEST code and structure, don't over enginnering
> **Note:** don't create unnecessary files. When creating new version of a file, archive or delete the legacy file
> **Note:** DO NOT ADD UNNECESSARY FEATURES! keep log simple

## Behavior

> **Note:** When user says "只需要", "不用", "先不要", "只要", "不需要改code", or "just need" — treat these as **hard scope constraints**. Do exactly what was asked, nothing more. Do not "helpfully" expand scope.
> **Note:** Do not trust documentation or assumptions for critical values (service names, data formats, API behavior). Verify against actual source code or runtime output before using.
> **Note:** Do not ask questions whose answers are obvious from context or irrelevant to the task. If the user gave clear instructions, execute them.

## Testing

> **Note:** You cannot spawn `claude` CLI from within Claude Code's Bash tool (nested sessions are blocked). For E2E tests that call `claude` headless, ignore it
> **Note:** When a plan identifies edge cases or boundary conditions, write a unit test for each BEFORE implementing. "All tests pass" only proves existing tests pass — missing tests hide bugs.

## E2E Testing Tips
- If a test hangs or is unexpectedly slow, add `eprintln!` timestamps at each step and run with `--nocapture` to identify which step is stuck.

## Documentation References
