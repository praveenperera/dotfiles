# Codex CLI delegation reference

Use `codex exec` only for Astra, the read-only advisor. Do not send implementation to Astra.

## Preflight

Confirm the installed CLI, authentication, model availability, and exact flags instead of relying on memory:

```sh
command -v codex
codex --version
codex login status
codex exec --help
codex debug models | rg 'gpt-6-astra'
```

Do not modify login or global configuration automatically. If authentication or the requested model is unavailable, report the exact failure.

## Run a fresh read-only advisor

Use the run directory, prompt contract, and postflight capture in [delegate-run.md](delegate-run.md), with the prompt in read-only mode. Use `low` effort for a focused question and `high` for a broad or high-risk one:

```sh
codex --ask-for-approval never exec \
  --cd "$PWD" \
  --ephemeral \
  --model gpt-6-astra \
  --config 'model_reasoning_effort="low"' \
  --output-last-message "$delegate_dir/raw/final.md" \
  - \
  < "$delegate_dir/prompts/task.md" \
  > "$delegate_dir/raw/stdout.txt" \
  2> "$delegate_dir/raw/stderr.txt"
delegate_exit_status=$?
printf '%s\n' "$delegate_exit_status" > "$delegate_dir/raw/exit-status.txt"
```

Add `--json` only when the event stream is useful; `--output-last-message` already captures the final natural-language report. Do not use a priority service tier unless the user explicitly requested it.

Give Astra the goal, the options, and the evidence so far. The root keeps the decision. Use `codex exec resume <session>` only when continuity is essential and the first run was intentionally persisted without `--ephemeral`.
