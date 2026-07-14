---
name: github
description: Read-only triage and routing for GitHub repositories, pull requests, issues, patches, and review comments through the connected GitHub app, local git, and compact prc exports. Use for general GitHub orientation or summaries before choosing a mutation, review-fix, CI, or publishing workflow.
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
- metadata, patch, requested changes, labels, checks summary, and flat comments
- actionable review feedback and its file or line context
- whether the request concerns review fixes, failing CI, local commits, or
  publishing

Do not infer unresolved-thread state from flat comments. Do not claim that a
comment is current merely because it references a line in an older diff.

## Compact review-comment export

When `prc` is installed and a compact flat export is useful, run one of:

```bash
prc 123 --compact
prc https://github.com/OWNER/REPO/pull/123 --compact
prc OWNER/REPO 123 --compact
```

Use `--code-only` when a large export needs filtering to code-referenced
comments. Treat this output as evidence about comment text, author, chronology,
and recorded code references only. `prc` cannot determine review-thread
resolution or reliably establish whether a comment is outdated against the
current diff. Route any task that depends on those states to
`../gh-address-comments/SKILL.md`, which performs thread-aware inspection.

## Route

- Unresolved threads, requested changes, inline feedback, or implementing
  review fixes: `../gh-address-comments/SKILL.md`
- Failing GitHub Actions checks or log diagnosis: `../gh-fix-ci/SKILL.md`
- Focused local staging or commits only: `../git-commit/SKILL.md`
- Commit, push, and open a draft pull request: `../yeet/SKILL.md`

Do not perform a routed workflow under this skill. Announce the specialist and
load its instructions before continuing.

## Output

Report the target inspected, evidence sources actually used, concise current
state, actionable items, uncertainty about thread or outdated state, and the
recommended specialist workflow. If the user requested a write action, explain
that this entrypoint only triages and route it rather than treating the request
as write authorization here.
