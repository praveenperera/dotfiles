# Claude delegation

## Select the model and transport

Use the explicit model ID for the routed model: `claude-sonnet-5-5`, `claude-haiku-5-5`, or `claude-opus-5-5`. Pick the effort from the effort tables in [SKILL.md](../SKILL.md). The native Claude Agent tool with `model: sonnet`, `haiku`, or `opus` selects the model but inherits the session effort; use the CLI when the task needs a specific effort. Do not use Fable unless the user names it.

Check the installed CLI before using its flags:

```sh
claude --version
claude --help
claude auth status
```

Use the run directory, prompt contract, and postflight capture in [delegate-run.md](delegate-run.md). Capture model identity, result, exit status, and repository state. Do not configure a fallback model or silently accept a different model after a refusal or availability failure.

## Read-only review

Write the task to `$delegate_dir/prompts/task.md` before starting print mode. Review and investigation default to Sonnet at `high`:

```sh
claude -p --model claude-sonnet-5-5 --effort high \
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

Run from the repository directory. Add only necessary context directories with `--add-dir`. Plan mode and tool denies are permission controls, not a proof of OS-level isolation. Keep the task read-only, inspect the effective tool permissions, and check the resulting repository state.

Provide the diff, relevant paths, and user constraints without telling the reviewer which conclusion to reach. Do not send prior review verdicts into an independent review.

## Scoped implementation

Use a native writer with explicit owned paths when available. For CLI implementation, adapt the review command: set the routed model and effort (Sonnet at `medium`, Haiku at `medium`, or Opus at `medium` or `high`), use `--permission-mode acceptEdits`, remove the edit-tool denies, and retain `Agent,Task` denies plus `--permission-prompts none`. Allow only the specific local verification commands needed by the task through the installed permission controls. Do not use permission-bypass flags to avoid a headless prompt.

Keep commits, staging, publication, messages, external writes, and nested delegation outside the worker's scope. If a needed command is denied, report the exact command; the root can run an already-authorized check or adjust the scoped invocation. Do not change global permissions.

Inspect the diff and exact model identity before accepting the result. Complete required checks on the integrated code without repeating reliable checks on unchanged code. A final message or zero process exit alone does not establish completion.
