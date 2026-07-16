---
name: github
description: Read-only triage and routing for GitHub repositories, pull requests, issues, patches, and review comments through the connected GitHub app and local git. Use for general GitHub orientation or summaries before choosing a mutation, review-fix, CI, or publishing workflow.
---

# GitHub

Orient the repository, issue, or pull request and route work to the narrowest
specialist workflow. This skill is read-only: do not comment, react, label,
resolve threads, create or edit issues or pull requests, change local files,
commit, push, or invoke mutating API operations.

## Resolve context

1. Prefer a repository, issue, pull-request number, or URL supplied by the user.
2. For “this branch” or “the current PR,” inspect the local repository and
   branch, then use `gh` only as needed to identify its pull request.
3. If the target remains ambiguous after read-only inspection, ask for the
   repository or item identifier.
4. Prefer the connected GitHub app for structured repository, issue, pull
   request, patch, and flat-comment reads. Use local `git` for checkout context
   and non-mutating `gh` commands only for gaps in connector coverage.

## Triage

Gather only the evidence needed to summarize the current state and classify the
next action:

- repository, issue, or pull-request orientation
- metadata, patch, requested changes, labels, and checks summary
- thread-aware review feedback, resolution state, and file or line context via `prc`
- whether the request concerns review fixes, failing CI, local commits, or
  publishing

Do not infer unresolved-thread state from flat comments. Do not claim that a
comment is current merely because it references a line in an older diff.

## Route

- Thread-aware PR review export via `prc`: `../pr-review-comments/SKILL.md`
- Unresolved threads, requested changes, inline feedback, or implementing
  review fixes: `../gh-address-comments/SKILL.md`
- Failing GitHub Actions checks or log diagnosis: `../gh-fix-ci/SKILL.md`
- Focused local staging or commits only: `../git-commit/SKILL.md`
- Commit, push, and open a draft pull request: `../yeet/SKILL.md`

Do not perform a routed workflow under this skill. Announce the specialist and
load its instructions before continuing.

## Output

Report the target inspected, evidence sources actually used, concise current
state, actionable items, any uncertainty remaining after thread-aware reads,
and the recommended specialist workflow. If the user requested a write action, explain
that this entrypoint only triages and route it rather than treating the request
as write authorization here.
