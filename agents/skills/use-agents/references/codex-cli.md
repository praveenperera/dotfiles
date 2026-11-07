# Codex CLI delegation reference

Use `codex exec` for read-only runs only: GPT-6.1 Sol (`gpt-6.1-sol`) at `xhigh` for reviews, and GPT-6 Astra (`gpt-6-astra`) for rare second opinions. Both draw on the single Codex plan, so do not send implementation or exploration here unless the user asks for Sol through the `use-sol` skill.

## Preflight

Confirm the installed CLI, authentication, model availability, and exact flags instead of relying on memory:

```sh
command -v codex
codex --version
codex login status
codex exec --help
codex debug models | rg 'gpt-6.1-sol|gpt-6-astra'
```

Do not modify login or global configuration automatically. If authentication or the requested model is unavailable, report the exact failure.

## Run a fresh read-only session

Use the run directory, prompt contract, and postflight capture in [delegate-run.md](delegate-run.md), with the prompt in read-only mode. Set the model and effort for the role: Sol at `xhigh` for a review, or Astra at `low` for a focused question and `high` for a broad or high-risk one:

```sh
delegate_model=gpt-6.1-sol   # or gpt-6-astra
delegate_effort=xhigh        # Astra: low or high
codex --ask-for-approval never exec \
  --cd "$PWD" \
  --ephemeral \
  --model "$delegate_model" \
  --config "model_reasoning_effort=\"$delegate_effort\"" \
  --output-last-message "$delegate_dir/raw/final.md" \
  - \
  < "$delegate_dir/prompts/task.md" \
  > "$delegate_dir/raw/stdout.txt" \
  2> "$delegate_dir/raw/stderr.txt"
delegate_exit_status=$?
printf '%s\n' "$delegate_exit_status" > "$delegate_dir/raw/exit-status.txt"
```

Add `--json` only when the event stream is useful; `--output-last-message` already captures the final natural-language report. Do not use a priority service tier unless the user explicitly requested it.

Give Sol the diff, relevant paths, and user constraints without telling it which conclusion to reach, and do not send prior review verdicts into an independent review. Give Astra the goal, the options, and the evidence so far. The root keeps the decision. Use `codex exec resume <session>` only when continuity is essential and the first run was intentionally persisted without `--ephemeral`.
