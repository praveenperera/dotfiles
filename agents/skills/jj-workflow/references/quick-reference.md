# jj Quick Reference

Inspect command help for the installed jj version when behavior matters. Commands marked as mutations require matching user scope.

## Inspection

| Command | Purpose |
|---|---|
| `jj root` | Find the repository root |
| `jj status` | Snapshot and show working-copy status |
| `jj diff --stat` | Summarize working-copy changes |
| `jj log -r 'trunk() \| trunk()..@ \| bookmarks()'` | Inspect trunk, local work, and bookmarks |
| `jj bookmark list --all-remotes` | Compare local and remote bookmarks |
| `jj git remote list` | List configured Git remotes |
| `jj log -r 'trunk()..@ & conflicts()'` | Find conflicted local changes |
| `jj resolve --list -r X` | List conflicted files in X |
| `jj file show <path> -r X` | Show a file at X |

## Creating and editing

| Command | Effect |
|---|---|
| `jj new X` | Create a new working-copy commit on X |
| `jj new A B` | Create a merge working-copy commit on A and B |
| `jj describe -m "message"` | Set the working-copy description |
| `jj commit -m "message"` | Describe the working copy and create a new one |
| `jj edit X` | Make X the working-copy revision |

`jj edit X` rewrites X as files change. Save the previous working-copy change ID before editing an older change and return with `jj edit <saved-id>`; a bare `jj new` creates a child of X rather than returning to the prior stack tip.

## Splitting and squashing

| Command | Effect |
|---|---|
| `jj split` | Interactively split `@` |
| `jj split -m "message" <fileset>` | Put selected paths in the first commit without opening a message editor |
| `jj split -r X` | Split X and rebase descendants |
| `jj squash --from X --into Y -u` | Move X's changes into Y and retain Y's description |
| `jj squash -m "message"` | Squash `@` into its parent with an explicit description |

Preview `jju sh` hunk indices before using `--hunks`, `--lines`, or `--pattern`; selections can change after each split.

## Rebasing

`-o` means `--onto`.

| Command | Effect |
|---|---|
| `jj rebase -r X -o Y` | Move only X onto Y and close the old graph hole |
| `jj rebase -s X -o Y` | Move X and all descendants onto Y |
| `jj rebase -b X -o Y` | Move the branch containing X relative to Y |
| `jj rebase -r X -A Y` | Insert X after Y |
| `jj rebase -r X -B Y` | Insert X before Y |

Prefer stable change IDs over positional revisions across multiple rewrites. Use `trunk()` as the destination instead of assuming a branch or remote name.

## Bookmarks and remotes

| Command | Effect |
|---|---|
| `jj bookmark create X -r Y` | Create X at Y |
| `jj bookmark set X -r Y` | Move X to Y |
| `jj bookmark delete X` | Delete local X; remote deletion is separate |
| `jj bookmark list --all-remotes` | Inspect local and remote targets |
| `jj git fetch` | Refresh remote Git state |
| `jj git push --dry-run --bookmark X` | Preview publication of X |
| `jj git push --bookmark X` | Publish only X |

Do not fetch, mutate bookmarks, or publish without matching authorization. Avoid bare `jj git push` when only specific bookmarks are in scope.

## Undo and recovery

| Command | Effect |
|---|---|
| `jj op log` | Inspect local operation history |
| `jj undo` | Undo the last local operation |
| `jj op restore X` | Restore repository state to operation X |

These commands do not reverse a remote push or a hosting-service action.

## Revsets

| Expression | Meaning |
|---|---|
| `@` | Working-copy commit |
| `@-` | Parent of `@` |
| `trunk()` | Configured default remote's trunk head |
| `X..Y` | Ancestors of Y that are not ancestors of X |
| `::X` | X and its ancestors |
| `X::` | X and its descendants |
| `roots(X)` | Roots of set X |
| `heads(X)` | Heads of set X |
| `X \| Y` | Union |
| `X & Y` | Intersection |
| `X ~ Y` | Set difference |
| `conflicts()` | Revisions containing conflicts |
| `bookmarks()` | Revisions with local bookmarks |
| `empty()` | Empty revisions |

## Verification pattern

```bash
jj log -r 'trunk() | <affected-revset>'
jj diff -r <change-id>
jj log -r '(<affected-revset>) & conflicts()'
jj bookmark list --all-remotes
jj status
```

No output from the conflict query means the selected revisions are conflict-free. Still run the project's relevant checks at every intended PR tip.
