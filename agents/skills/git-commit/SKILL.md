---
name: git-commit
description: Prepare focused git commits using Praveen Perera's commit workflow and message rules. Use when the user asks Codex to create a commit, split outstanding changes into logical commits, draft a commit message, stage changes, separate unrelated working-tree edits, or verify a commit is ready.
---

# Git Commit

Prepare only the focused local commit work the user requested. A request to
draft a message or verify readiness is read-only; stage or commit only when the
user explicitly asks for it. Do not push, open a pull request, or include
unrelated changes.

## Workflow

1. Read `$HOME/.agents/commit-message-guide.md` before drafting a message. If it
   is unavailable, stop and ask the user for the guide instead of inventing
   replacement rules.
2. Inspect `git status --short`, `git diff --stat`, and `git diff` before
   staging. Also inspect `git diff --cached` because the index may already
   contain user changes.
3. Partition the requested work into reviewable reasons for change. Keep
   distinct fixes, features, generated output, formatting, and tooling changes
   separate when they do not belong to one reason.
4. For an authorized commit, stage only the intended files or hunks. Preserve unrelated staged and
   unstaged changes. Do not use `git add .` or `git add -A` unless the user
   explicitly requests every reviewed change.
5. Re-read `git diff --cached --stat` and `git diff --cached`. Confirm every
   staged hunk belongs to the next commit and no requested hunk is missing.
6. Draft the message from the staged diff and the external guide. Give every
   commit a subject that uniquely identifies its change; avoid generic subjects
   such as `Address feedback`, `Fix comments`, or `Update code`.
7. When committing is authorized, commit non-interactively with an explicit message. Do not bypass hooks,
   signing, verification, or repository policy. If the commit fails, report the
   exact failure and leave the worktree and index intact.
8. Repeat only for the remaining logical groups in the user's requested scope.
9. Report each commit hash and subject, verification performed by hooks, and
   remaining staged or unstaged changes.

## Scope and safety

- Ask before staging when the requested scope cannot be separated confidently.
- Use hunk staging for mixed files. If a single hunk combines unrelated work,
  ask before editing solely to make it separable.
- Never amend, rebase, reset, restore, force, skip hooks, change author identity,
  or alter signing configuration unless the user explicitly requests that
  operation.
- Use `Praveen Perera` only when an author identity is required. Never add AI
  co-authors, generated-by notes, or tool attribution.
- A request to create a commit authorizes local staging and committing only.
  Drafting or readiness checks do not. Route a later
  push or pull-request request to the appropriate publishing workflow.
