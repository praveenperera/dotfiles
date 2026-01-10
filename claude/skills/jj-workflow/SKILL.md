---
name: jj-workflow
description: Guide for jj (Jujutsu) version control workflows including splitting changes, creating stacked PRs, independent PRs, and hybrid approaches. Use when working with jj commands, feature branches, or preparing commits for pull requests.
version: "1.0.0"
model: inherit
---

# jj (Jujutsu) Workflow Guide

I help with jj version control workflows, especially:
- Splitting mixed changes into logical commits
- Creating stacked PRs (dependent chain)
- Creating independent PRs (each targets main)
- Hybrid approaches (mix of stacked and independent)

## Core Concepts

### Working Copy is Always a Commit
In jj, your working copy state IS a commit. No staging area. Changes auto-track.

### Automatic Descendant Rebasing
When you edit commit X, all descendants automatically rebase on top.

### Bookmarks Don't Auto-Move
Unlike Git branches, jj bookmarks stay fixed. Explicitly move with `jj bookmark set`.

### Conflicts Are First-Class
Conflicts can exist in commits. Operations don't stop. Resolve when ready.

## Workflow Selection

**Stacked PRs** when:
- Features genuinely depend on each other
- You want PR B to include PR A's changes
- Merge order matters (A before B before C)

**Independent PRs** when:
- Features are truly separate
- Can merge in any order
- Don't want to wait for other PRs

**Hybrid** when:
- Some features depend on each other, others don't
- E.g., A→B stacked, C independent

## Key Commands

### Viewing State
```bash
jj log                      # Commit graph
jj log -r 'main..@'         # Commits since main
jj status                   # Working copy status
jj diff                     # Changes in @
```

### Splitting Changes
```bash
jj split                    # Interactive TUI
jj split "glob:src/auth/*"  # By file pattern
jj split -r <change-id>     # Split older commit
jj describe -r @- -m "msg"  # Describe the split-off commit
```

### Making Commits Independent
```bash
# Break B out of stack (main→A→B→C becomes main→A→C, main→B)
jj rebase -r <B-change-id> -d main
```

### Bookmarks
```bash
jj bookmark create pr/name -r <change-id>
jj bookmark set pr/name -r @          # Move bookmark
jj bookmark list                       # List all
```

### Pushing
```bash
jj git push                           # Push all tracking bookmarks
jj git push --bookmark pr/feature     # Push specific
```

### Undoing Mistakes
```bash
jj undo                               # Undo last operation
jj op log                             # Operation history
jj op restore <op-id>                 # Restore to point in time
```

## Creating PRs

### Stacked PRs
```bash
# After: main → A → B → C
jj bookmark create pr/a -r <A>
jj bookmark create pr/b -r <B>
jj bookmark create pr/c -r <C>
jj git push

gh pr create --head pr/a --base main
gh pr create --head pr/b --base pr/a
gh pr create --head pr/c --base pr/b
```

### Independent PRs
```bash
# Transform stack to independent
jj rebase -r <B> -d main
jj rebase -r <C> -d main
# Now: main → A, main → B, main → C

jj bookmark create pr/a -r <A>
jj bookmark create pr/b -r <B>
jj bookmark create pr/c -r <C>
jj git push

gh pr create --head pr/a --base main
gh pr create --head pr/b --base main
gh pr create --head pr/c --base main
```

## Updating After Review

```bash
# Edit an older commit
jj edit <change-id>
# Make changes...
jj new                    # Create new working copy

# Descendants auto-rebase!
jj git push               # Force-push updates
```

## Reference: Full Guide

For comprehensive documentation including:
- Detailed workflow diagrams
- Pitfalls and gotchas
- Git interop details
- Configuration recommendations

See: references/jj-feature-branching-guide.md
