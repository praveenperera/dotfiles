---
name: delegate-grok
description: "Explicit-only helper for delegating coding work or read-only analysis to the Grok CLI (grok-4.5). Use only when the user explicitly invokes $delegate-grok or explicitly asks to delegate to Grok / the grok CLI."
---

# Delegate Grok

Open the Grok CLI with `grok-4.5` and delegate the current task. This is explicit-only; do not use it for ordinary requests unless the user names `$delegate-grok`, Grok CLI, or asks to delegate to Grok.

## Command

Save the prompt and raw output under `_scratch/delegate-grok/<timestamp>/`:

```sh
scratch="_scratch/delegate-grok/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$scratch/prompts" "$scratch/raw"
```

Choose the mode from the user request, then write `$scratch/prompts/task.md`:

```markdown
You are handling this coding task in the repository.

User request:
<paste the user's request>

Mode:
<read-only planning|read-only review|implementation>

Instructions:
- Inspect the repository before answering or editing.
- If mode is read-only, do not edit files.
- If mode is implementation, edit repository files as needed to complete the request.
- Run relevant verification commands when practical for implementation mode.
- Do not commit, push, comment on PRs, or change external state.
- Finish with the result, changed files if any, verification results if any, and blockers.
```

Run Grok in read-only mode:

```sh
grok \
  --prompt-file "$scratch/prompts/task.md" \
  --cwd "$PWD" \
  --permission-mode plan \
  --output-format json \
  -m grok-4.5 \
  > "$scratch/raw/grok-4.5.json"
```

Run Grok in implementation mode:

```sh
grok \
  --prompt-file "$scratch/prompts/task.md" \
  --cwd "$PWD" \
  --always-approve \
  --output-format json \
  -m grok-4.5 \
  > "$scratch/raw/grok-4.5.json"
```

Use `--permission-mode plan` for read-only mode. Use `--always-approve` for implementation mode so Grok can approve tool executions without prompts. Before substituting another model, ask the user. After Grok returns, inspect the diff yourself before reporting completion.
