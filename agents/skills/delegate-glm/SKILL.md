---
name: delegate-glm
description: "Explicit-only helper for delegating coding work or read-only analysis to OpenCode with Z.ai Coding Plan GLM 5.2. Use only when the user explicitly invokes $delegate-glm or explicitly asks to delegate to OpenCode Z.ai Coding Plan / zai-coding-plan/glm-5.2."
---

# Delegate GLM

Open OpenCode with Z.ai Coding Plan GLM 5.2 and delegate the current task. This is explicit-only; do not use it for ordinary requests unless the user names `$delegate-glm`, OpenCode Z.ai Coding Plan, GLM 5.2, or `zai-coding-plan/glm-5.2`.

## Command

Save the prompt and raw output under `_scratch/delegate-glm/<timestamp>/`:

```sh
scratch="_scratch/delegate-glm/$(date +%Y%m%d-%H%M%S)"
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

Run OpenCode in read-only mode:

```sh
prompt=$(< "$scratch/prompts/task.md")
opencode run \
  --model zai-coding-plan/glm-5.2 \
  --agent plan \
  --format json \
  --dir "$PWD" \
  --title "delegate glm coding task" \
  "$prompt" \
  > "$scratch/raw/opencode-zai-glm-5.2.jsonl"
```

Run OpenCode in implementation mode:

```sh
prompt=$(< "$scratch/prompts/task.md")
opencode run \
  --model zai-coding-plan/glm-5.2 \
  --agent build \
  --auto \
  --format json \
  --dir "$PWD" \
  --title "delegate glm coding task" \
  "$prompt" \
  > "$scratch/raw/opencode-zai-glm-5.2.jsonl"
```

Use `--agent plan` for read-only mode. Use `--agent build --auto` for implementation mode so OpenCode can approve non-denied edit permissions. Before substituting another model, ask the user. After OpenCode returns, inspect the diff yourself before reporting completion.
