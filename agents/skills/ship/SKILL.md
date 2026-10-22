---
name: ship
description: Finish the current work with the repository's configured sequence (commit, push, PR, Sol review, Greptile 5/5, merge, deploy, clean up), adjusted by short words such as `+sol`, `+settle` to settle the T3 thread afterward, `-deploy`, `minor`, `confirm` to wait for a go-ahead, or `?` for a status check. Use only when the user invokes /ship or $ship.
argument-hint: "[? | confirm | +step | -step | only step... | high | minor | major]"
disable-model-invocation: true
---

# Ship

`/ship` carries the current work to the finish this repository uses. The sequence comes from [repos.toml](repos.toml), not from interpretation: never add, drop, or reorder a step the script did not put in the plan.

## Resolve the plan

Run the resolver from the repository root with the user's words passed as one argument. `<task>` is the branch name or a short slug of the work:

```sh
python3 -I ~/.agents/skills/ship/scripts/ship_plan.py --task <task> -- "$ARGUMENTS"
```

The same run saves the plan as `_scratch/ship/<task>/plan.json` and prints a `saved:` line, so do not run the resolver again for `--json`. Chain the read-only checks you need for the plan, such as `git status --short` and `git diff --stat`, onto that command.

- On a nonzero exit, report the error line and stop. Do not guess what the word meant.
- Otherwise show the `ship:` and `plan:` lines, then the concrete actions each step will take in this checkout: the branch, the files or hunks to commit, the PR target, the deploy commands with their placeholders filled, and what clean will remove. Then start in the same turn; the invocation is the go-ahead. Inside `/run-auto`, also log the plan in the decision log.
- Stop and wait for the go-ahead instead when the output has `confirm: wait for go-ahead`, from the `confirm` word, or when the plan is not obvious for the work, as in the cases below. Put every question in the same message as the plan, and start once they are answered.
- Ask instead of assuming when the repository is new to the table, which the `ship:` line shows as `default fallback` or `external fallback`. Ask which steps it should use and what deploy means there, and offer to add its entry to [repos.toml](repos.toml) before running.
- Also ask when anything is unclear: which changes belong to this task, such as unrelated edits or unpushed commits you cannot attribute; a placeholder the notes don't settle; or a plan that looks wrong for the work, such as a deploy for a docs-only change.
- Record finished steps only before the turn ends to wait for a wake-up, such as a PR watch, a `Monitor`, or a homebased task: chain `python3 -I ~/.agents/skills/ship/scripts/ship_plan.py --task <task> --done <step>...` onto the last command before the wait, and resume from the saved plan's `done` list when the thread wakes. A run that finishes in one turn records nothing.

Words: a bare or `+` step adds it, `-step` removes it, `only` replaces the default with the steps that follow, `high` or `xhigh` sets the Sol effort, `patch`, `minor`, or `major` sets the bump, `confirm` waits for the go-ahead before starting, and `?` reports status without changing anything. The resolver also accepts dictation variants such as "soul", "reptile", and "XI", and `s` or `set` for `settle`.

## Authorization

Invoking `/ship` is explicit authorization for every step in its resolved plan, including merge, deploy, and publish, once any open questions are answered and, with `confirm`, the user gives the go-ahead. It holds inside `/run-auto`. It authorizes nothing else: no force-push, history rewrite, or `--no-verify`; no step the plan lacks; and no deletion of anything this run did not create.

## Status mode

When the plan's mode is `status`, change nothing and ask nothing. Report what is unfinished:

- unticked items in this task's `_scratch/*/TASKS.md`
- uncommitted changes in this task's files and unpushed commits
- the PR's state, Greptile score, unresolved threads, and CI
- whether the pushed commit was deployed, marking it unverified when nothing in this session shows it

End with the exact `/ship` command that would finish the work, or say that nothing is left.

## Track what this run creates

Append one line per created item to `_scratch/ship/<task>/created.txt` as `<kind> <value>`, in the same command that creates the item, such as `git switch -c <branch> && echo "branch <branch>" >> _scratch/ship/<task>/created.txt`, not as a separate call. Kinds: `worktree`, `branch`, `remote-branch`, `scratch`, `tmp`, `process`, `monitor`, `homebased-task`, `container`, `image`, `preview`. The clean step deletes only what this manifest lists. Add items this task created before `/ship` was invoked, such as its worktree, branch, and `_scratch/<task>/`, when the conversation shows that this task created them; append those in the first command of the run.

## Steps

Run the plan's steps in order. When a step fails, stop: report the failure with its output, skip `clean` for worktrees and branches, still stop the processes this run started, and do not settle.

When the plan has `pr`, never push the default branch. If HEAD is on it, create the task branch before the first commit or push with `git switch -c <branch>`, so any unpushed local commits move with it; when those commits are all this task's, point the local default branch back at its remote with `git branch -f <default> origin/<default>`.

### sol

