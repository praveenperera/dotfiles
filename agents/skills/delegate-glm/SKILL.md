---
name: delegate-glm
description: "Helper for delegating coding work or read-only analysis to OpenCode with Z.ai Coding Plan GLM 5.2."
disable-model-invocation: true
---

# Delegate GLM

Delegate the requested work to OpenCode with Z.ai Coding Plan GLM 5.2.

## Prepare the run

Create a per-run evidence directory:

```sh
scratch="_scratch/delegate-glm/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$scratch/prompts" "$scratch/raw" "$scratch/repository"
```

Record the repository baseline before invoking OpenCode:

```sh
git status --short > "$scratch/repository/baseline-status.txt"
git diff --no-ext-diff --binary > "$scratch/repository/baseline-diff.patch"
git diff --cached --no-ext-diff --binary > "$scratch/repository/baseline-cached-diff.patch"
git ls-files --others --exclude-standard -z |
  while IFS= read -r -d '' path; do
    shasum -a 256 -- "$path"
  done > "$scratch/repository/baseline-untracked-sha256.txt"
```

Choose `read-only planning`, `read-only review`, or `implementation` from the user's explicit request. Do not silently upgrade read-only work to implementation. Write `$scratch/prompts/task.md` using this complete contract:

```markdown
You are handling a delegated task in the current repository.

User request:
<paste the user's request verbatim>

Mode:
<read-only planning|read-only review|implementation>

Objective:
<one concrete outcome>

Authority:
<state whether the delegate may only inspect, or may edit files within Owned scope>

Owned scope:
<exact files, directories, or responsibility the delegate owns>

Excluded scope:
<files, systems, external state, and unrelated changes that must not be modified>

Evidence:
<repository files, logs, diffs, commands, or other artifacts to inspect>

Verification:
<specific checks to run, or "read-only analysis; no verification command required">

Success condition:
<observable conditions that mean the objective is complete>

Stop conditions:
<conditions requiring the delegate to stop without expanding authority, such as missing access, ambiguous destructive action, scope conflict, or required external-state change>

Operating rules:
- Read and obey applicable AGENTS.md files before acting.
- Inspect the supplied evidence and relevant repository context before concluding.
- Preserve pre-existing and concurrent changes; never revert work you do not own.
- In read-only mode, do not edit repository files.
- In implementation mode, edit only the Owned scope and run the specified verification.
- Do not commit, stage, push, open or modify pull requests, post comments, or change external state.
- Do not expand the scope or substitute a different model. Stop and report when a stop condition applies.

Final report:
- State the result against the Success condition.
- List changed files, or state that none changed.
- Report verification commands and outcomes.
- Report blockers, residual risks, and any scope conflict.
```

Make every field concrete. Preserve user constraints verbatim where possible. For implementation, grant edit authority only to the minimum owned scope; do not use phrases such as "edit anything needed."

## Invoke OpenCode

Use the plan agent for either read-only mode:

```sh
prompt=$(< "$scratch/prompts/task.md")
opencode run \
  --model zai-coding-plan/glm-5.2 \
  --agent plan \
  --format json \
  --dir "$PWD" \
  --title "delegate glm coding task" \
  "$prompt" \
  > "$scratch/raw/stdout.jsonl" \
  2> "$scratch/raw/stderr.txt"
exit_status=$?
printf '%s\n' "$exit_status" > "$scratch/raw/exit-status.txt"
```

Use the build agent for implementation:

```sh
prompt=$(< "$scratch/prompts/task.md")
opencode run \
  --model zai-coding-plan/glm-5.2 \
  --agent build \
  --format json \
  --dir "$PWD" \
  --title "delegate glm coding task" \
  "$prompt" \
  > "$scratch/raw/stdout.jsonl" \
  2> "$scratch/raw/stderr.txt"
exit_status=$?
printf '%s\n' "$exit_status" > "$scratch/raw/exit-status.txt"
```

Do not add automatic approval flags. Before substituting another model, ask the user.

## Inspect and report

Always record postflight state, including when OpenCode exits unsuccessfully:

```sh
git status --short > "$scratch/repository/postflight-status.txt"
git diff --no-ext-diff --binary > "$scratch/repository/postflight-diff.patch"
git diff --cached --no-ext-diff --binary > "$scratch/repository/postflight-cached-diff.patch"
git ls-files --others --exclude-standard -z |
  while IFS= read -r -d '' path; do
    shasum -a 256 -- "$path"
  done > "$scratch/repository/postflight-untracked-sha256.txt"
```

Inspect stdout, stderr, exit status, and every baseline/postflight status, diff, and untracked-file manifest. In read-only mode, treat any repository change as a failure and report it. In implementation mode, confirm every new change is within the owned scope and independently assess the requested verification before reporting completion. Do not claim success from the delegate's report alone.
