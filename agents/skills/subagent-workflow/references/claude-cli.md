# Claude delegation

## Select the model and transport

Use a native Claude Agent tool when it exposes the requested model. Verify that a `fable` alias resolves to Fable 5.1; do not assume an older alias or a client name identifies the version. For the CLI, use the explicit model ID `claude-fable-5-1`. Use `claude-opus-5` only for a selected Opus pass. Default both to `high` effort.

Check the installed CLI before using its flags:

```sh
claude --version
claude --help
claude auth status
```

Use the same scope, completion conditions, and evidence requirements as other delegates. Capture model identity, result, exit status, and repository state. Do not configure a fallback model or silently accept a different model after a refusal or availability failure.

## Read-only review

Use the run directory and baseline capture in [codex-cli.md](codex-cli.md). Write the task to `$delegate_dir/prompts/task.md` before starting print mode:

```sh
claude -p --model claude-fable-5-1 --effort high \
  --permission-mode plan \
  --permission-prompts none \
  --disallowedTools 'Edit,Write,NotebookEdit,Agent,Task' \
  --no-session-persistence \
  --output-format json \
  < "$delegate_dir/prompts/task.md" \
  > "$delegate_dir/raw/result.json" \
  2> "$delegate_dir/raw/stderr.txt"
delegate_exit_status=$?
printf '%s\n' "$delegate_exit_status" > "$delegate_dir/raw/exit-status.txt"
```

Run from the repository directory. Add only necessary context directories with `--add-dir`. Plan mode and tool denies are permission controls, not a proof of OS-level isolation. Keep the task read-only, inspect the effective tool permissions, and check the resulting repository state. If strict filesystem isolation is required, use an available sandbox or isolated copy.

Provide the diff, relevant paths, and user constraints without telling the reviewer which conclusion to reach. Do not send prior review verdicts into an independent review.

## Scoped implementation

Use a native writer with explicit owned paths when available. For CLI implementation, adapt the review command to `--permission-mode acceptEdits`, remove the edit-tool denies, and retain `Agent,Task` denies plus `--permission-prompts none`. Allow only the specific local verification commands needed by the task through the installed permission controls. Do not use permission-bypass flags to avoid a headless prompt.

Keep commits, staging, publication, messages, external writes, and nested delegation outside the worker's scope. If a needed command is denied, report the exact command; the root can run an already-authorized check or adjust the scoped invocation. Do not change global permissions.

Inspect the diff and exact model identity before accepting the result. Complete required checks on the integrated code without repeating reliable checks on unchanged code. A final message or zero process exit alone does not establish completion.

## Sources

- [Fable 5.1 model documentation](https://platform.claude.com/docs/en/models/fable-5-1/overview)
- Installed `claude --help`, checked on 2026-09-04