Review the change against the merge base of the default branch, including uncommitted work, with GPT-6.1 Sol at the plan's effort in a fresh read-only session.

- In a T3 thread, use `delegate_task` with the Codex provider and model `gpt-6.1-sol` from `orchestrator_capabilities`.
- Elsewhere, run the Codex review command from [the review-fix-loop provider reference](../review-fix-loop/references/providers.md#codex-review) with the plan's effort, submitting it through [homebased](../homebased/SKILL.md) when it will run long.

Fix the actionable findings, which are correctness, security, data-loss, and maintainability problems; drop style preferences and speculation. Run the repository's checks, then run one fresh review of the fixed code. When that second review still has actionable findings, stop and report them.

### commit

Follow [git-commit](../git-commit/SKILL.md). Stage only this task's files or hunks. Add a branch created for the PR to the manifest. Skip the step when there is nothing to commit.

### push

Push the branch with tracking. When the remote has moved, integrate with `git pull --rebase` only while every local commit is unpushed, and stop on a conflict.

### pr

Open the PR against the plan's `pr` target, as a draft when the plan says draft. For a fork, use `--repo <target> --head <owner>:<branch>`. Write the body as Markdown prose: what changed, why, the root cause for a fix, and how it was verified. In a T3 thread, call `link_pull_request` with its URL.

### greptile

Pass when all of these hold for the PR's current head commit:

- Greptile's latest summary comment for that commit shows `Confidence Score: 5/5`
- `prc <pr> --compact --code-only --unresolved-only` lists no threads
- required CI checks pass

Until it passes, address each comment with [gh-address-comments](../gh-address-comments/SKILL.md), reply on its thread, push, and comment `@greptileai review` unless a review of the new head is already running. Allow at most 3 such rounds; then stop and report what remains.

To wait, record the finished steps with `--done`, then use `watch_pull_request` in a T3 thread and end the turn; resume from `plan.json` when T3 wakes the thread. Elsewhere, poll every 20 to 30 seconds for at most 20 minutes.

### merge

Merge only after the greptile step passed, when it is in the plan, and required CI is green: `gh pr merge <n> --squash --match-head-commit <sha>`. Leave out `--delete-branch`, which fails in a worktree; clean deletes the branches. Then run `git pull --ff-only` in the main checkout, the first entry of `git worktree list`, and stop when it is not on the default branch or the pull fails.

### deploy and publish

Run the plan's commands in order from the repository root: the main checkout after a merge, otherwise the current checkout. Fill each remaining placeholder, such as `<item>`, `<next>`, `<site>`, or `<version>`, from the plan's notes and the repository; never run a command with a placeholder left in it. Run a long command, such as a release watch, a fleet update, or a test suite, as a [homebased](../homebased/SKILL.md) task workload; in a T3 Claude thread, arm `Monitor` with `homebased task wait <id>` so the thread wakes once, when it ends. `watch_run.sh` prints a single line at the end, so it is safe to wait on directly. Verify the result the commands promise, such as a healthy service, a finished release run, or the new installed version.

### clean

Remove each manifest item that is still present, then the manifest:

- **Worktree and branch:** remove a worktree with `git worktree remove` only when it has no uncommitted or untracked changes, and leave one you are standing in by changing to the main checkout first. Delete a branch with `git branch -d`, which succeeds only when its commits are on the default branch. After a squash merge, the branch counts as merged only when `gh pr view <n> --json state,headRefOid` shows `MERGED` and `headRefOid` equals the local branch tip; then `git branch -D` is allowed, and `git push origin --delete <branch>` removes the remote branch. Never use `--force` or `-D` in any other case. While the PR is open, keep both.
- **Scratch:** before deleting `_scratch/<task>/` and `_scratch/ship/<task>/`, move anything the final report links to into `~/code/research/ship/<repo>/<task>/`.
- **Processes and services:** stop dev servers, Monitors, background shells, and homebased tasks this run started that are still running, and close the previews it opened.
- **Containers and images:** remove the ones built only for testing.

Never touch stashes, other agents' worktrees or scratch, or anything not in the manifest. List anything you kept, with the reason.

### settle

Opt-in: no repository includes it by default, so it runs only when the user types `+settle` (or `+s` or `+set`). Only in a T3 thread, and only when every earlier step succeeded and nothing needs the user, call `t3_thread_organize` with `action: "settle"` and no `threadId` as the last tool call of the turn. T3 applies it when the turn ends. Outside T3, report that settle was skipped.

## Report

List anything that needs the user first, then: the plan line, each step's result with its evidence (commit, PR URL, Greptile score, release or deploy check), what clean removed and kept, and where moved notes went.

## Change a repository's default

Edit [repos.toml](repos.toml), then run `python3 -I ~/.agents/skills/ship/scripts/ship_plan.py --check` and the tests in `tests/`. When `/ship` runs in a repository without an entry, ask for its steps and add the entry before running.
