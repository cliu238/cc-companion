# CLAUDE.md

> **Note:** Use best RUST language practice.
> **Note:** always use SIMPLEST code and structure, don't over enginnering
> **Note:** don't create unnecessary files. When creating new version of a file, archive or delete the legacy file
> **Note:** DO NOT ADD UNNECESSARY FEATURES! keep log simple

## Testing

> **Note:** You cannot spawn `claude` CLI from within Claude Code's Bash tool (nested sessions are blocked). For E2E tests that call `claude` headless, ask the user to run them in a separate terminal:
> ```bash
> cargo test --test e2e -- --ignored --test-threads=1
> ```

## Documentation References
