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

In the task prompt, tell Grok to use its built-in `read_file`, `list_dir`, and `grep` tools for repository inspection. If the task requires shell inspection that those tools cannot perform, add an exact `Bash(...)` allow rule before the run. Match the complete command, including pipes, redirections, and arguments; do not grant a general shell pattern.

In a headless `dontAsk` or `acceptEdits` run, Grok can receive an unapproved tool call as `User cancelled`. Tell it to treat that result as a permission denial, not as a user stop, and retry once with an approved built-in inspection tool. If no approved tool can perform the action, it must report the exact rejected action and stop. The orchestrator may then add only that exact safe allow rule in a fresh pass.

Do not add `--no-plan` to recover from a permission denial. Plan mode and the `grok-build-plan` agent are independent of tool permissions.

## Run a fresh read-only delegate

Use both deny-by-default permissions and the read-only OS sandbox:

```sh
delegate_session_id="$(uuidgen | tr '[:upper:]' '[:lower:]')"
printf '%s\n' "$delegate_session_id" > "$delegate_dir/raw/session-id.txt"

grok \
  --prompt-file "$delegate_dir/prompts/task.md" \
  --cwd "$PWD" \
  --session-id "$delegate_session_id" \
  --model grok-4.6 \
  --reasoning-effort high \
  --permission-mode dontAsk \
  --sandbox read-only \
  --no-subagents \
  --disable-web-search \
  --verbatim \
  --output-format streaming-json \
  > "$delegate_dir/raw/events.ndjson" \
  2> "$delegate_dir/raw/stderr.txt"
delegate_exit_status=$?
printf '%s\n' "$delegate_exit_status" > "$delegate_dir/raw/exit-status.txt"

grok export "$delegate_session_id" "$delegate_dir/raw/transcript.md" \
  2> "$delegate_dir/raw/export-stderr.txt"
```

Omit `--disable-web-search` only when live web or X evidence is part of the assigned task. A web-enabled pass remains read-only unless the user separately authorizes an external mutation.

Keep the raw NDJSON. It records session updates and tool failures that the plain final output can omit. The transcript is for quick review; it does not replace the raw event capture. If the NDJSON does not explain a failure, export a local trace with `grok trace "$delegate_session_id" --local --output "$delegate_dir/raw/session-trace.tar.gz"`.

## Run a fresh implementation delegate

Use `acceptEdits` so Grok can edit files without an approval prompt. Add one exact `--allow` rule for each required verification command and each shell inspection command that is not in Grok's built-in read-only set. The examples below are placeholders; replace them with the real commands:

```sh
delegate_session_id="$(uuidgen | tr '[:upper:]' '[:lower:]')"
printf '%s\n' "$delegate_session_id" > "$delegate_dir/raw/session-id.txt"

grok \
  --prompt-file "$delegate_dir/prompts/task.md" \
  --cwd "$PWD" \
  --session-id "$delegate_session_id" \
  --model grok-4.6 \
  --reasoning-effort high \
  --permission-mode acceptEdits \
  --sandbox workspace \
  --no-subagents \
  --disable-web-search \
  --verbatim \
  --allow 'Bash(exact verification command)' \
  --allow 'Bash(exact shell inspection command, when required)' \
  --output-format streaming-json \
  > "$delegate_dir/raw/events.ndjson" \
  2> "$delegate_dir/raw/stderr.txt"
delegate_exit_status=$?
printf '%s\n' "$delegate_exit_status" > "$delegate_dir/raw/exit-status.txt"

grok export "$delegate_session_id" "$delegate_dir/raw/transcript.md" \
  2> "$delegate_dir/raw/export-stderr.txt"
```

`acceptEdits` approves file edits, not every shell command. Unmatched shell calls can still fail in a headless run, so keep the exact command allows. Do not use `--always-approve`, `bypassPermissions`, or sandbox `off` for delegated work. Do not allow commit, staging, push, pull-request, deployment, messaging, or other external-state commands.

The `workspace` sandbox limits writes to the working directory, Grok state, and temporary directories. `acceptEdits` does not enforce the prompt's narrower owned scope, so baseline and postflight comparison is still required. If a formatter or verification command can modify files outside owned scope, do not grant it to the delegate; run it independently after integration.

## Inspect and integrate

Always record postflight state, including after a nonzero exit. Inspect the exit status, NDJSON events, transcript, stderr, baseline, and postflight artifacts. A zero exit and Grok's report are not proof of correctness.

- Treat any read-only repository mutation as a failed pass.
- Reject implementation changes outside owned scope.
- Verify important claims against the source and run the required repository checks independently.
- Treat `User cancelled` from a tool call in a headless permission mode as a permission denial unless there is separate evidence that the user stopped the run. Confirm it in the NDJSON or trace. If Grok did not recover with its one approved-tool retry, add only the exact safe rule the task needs in a fresh pass. Do not relax the whole permission mode or sandbox.

## Current command sources

- [Grok Build headless mode](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/14-headless-mode.md)
- [Grok Build sandbox mode](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/18-sandbox.md)
- [Grok Build permissions and safety](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/22-permissions-and-safety.md)
- the locally installed `grok --help` and `grok models` output
