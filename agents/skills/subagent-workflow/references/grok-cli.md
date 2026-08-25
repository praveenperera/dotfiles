# Grok CLI delegation reference

## Preflight

Confirm the installed CLI, authentication, model availability, and exact flags:

```sh
command -v grok
grok --version
grok models | rg 'grok-4\.6'
grok --help
```

Do not modify login or global configuration automatically. If authentication or `grok-4.6` is unavailable, report the exact failure instead of substituting a model.

## Use the shared evidence directory and prompt

Use the run directory, baseline capture, prompt contract, and postflight capture from [codex-cli.md](codex-cli.md). Put the complete prompt in `$delegate_dir/prompts/task.md`.

Grok headless mode starts a fresh session by default. Do not resume a prior session for an independent pass. Always pass `--no-subagents`; the orchestrator owns decomposition and integration.

Use `high` reasoning by default. Use `xhigh` only when the user requests it or when a consequential pass has evidence that the added cost and latency are useful. Do not silently lower the effort.

## Run a fresh read-only delegate

Use both deny-by-default permissions and the read-only OS sandbox:

```sh
grok \
  --prompt-file "$delegate_dir/prompts/task.md" \
  --cwd "$PWD" \
  --model grok-4.6 \
  --reasoning-effort high \
  --permission-mode dontAsk \
  --sandbox read-only \
  --no-subagents \
  --disable-web-search \
  --verbatim \
  --output-format plain \
  > "$delegate_dir/raw/final.md" \
  2> "$delegate_dir/raw/stderr.txt"
delegate_exit_status=$?
printf '%s\n' "$delegate_exit_status" > "$delegate_dir/raw/exit-status.txt"
```

Omit `--disable-web-search` only when live web or X evidence is part of the assigned task. A web-enabled pass remains read-only unless the user separately authorizes an external mutation.

## Run a fresh implementation delegate

Use deny-by-default permissions. Add one exact `--allow` rule for each owned edit path and required verification command. The examples below are placeholders; replace them with the real scope and commands:

```sh
grok \
  --prompt-file "$delegate_dir/prompts/task.md" \
  --cwd "$PWD" \
  --model grok-4.6 \
  --reasoning-effort high \
  --permission-mode dontAsk \
  --sandbox workspace \
  --no-subagents \
  --disable-web-search \
  --verbatim \
  --allow 'Edit(path/to/owned/**)' \
  --allow 'Bash(exact verification command)' \
  --output-format plain \
  > "$delegate_dir/raw/final.md" \
  2> "$delegate_dir/raw/stderr.txt"
delegate_exit_status=$?
printf '%s\n' "$delegate_exit_status" > "$delegate_dir/raw/exit-status.txt"
```

`--allow` is not an allowlist by itself. `--permission-mode dontAsk` makes unmatched non-read-only calls fail, so keep both. Do not use `--always-approve`, `bypassPermissions`, or sandbox `off` for delegated work. Do not allow commit, staging, push, pull-request, deployment, messaging, or other external-state commands.

The `workspace` sandbox limits writes to the working directory, Grok state, and temporary directories, but it does not enforce the prompt's narrower owned scope. Baseline and postflight comparison is still required. If a formatter or verification command can modify files outside owned scope, do not grant it to the delegate; run it independently after integration.

## Inspect and integrate

Always record postflight state, including after a nonzero exit. Inspect the exit status, final message, stderr, baseline, and postflight artifacts. A zero exit and Grok's report are not proof of correctness.

- Treat any read-only repository mutation as a failed pass.
- Reject implementation changes outside owned scope.
- Verify important claims against the source and run the required repository checks independently.
- If a permission rule blocks a required action, add only the exact rule the task needs in a fresh pass. Do not relax the whole permission mode or sandbox.

## Current command sources

- [Grok Build headless mode](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/14-headless-mode.md)
- [Grok Build sandbox mode](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/18-sandbox.md)
- [Grok Build permissions and safety](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/22-permissions-and-safety.md)
- the locally installed `grok --help` and `grok models` output
