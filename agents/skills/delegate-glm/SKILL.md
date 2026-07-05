---
name: delegate-glm
description: "Explicit-only helper for delegating a coding plan to OpenCode with Z.ai Coding Plan GLM 5.2. Use only when the user explicitly invokes $delegate-glm or explicitly asks to delegate to OpenCode Z.ai Coding Plan / zai-coding-plan/glm-5.2."
---

# Delegate GLM

Open OpenCode with Z.ai Coding Plan GLM 5.2 and ask it for a coding plan. This is explicit-only; do not use it for ordinary planning requests unless the user names `$delegate-glm`, OpenCode Z.ai Coding Plan, GLM 5.2, or `zai-coding-plan/glm-5.2`.

## Command

Save the prompt and raw output under `_scratch/delegate-glm/<timestamp>/`:

```sh
scratch="_scratch/delegate-glm/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$scratch/prompts" "$scratch/raw"
```

Write the request to `$scratch/prompts/plan.md`:

```markdown
You are producing a coding plan only.

User request:
<paste the user's request>

Instructions:
- Inspect the repository before planning.
- Return a concise implementation plan with likely files, steps, verification, risks, and blocking questions.
- Do not edit files, commit, push, or change external state.
```

Run OpenCode:

```sh
prompt=$(< "$scratch/prompts/plan.md")
opencode run \
  --model zai-coding-plan/glm-5.2 \
  --format json \
  --dir "$PWD" \
  --title "delegate glm coding plan" \
  "$prompt" \
  > "$scratch/raw/opencode-zai-glm-5.2.jsonl"
```

Before substituting another model, ask the user. Treat the output as advice and verify it against the repo before making changes.
