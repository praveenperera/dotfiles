# Delegate run reference

Every CLI transport uses the same run directory, prompt contract, and postflight capture. The transport references give the model flags and permission rules.

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

- Read and obey applicable AGENTS.md and CLAUDE.md files before acting.
- Inspect relevant repository context before concluding or editing.
- Do all work yourself. Do not spawn subagents, nested agents, or multi-agent orchestration.
- Preserve pre-existing and concurrent changes; never revert work you do not own.
- In read-only mode, do not edit any repository file.
- In implementation mode, edit only the owned scope.
- Do not commit, stage, push, open or modify pull requests, deploy, post messages, or change external state.
- Stop and report rather than expanding authority.
- Do not wait inside a command that may run longer than a few minutes. Write `RESUME.md` under `_scratch/` with the task id, what each outcome means, and your next steps; submit the command as a homebased `task` with `thread` set to your parent task's thread; then report `blocked` with `WAITING <task-id>` and exit.

## Final report

- State the result against the success condition.
- List changed files, or state that none changed.
- Report verification commands and exact outcomes.
- Report blockers, residual risks, and scope conflicts.
```

## Handle duration without polling

For a run expected to exceed two minutes, submit it through the `homebased` skill. The daemon records the task lifecycle and sends a `HOMEBASED_EVENT` when the run ends. Use Claude Code's runtime-managed background execution only when homebased is unavailable.

Do not keep an agent active only to poll. On the next wake, reconcile the status file and process identity before trusting a notification. Treat the delegate's final message as an artifact, not the source of truth.

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

Prefer a new invocation when an independent perspective or clean repair context matters. Choose the number and kind of follow-ups from the task's evidence, risk, and expected value.
