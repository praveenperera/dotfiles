# Codex CLI delegation reference

## Preflight

Confirm the installed CLI, authentication, model availability, and exact flags instead of relying on memory:

```sh
command -v codex
codex --version
codex login status
codex exec --help
codex debug models | rg 'gpt-5\.6-(sol|luna)'
```

Do not modify login or global configuration automatically. If authentication or the requested model is unavailable, report the exact failure.

## Create evidence artifacts

Run from the repository root:

```sh
delegate_run_id="$(date +%Y%m%d-%H%M%S)-$$"
delegate_dir="_scratch/subagent-workflow/$delegate_run_id"
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

Default to Sol with high reasoning. Use Luna only for a repeated or high-volume exact mechanical read:

```sh
codex --ask-for-approval never exec \
  --cd "$PWD" \
  --ephemeral \
  --model gpt-5.6-sol \
  --config 'model_reasoning_effort="high"' \
  --sandbox read-only \
  --output-last-message "$delegate_dir/raw/final.md" \
  - \
  < "$delegate_dir/prompts/task.md" \
  > "$delegate_dir/raw/stdout.txt" \
  2> "$delegate_dir/raw/stderr.txt"
delegate_exit_status=$?
printf '%s\n' "$delegate_exit_status" > "$delegate_dir/raw/exit-status.txt"
```

For a single easy, tightly scoped task, keep Sol and change the reasoning effort to `low`. For repeated or high-volume mechanical work, change the model to `gpt-5.6-luna` and use `low` reasoning unless the exact procedure requires several tool-driven steps.

## Run a fresh implementation delegate

Use workspace-write only after assigning an exact owned scope:

```sh
codex --ask-for-approval never exec \
  --cd "$PWD" \
  --ephemeral \
  --model gpt-5.6-sol \
  --config 'model_reasoning_effort="high"' \
  --sandbox workspace-write \
  --output-last-message "$delegate_dir/raw/final.md" \
  - \
  < "$delegate_dir/prompts/task.md" \
  > "$delegate_dir/raw/stdout.txt" \
  2> "$delegate_dir/raw/stderr.txt"
delegate_exit_status=$?
printf '%s\n' "$delegate_exit_status" > "$delegate_dir/raw/exit-status.txt"
```

Do not use `--dangerously-bypass-approvals-and-sandbox`. Add a writable directory with `--add-dir` only when the owned scope explicitly requires it. Add `--json` only when the event stream is useful; `--output-last-message` already captures the final natural-language report.

Use `codex exec review` or `codex review` only when their target flags match the requested review. A self-contained `codex exec` prompt is more flexible for repository analysis and custom review contracts.

## Handle duration without polling

For a run expected to exceed two minutes, use Claude Code's runtime-managed background execution or a detached supervisor that records a durable job ID, start time, deadline, process identity, terminal status, and exit code in the run directory. Register the runtime completion event when available.

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

Inspect exit status, stdout, stderr, final message, baseline, and postflight artifacts. A read-only mutation is a failed pass. For implementation, reject changes outside owned scope. Independently inspect the diff and run repository verification before reporting completion.

## Choose follow-ups deliberately

Prefer a new ephemeral invocation when an independent perspective or clean repair context matters. Choose the number and kind of follow-ups from the task's evidence, risk, and expected value.

Use `codex exec resume <session>` only when continuity is essential and the first run was intentionally persisted without `--ephemeral`.

## Current command sources

- [Codex CLI developer commands](https://learn.chatgpt.com/docs/developer-commands?surface=cli#codex-exec)
- [Codex non-interactive mode](https://learn.chatgpt.com/docs/noninteractive)
- the locally installed `codex exec --help` and `codex debug models` output
