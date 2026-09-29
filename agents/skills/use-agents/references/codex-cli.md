# Codex CLI delegation reference

## Preflight

Confirm the installed CLI, authentication, model availability, and exact flags instead of relying on memory:

```sh
command -v codex
codex --version
codex login status
codex exec --help
codex debug models | rg 'gpt-6-astra|gpt-6\.1-sol'
```

Do not modify login or global configuration automatically. If authentication or the requested model is unavailable, report the exact failure.

## Create evidence artifacts

Run from the repository root:

```sh
delegate_run_id="$(date +%Y%m%d-%H%M%S)-$$"
delegate_dir="_scratch/use-agents/$delegate_run_id"
mkdir -p "$delegate_dir/prompts" "$delegate_dir/raw" "$delegate_dir/repository"

git status --short > "$delegate_dir/repository/baseline-status.txt"
git diff --no-ext-diff --binary > "$delegate_dir/repository/baseline-diff.patch"
git diff --cached --no-ext-diff --binary > "$delegate_dir/repository/baseline-cached-diff.patch"
git ls-files --others --exclude-standard -z |
  while IFS= read -r -d '' delegate_path; do
    shasum -a 256 -- "$delegate_path"
  done > "$delegate_dir/repository/baseline-untracked-sha256.txt"
```

Put the complete prompt in `$delegate_dir/prompts/task.md`.

## Prompt contract

```markdown
# Delegated task

Mode: <read-only analysis|implementation>

## Objective

<one concrete outcome>

## Authority

<inspect only, or edit only the exact owned scope>

## Owned scope

<files, directories, or responsibility owned by this delegate>

## Excluded scope

<files, systems, external state, and unrelated changes that must not be modified>

## Evidence

<files, logs, diffs, commands, issue text, or other artifacts to inspect>

## Constraints

<user requirements, compatibility needs, minimality rules, and forbidden actions>

## Verification

<specific checks to run, or read-only analysis with no command required>

## Success condition

<observable conditions that mean the task is complete>

## Stop conditions

<missing access, ownership conflict, ambiguous destructive action, architecture mismatch, or required external change>

## Operating rules

- Read and obey applicable AGENTS.md files before acting.
- Inspect relevant repository context before concluding or editing.
- Do all work yourself. Do not spawn subagents, nested agents, or multi-agent orchestration.
- Preserve pre-existing and concurrent changes; never revert work you do not own.
- In read-only mode, do not edit any repository file.
- In implementation mode, edit only the owned scope.
- Do not commit, stage, push, open or modify pull requests, deploy, post messages, or change external state.
- Do not use a priority service tier unless the user explicitly requested it.
- Stop and report rather than expanding authority.

## Final report

- State the result against the success condition.
- List changed files, or state that none changed.
- Report verification commands and exact outcomes.
- Report blockers, residual risks, and scope conflicts.
```

## Run a fresh read-only delegate

Use Sol 6.1 at `high` for read-only review, audits, and investigation, or `xhigh` for a deep audit of a large or high-risk change:

```sh
codex --ask-for-approval never exec \
  --cd "$PWD" \
  --ephemeral \
  --model gpt-6.1-sol \
  --config 'model_reasoning_effort="high"' \
  --output-last-message "$delegate_dir/raw/final.md" \
  - \
  < "$delegate_dir/prompts/task.md" \
  > "$delegate_dir/raw/stdout.txt" \
  2> "$delegate_dir/raw/stderr.txt"
delegate_exit_status=$?
printf '%s\n' "$delegate_exit_status" > "$delegate_dir/raw/exit-status.txt"
```

Use Astra only as a read-only advisor for a second opinion on a big design or architecture decision. Change the model to `gpt-6-astra` and effort to `low` for a focused question or `high` for a broad or high-risk one, and keep the prompt in read-only mode. The root keeps the decision.

## Run a fresh implementation delegate

Run implementation only after assigning an exact owned scope. Sol 6.1 is the implementation worker; use `high` by default and `low` for easy mechanical work, following the effort table in [SKILL.md](../SKILL.md). Do not send implementation to Astra:

```sh
codex --ask-for-approval never exec \
  --cd "$PWD" \
  --ephemeral \
  --model gpt-6.1-sol \
  --config 'model_reasoning_effort="high"' \
  --output-last-message "$delegate_dir/raw/final.md" \
  - \
  < "$delegate_dir/prompts/task.md" \
  > "$delegate_dir/raw/stdout.txt" \
  2> "$delegate_dir/raw/stderr.txt"
delegate_exit_status=$?
printf '%s\n' "$delegate_exit_status" > "$delegate_dir/raw/exit-status.txt"
```

Add `--json` only when the event stream is useful; `--output-last-message` already captures the final natural-language report.

Use `codex exec review` or `codex review` only when their target flags match the requested review. A self-contained `codex exec` prompt is more flexible for repository analysis and custom review contracts.

## Handle duration without polling

For a run expected to exceed two minutes, submit it through the `homebased` skill. The daemon records the task lifecycle and sends a `HOMEBASED_EVENT` when the run ends. Use Claude Code's runtime-managed background execution only when homebased is unavailable.

Do not keep an agent active only to poll. On the next wake, reconcile the status file and process identity before trusting a notification. Treat the Codex final message as an artifact, not the source of truth.

## Record postflight state

Always run postflight capture, including after a nonzero exit:

```sh
git status --short > "$delegate_dir/repository/postflight-status.txt"
git diff --no-ext-diff --binary > "$delegate_dir/repository/postflight-diff.patch"
git diff --cached --no-ext-diff --binary > "$delegate_dir/repository/postflight-cached-diff.patch"
git ls-files --others --exclude-standard -z |
  while IFS= read -r -d '' delegate_path; do
    shasum -a 256 -- "$delegate_path"
  done > "$delegate_dir/repository/postflight-untracked-sha256.txt"
```

Inspect exit status, stdout, stderr, final message, baseline, and postflight artifacts. A read-only mutation is a failed pass. For implementation, reject changes outside owned scope. Inspect the diff and complete required verification on the integrated code. Reuse reliable results for unchanged code; repeat checks only for new changes, failures, missing evidence, or unresolved concerns.

## Choose follow-ups deliberately

Prefer a new ephemeral invocation when an independent perspective or clean repair context matters. Choose the number and kind of follow-ups from the task's evidence, risk, and expected value.

Use `codex exec resume <session>` only when continuity is essential and the first run was intentionally persisted without `--ephemeral`.
