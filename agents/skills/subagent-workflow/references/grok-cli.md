# Grok CLI delegation reference

## Preflight

Confirm the installed CLI, authentication, model availability, and exact flags:

```sh
command -v grok
grok --version
grok models | rg 'grok-4\.6'
grok --help
grok inspect
grok inspect --json
```

Save both `grok inspect` forms with the run artifacts. They name loaded permission sources and hooks. Do not modify login or global configuration automatically. If authentication, always-approve mode, the requested sandbox, or `grok-4.6` is unavailable, report the exact failure instead of substituting another model or running without a sandbox.

## Permission preflight invariant

A headless Grok pass may start only when all of the following hold. This is the permission boundary. It is not a complete command blacklist.

1. The command uses `--always-approve`. Grok documents that flag as the automation mode. Do not use `dontAsk`: in observed runs, one unapproved `run_terminal_command` call ended the full session as `cancelled` instead of letting Grok recover with an allowed tool.
2. `--sandbox` is `read-only` for analysis or `workspace` for implementation. Never use sandbox `off`.
3. Always-approve is not locked off in `requirements.toml`. CLI `--always-approve` wins over `~/.grok/config.toml`.
4. No shell `ask` rule is loaded. Open every permission source named by `grok inspect`. If any `permissions.ask` entry or native ask rule targets Bash, stop. Those rules still prompt under always-approve and can cancel a headless session. Do not edit the source files. A matching `--deny` for that exact command is valid explicit handling, because deny wins over ask; otherwise reject the pass.
5. The command includes deny rules for this repository's publication and external-mutation tools. Inspect the repository for those tools (git hosting CLIs, deploy wrappers, package publish scripts, messaging CLIs). Add a deny for each one. Omit only the exact actions that the user separately authorizes. If that inventory is not reliable, do not use Grok.

Increased tool permission does not expand the delegate's authority. Keep the prompt's owned paths and external-state limits.

Prefix rules match only at the start of a wrapper-peeled command. `Bash(git push:*)` becomes prefix `git push` and does not match `git -C <path> push`. Grok peels `env`, `timeout`, `nice`, and similar process wrappers; it does not peel `git -C`, `git -c`, or `sudo`. Write those flag forms explicitly. Do not add a long list of network tools and then treat a clean git diff as proof that nothing external ran.

The copy-paste commands below are the standard git, gh, and sudo backstop, including the git `-C` and `-c` publication forms. Append the repository-specific denies from step 5. Do not treat this list as complete.

## Use the shared evidence directory and prompt

Use the run directory, baseline capture, prompt contract, and postflight capture from [codex-cli.md](codex-cli.md). Put the complete prompt in `$delegate_dir/prompts/task.md`.

Grok headless mode starts a fresh session by default. Do not resume a prior session for an independent pass. Always pass `--no-subagents`; the orchestrator owns decomposition and integration.

Use `high` reasoning by default. Use `xhigh` only when the user requests it or when a consequential pass has evidence that the added cost and latency are useful. Do not silently lower the effort.

## Run a fresh read-only delegate

Use always-approve so inspection commands do not trigger a headless permission cancellation. Pair it with the read-only OS sandbox so the repository cannot be changed:

```sh
delegate_session_id="$(uuidgen | tr '[:upper:]' '[:lower:]')"
printf '%s\n' "$delegate_session_id" > "$delegate_dir/raw/session-id.txt"

grok \
  --prompt-file "$delegate_dir/prompts/task.md" \
  --cwd "$PWD" \
  --session-id "$delegate_session_id" \
  --model grok-4.6 \
  --reasoning-effort high \
  --always-approve \
  --sandbox read-only \
  --no-subagents \
  --disable-web-search \
  --verbatim \
  --deny 'Bash(git add:*)' \
  --deny 'Bash(git commit:*)' \
  --deny 'Bash(git push:*)' \
  --deny 'Bash(git -C * add*)' \
  --deny 'Bash(git -C * commit*)' \
  --deny 'Bash(git -C * push*)' \
  --deny 'Bash(git -c * add*)' \
  --deny 'Bash(git -c * commit*)' \
  --deny 'Bash(git -c * push*)' \
  --deny 'Bash(gh *)' \
  --deny 'Bash(sudo *)' \
  --output-format streaming-json \
  > "$delegate_dir/raw/events.ndjson" \
  2> "$delegate_dir/raw/stderr.txt"
delegate_exit_status=$?
printf '%s\n' "$delegate_exit_status" > "$delegate_dir/raw/exit-status.txt"

grok export "$delegate_session_id" "$delegate_dir/raw/transcript.md" \
  2> "$delegate_dir/raw/export-stderr.txt"
```

