---
name: jj-workflow
description: Guide for jj (Jujutsu) version control workflows including splitting changes, creating stacked PRs, independent PRs, and hybrid approaches. Use when working with jj commands, feature branches, or preparing commits for pull requests.
version: "2.0.0"
model: inherit
---

# jj Workflow Skill

## What is jj?

jj (Jujutsu) is a Git-compatible version control system with a simpler mental model for history editing.

**When to use jj:**
- Working on multiple features that touch overlapping files
- Maintaining stacks of dependent PRs
- Frequently reordering, squashing, or splitting commits
- You want to split changes after-the-fact rather than commit perfectly upfront

**Why jj over Git:**
- No staging area complexity - working copy is always a commit
- Edit any commit and descendants auto-rebase
- Conflicts are first-class - defer resolution, operations don't stop
- Every operation is undoable with `jj undo`
- Splitting commits is trivial with `jj split`

---

## Core Mental Model

- **Working copy (@) is always a commit** - no staging area, changes auto-track
- **Bookmarks don't auto-move** - must explicitly `jj bookmark set` (unlike Git branches)
- **Descendants auto-rebase** - edit commit A and B/C/D rebase automatically
- **Change IDs persist** - commit hashes change on rebase, change IDs don't
- **Conflicts are first-class** - can exist in commits; resolve when ready
- **Every operation is undoable** - `jj undo` or `jj op restore`

---

## Which Reference to Read

### Creating PRs from Commits

**Read `references/modes/stacked.md` when:**
- Features genuinely depend on each other (B needs A's code)
- You want PR B to include PR A's changes
- Merge order matters (must merge A before B before C)

**Read `references/modes/independent.md` when:**
- Features are truly separate and don't share code
- PRs can merge in any order
- You don't want to wait for other PRs to merge first

**Read `references/modes/hybrid.md` when:**
- Some features depend on each other, others don't
- Example: A→B are stacked, but C is independent

### Preparing Commits

**Read `references/modes/split.md` when:**
- You have one commit with mixed changes from multiple features
- You need to separate changes into logical commits
- You want to split by file pattern or interactively

### After PR Review

**Read `references/modes/update.md` when:**
- You received review feedback and need to update a commit
- You need to edit an older commit in your stack
- You're confused about `jj edit` vs `jj new`

### After PR Merges

**Read `references/modes/sync.md` when:**
- A PR was merged and you need to update remaining stack
- You want to sync with latest master
- You need to delete merged bookmarks and update PR bases

**Read `references/rebase-after-squash.md` when:**
- GitHub squash-merged your PR (not regular merge)
- You're confused why Git rebase is painful but jj is easy
- You need the specific command for post-squash-merge rebase

### Reference

**Read `references/quick-reference.md` when:**
- You need command syntax or revset expressions

**Read `references/full-guide.md` when:**
- You want comprehensive documentation or deep jj vs Git comparison
- Other references don't answer your question

**See `examples/` folder for:**
- Complete copy-paste workflow scripts (stacked, independent, post-squash-merge)

---

## Quick Start Commands

```bash
# assess current state
jj git fetch
jj log -r 'master..@'
jj status

# get change IDs for commits
jj log -r 'master..@-' --no-graph -T 'change_id.short() ++ " " ++ description.first_line() ++ "\n"'
```

Then read the appropriate reference above based on your goal.

---

## Common Pitfalls

### Bookmark didn't move
jj bookmarks don't auto-move. After changes: `jj bookmark set pr/feature -r @`

### Can't push conflicted commit
jj won't push conflicts. Resolve first: `jj edit <commit>`, fix files, `jj new`, then push.

### Undo doesn't undo push
`jj undo` is local only. To undo a push: `jj bookmark set X -r @~1` then `jj git push`

### File edits not checkpointed
jj snapshots only when you run a command. Run `jj status` before risky work to checkpoint.

### Detached HEAD in Git tools
Normal in colocated mode. jj doesn't track "current branch". Use jj commands, not git checkout.

### Interactive commands hang in automation
Commands like `jj split` and `jj squash` open an editor by default. Use flags to skip:
```bash
jj split -m "feat: description" file1.ts file2.ts
jj squash --from X --into Y -u  # -u keeps destination message
```
