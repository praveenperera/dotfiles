---
name: use-luna-max
description: Delegate implementation, tests, and mechanical repository work to GPT-5.6 Luna at max reasoning through the Codex CLI. Use only when the user explicitly says "use Luna", "use Luna sub-agents", or "use Luna Max". Do not use for any other request.
disable-model-invocation: false
---

# Use Luna Max

Delegate to the Codex CLI with model `gpt-5.6-luna` and reasoning effort `max`.

## Invocation gate

Use this skill only when the user explicitly names Luna: "use Luna", "use Luna sub-agents", "use Luna Max", or `/use-luna-max`. Do not use it for any other request. A task that looks like a good fit — tests, a rename, a bulk edit, a delegation — is not sufficient. Without the explicit words, do the work in the root thread.

## What Luna gets

When the gate is open, delegate to Luna:

- test writing and test extension, including fixtures and table cases
- mechanical or repetitive edits: renames, signature changes across call sites, import moves, format or API migrations
- bulk transforms, inventory, and classification over many files
- boilerplate that follows an established pattern already present in the repository

The directive stays in effect for the rest of the session, until the user cancels it. Luna writes all the implementation while it is active; do not write the implementation in the root thread. Keep sending Luna the next piece of work until the task is complete.

## What the root agent keeps

Keep in the root thread, whether or not "use Luna" is active:

- review, correctness judgment, and acceptance of Luna's output
- architecture, domain modeling, and API or UI surface design
- ideation, tradeoff analysis, and anything that needs knowledge Luna does not have

Give Luna a decided design and a stated scope. The root agent checks the work.

## Preflight

Confirm the CLI, the login, and the model instead of relying on memory:

```sh
command -v codex
codex login status
codex debug models | rg 'gpt-5\.6-luna'
```

If the CLI, the authentication, or the model is unavailable, report the exact failure. Do not substitute a different model.

## Prepare the run

Run from the repository root:

```sh
luna_run_id="$(date +%Y%m%d-%H%M%S)-$$"
luna_dir="_scratch/use-luna-max/$luna_run_id"
mkdir -p "$luna_dir/prompts" "$luna_dir/raw" "$luna_dir/repository"

git status --short > "$luna_dir/repository/baseline-status.txt"
git diff --no-ext-diff --binary > "$luna_dir/repository/baseline-diff.patch"
git diff --cached --no-ext-diff --binary > "$luna_dir/repository/baseline-cached-diff.patch"
```

Write the complete prompt to `$luna_dir/prompts/task.md`. The prompt must be self-contained; Luna does not see this session.

```markdown
# Delegated task

Mode: <tests|mechanical edit|implementation>

## Objective

<one concrete outcome, already decided>

## Owned scope

<exact files or directories Luna may edit>

## Excluded scope

<files, systems, and unrelated changes Luna must not touch>

## Approach

<the decided design, plus the steps, patterns, or existing example to follow>

## Evidence

<files to read, existing tests to imitate, relevant paths>

## Verification

<the exact command to run, and the expected result>

## Success condition

<observable conditions that mean the task is complete>

## Stop conditions

- The stated approach does not fit a case.
- A design decision outside the stated approach is required.
- The owned scope is not sufficient.
Stop and report instead of deciding.

## Operating rules

- Read and obey applicable AGENTS.md files before acting.
- Follow the patterns already in the repository; do not introduce new abstractions.
- Make the smallest change that satisfies the objective.
- Edit only the owned scope. Preserve all other changes.
- Do all work yourself. Do not spawn subagents.
- Do not commit, stage, push, open or modify pull requests, or change external state.

## Final report

- State the result against the success condition.
- List changed files, or state that none changed.
- Report the verification command and its exact output.
- Report blockers and anything skipped.
```

Split large implementation work into passes that each have their own objective, scope, and verification. Send the passes one after the other.

## Run Luna

Read-only work, such as inventory or classification:

```sh
codex --ask-for-approval never exec \
  --cd "$PWD" \
  --ephemeral \
  --model gpt-5.6-luna \
  --config 'model_reasoning_effort="max"' \
  --sandbox read-only \
  --output-last-message "$luna_dir/raw/final.md" \
  - \
  < "$luna_dir/prompts/task.md" \
  > "$luna_dir/raw/stdout.txt" \
  2> "$luna_dir/raw/stderr.txt"
printf '%s\n' "$?" > "$luna_dir/raw/exit-status.txt"
```

Tests, mechanical edits, and implementation:

```sh
codex --ask-for-approval never exec \
  --cd "$PWD" \
  --ephemeral \
  --model gpt-5.6-luna \
  --config 'model_reasoning_effort="max"' \
  --sandbox workspace-write \
  --output-last-message "$luna_dir/raw/final.md" \
  - \
  < "$luna_dir/prompts/task.md" \
  > "$luna_dir/raw/stdout.txt" \
  2> "$luna_dir/raw/stderr.txt"
printf '%s\n' "$?" > "$luna_dir/raw/exit-status.txt"
```

Rules:

- Keep `model_reasoning_effort="max"`. Lower efforts are for high-volume fan-out, which this skill does not cover.
- Use `--ephemeral` so every pass starts clean. Resume only when continuity is essential and the first run was persisted on purpose.
- Do not use `--dangerously-bypass-approvals-and-sandbox`.
- Add `--add-dir <path>` only when the owned scope requires it.
- For a run longer than two minutes, use background execution. Do not keep an agent active only to poll.

## Inspect and accept

Always capture postflight state, including after a nonzero exit:

```sh
git status --short > "$luna_dir/repository/postflight-status.txt"
git diff --no-ext-diff --binary > "$luna_dir/repository/postflight-diff.patch"
```

Then:

1. Read the diff yourself. Reject every change outside the owned scope.
2. Run the repository verification yourself. Do not accept Luna's report as proof.
3. For tests, confirm the tests fail without the change under test and pass with it. Reject tests that only restate literals or assert implementation details.
4. Simplify the result in the root thread when Luna produced repetition or an unnecessary abstraction.

The root agent owns the result. Luna's final message is an artifact, not a verdict.
