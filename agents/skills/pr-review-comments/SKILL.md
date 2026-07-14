---
name: pr-review-comments
description: Fetch and analyze GitHub PR review comments and code-level feedback using the prc CLI. Accepts PR numbers (auto-detects repo), PR URLs, or owner/repo format.
disable-model-invocation: true
---

# PR Review Comments

Load GitHub pull request review comments with `prc` so the agent has compact, code-linked feedback context. Prefer this skill for flat comment export; route thread-aware resolution and fix implementation to `../gh-address-comments/SKILL.md`.

## Workflow

1. Resolve the PR target from the user request:
   - PR number only (auto-detects repo from git remote when in a git repo)
   - full PR URL
   - `owner/repo` plus PR number
2. Fetch comments with `--compact`:

```bash
prc 123 --compact
prc https://github.com/OWNER/REPO/pull/123 --compact
prc OWNER/REPO 123 --compact
```

3. If the export is large or noisy, re-run with `--code-only` to keep only comments that include code references:

```bash
prc OWNER/REPO 123 --compact --code-only
```

4. Summarize actionable feedback with author, comment body, and file/line references. Use the context to answer the user or prepare fixes.

## Limits

- `prc` is a flat export of comment text, author, chronology, and recorded code references
- it cannot determine review-thread resolution or reliably decide whether a comment is outdated against the current diff
- do not use a GitHub token flag; rely on the configured `gh` auth
- for unresolved threads, requested changes, or implementing selected fixes, load `../gh-address-comments/SKILL.md`

## Output

Report the PR target, the `prc` command used, a concise list of actionable comments with code anchors when present, and any uncertainty that requires thread-aware inspection.
