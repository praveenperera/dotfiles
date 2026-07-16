---
name: pr-review-comments
description: Fetch and analyze GitHub PR conversations, review submissions, and thread-aware code feedback using the prc CLI. Use for PR comment summaries, unresolved or outdated thread inspection, requested changes, and code-linked review context. Accepts PR numbers, PR URLs, or owner/repo format.
---

# PR Review Comments

Load GitHub pull request review context with `prc` so the agent has compact conversation comments, review submissions, and grouped code-review threads with resolution and outdated state. Route implementation work to `../gh-address-comments/SKILL.md`.

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

3. For code feedback, use `--code-only` to omit conversation comments and review submissions:

```bash
prc OWNER/REPO 123 --compact --code-only
```

4. For current unresolved threads, add `--unresolved-only`:

```bash
prc OWNER/REPO 123 --compact --code-only --unresolved-only
```

5. Summarize actionable feedback with thread state, author, comment body, and file or line references. Treat unresolved outdated threads as requiring human triage rather than silently discarding them.

## Limits

- do not use a GitHub token flag; rely on the configured `gh` auth
- `prc` is read-only; it does not reply to or resolve review threads
- for implementing selected fixes or performing GitHub writes, load `../gh-address-comments/SKILL.md`

## Output

Report the PR target, the `prc` command used, a concise list of actionable threads or comments with state and code anchors, and any outdated or ambiguous feedback requiring human judgment.
