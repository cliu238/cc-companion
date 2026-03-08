# Usage API Rate Limit Experiment

Standalone script to determine safe polling intervals for `api.anthropic.com/api/oauth/usage`.

## Usage

```bash
# First run (conservative, ~2-4 hours)
uv run python experiment.py

# Resume after interruption
uv run python experiment.py --resume

# Include burst phase (risky, opt-in)
uv run python experiment.py --enable-burst

# Dry run (simulated, for testing the script itself)
uv run python experiment.py --dry-run
```

## Output

Results are saved to `experiment_output/` (or `--output-dir`):
- `log.txt` — one JSON line per request
- `checkpoint.json` — resume state
- `summary.txt` — final analysis

See issue #2 for context.