Omit `--disable-web-search` only when live web or X evidence is part of the assigned task. A web-enabled pass remains read-only unless the user separately authorizes an external mutation.

Keep the raw NDJSON. It records tool failures that the plain final output can omit. The transcript is for quick review; it does not replace the raw event capture. If the NDJSON does not explain a failure, export a local trace with `grok trace "$delegate_session_id" --local --output "$delegate_dir/raw/session-trace.tar.gz"`.

## Run a fresh implementation delegate

Use always-approve with the workspace sandbox. Grok can inspect, edit, and run the required verification without interactive permission prompts. Keep the standard publication denies and append denies for the repository's actual publication tools:

```sh
delegate_session_id="$(uuidgen | tr '[:upper:]' '[:lower:]')"
printf '%s\n' "$delegate_session_id" > "$delegate_dir/raw/session-id.txt"

grok \
  --prompt-file "$delegate_dir/prompts/task.md" \
  --cwd "$PWD" \
  --session-id "$delegate_session_id" \
  --model grok-4.6 \
  --reasoning-effort high \
  --always-approve \
  --sandbox workspace \
  --no-subagents \
  --disable-web-search \
  --verbatim \
  --deny 'Bash(git add:*)' \
  --deny 'Bash(git commit:*)' \
  --deny 'Bash(git push:*)' \
  --deny 'Bash(git -C * add*)' \
  --deny 'Bash(git -C * commit*)' \
  --deny 'Bash(git -C * push*)' \
  --deny 'Bash(git -c * add*)' \
  --deny 'Bash(git -c * commit*)' \
  --deny 'Bash(git -c * push*)' \
  --deny 'Bash(gh *)' \
  --deny 'Bash(sudo *)' \
  --output-format streaming-json \
  > "$delegate_dir/raw/events.ndjson" \
  2> "$delegate_dir/raw/stderr.txt"
delegate_exit_status=$?
printf '%s\n' "$delegate_exit_status" > "$delegate_dir/raw/exit-status.txt"

grok export "$delegate_session_id" "$delegate_dir/raw/transcript.md" \
  2> "$delegate_dir/raw/export-stderr.txt"
```

The workspace sandbox limits writes to the working directory, Grok state, and temporary directories. It does not enforce the prompt's narrower owned scope. If a formatter or verification command can modify files outside owned scope, run it independently after integration instead of asking Grok to run it.

## Inspect and integrate

Always record postflight state, including after a nonzero exit. Inspect the exit status, NDJSON events, transcript, stderr, baseline, and postflight artifacts. A zero exit and Grok's report are not proof of correctness.

- Treat any read-only repository mutation as a failed pass.
- Reject implementation changes outside owned scope.
- Verify important claims against the source and complete required checks on the integrated code. Reuse reliable results for unchanged code; repeat only for changes, failures, missing evidence, or unresolved concerns.
- Treat `cancelled` after a tool call as a permission or hook failure unless there is separate evidence that the user stopped the run.
- If a deny rule or hook blocks a required action, do not remove or weaken it automatically. Report the exact action and reroute the work or ask for the specific authority.
- Do not treat a clean git postflight as proof that no external mutation occurred. Check the raw events for network-capable commands and every repository-specific publication tool.

## Current command sources

- [Grok Build headless mode](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/14-headless-mode.md)
- [Grok Build sandbox mode](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/18-sandbox.md)
- [Grok Build permissions and safety](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/22-permissions-and-safety.md)
- the locally installed `grok --help`, `grok models`, and `grok inspect` output
