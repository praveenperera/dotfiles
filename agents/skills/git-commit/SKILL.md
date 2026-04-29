---
name: git-commit
description: Prepare focused git commits using Praveen Perera's commit workflow and message rules. Use when the user asks Codex to create a commit, split outstanding changes into logical commits, draft a commit message, stage changes, separate unrelated working-tree edits, or verify a commit is ready.
---

# Git Commit

Use this skill to create small, intentional commits from the current working
tree. When outstanding changes contain unrelated work, split them into logical
commits instead of bundling everything together.

## Workflow

1. Inspect the working tree before staging:
   - `git status --short`
   - `git diff --stat`
   - `git diff`
2. Group outstanding changes by logical purpose.
3. Identify the exact files and hunks that belong to the next commit.
4. Leave changes outside the current logical commit unstaged.
5. Stage only intended changes, using hunk staging when needed.
6. Re-check the staged diff before committing:
   - `git diff --cached --stat`
   - `git diff --cached`
7. Write the commit message from the staged diff, not from assumptions.
8. Commit non-interactively with an explicit message.
9. Repeat for each remaining logical group the user wants committed.
10. Report each commit hash and any remaining unstaged changes.

If the user asks to commit all outstanding work and the diff contains unrelated
changes, create multiple logical commits. Do not squash unrelated fixes,
features, formatting, generated files, or config changes into one commit just
because they are currently outstanding.

If the requested commit scope is unclear, ask before staging or committing. If
the worktree contains unrelated changes that should not be committed yet, mention
them and keep them out of the commit unless the user explicitly includes them.

## Splitting Logical Commits

Treat a logical commit as one reviewable reason for change. Split commits by:

- distinct features or fixes
- unrelated files or subsystems
- mechanical formatting versus behavior changes
- generated output versus source changes
- config/tooling changes versus application code

For mixed files, use hunk staging. If a hunk contains lines from two unrelated
changes, ask before editing files solely to make the split possible.

## Message Rules

Follow `$HOME/.agents/commit-message-guide.md`:

- use imperative mood: `Add feature`, not `Added feature`
- limit the subject to 50 characters
- capitalize the subject in sentence case, not all caps or title case
- do not end the subject with a period
- separate the subject from the body with a blank line
- wrap the body at 72 characters
- explain what changed and why, not how
- include a body only when it adds important context beyond the subject

Keep subjects short, capitalized, imperative, and without a trailing period.

## Attribution

- use `Praveen Perera` if an author identity is needed
- never add Claude, Codex, or AI co-authors
- never add generated-by notes or tool attribution to the commit message

## Staging Discipline

Prefer precise staging over broad staging. Use file-level staging only when the
whole file belongs to the commit. Use hunk staging for mixed files.

Good commands:

```bash
git add path/to/file
git add -p path/to/file
git commit -m "Add focused commit skill"
```

Avoid commands that accidentally include unrelated work:

```bash
git add .
git add -A
```

Only use broad staging when the user explicitly wants every current change
committed and the status output has been reviewed.
