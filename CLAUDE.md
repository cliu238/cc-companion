# CLAUDE.md

> **Note:** Use best RUST language practice.
> **Note:** always use SIMPLEST code and structure, don't over enginnering
> **Note:** don't create unnecessary files. When creating new version of a file, archive or delete the legacy file
> **Note:** DO NOT ADD UNNECESSARY FEATURES! keep log simple

## Testing

> **Note:** You cannot spawn `claude` CLI from within Claude Code's Bash tool (nested sessions are blocked). For E2E tests that call `claude` headless, ignore it
> **Note:** When a plan identifies edge cases or boundary conditions, write a unit test for each BEFORE implementing. "All tests pass" only proves existing tests pass — missing tests hide bugs.

## E2E Testing Tips
- If a test hangs or is unexpectedly slow, add `eprintln!` timestamps at each step and run with `--nocapture` to identify which step is stuck.

## Documentation References
