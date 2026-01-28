# jj Quick Reference

## Essential Commands

| Command | Purpose |
|---------|---------|
| `jj status` | Working copy status + trigger snapshot |
| `jj diff` | Changes in working copy |
| `jj log` | Commit graph |
| `jj log -r 'master..@'` | Commits since master |
| `jj file show <path> -r X` | Show file contents at revision X |

## Creating & Editing

| Command | Purpose |
|---------|---------|
| `jj new` | Create new empty commit on @ |
| `jj new X` | Create commit based on X |
| `jj new A B` | Create merge commit |
| `jj describe -m "msg"` | Set commit message |
| `jj edit X` | Make X the working copy (edit older commit) |
| `jj commit -m "msg"` | Describe @ and create new |

## Splitting & Squashing

| Command | Purpose |
|---------|---------|
| `jj split` | Interactive split |
| `jj split "glob:pattern"` | Split by file pattern |
| `jj split -r X` | Split older commit |
| `jj squash` | Squash @ into parent |
| `jj squash --into X` | Move @ changes into X |
| `jj squash -u` | Squash, keep destination message (no editor) |

### Non-Interactive Hunk Selection (`jju sh`)

| Command | Purpose |
|---------|---------|
| `jju sh --preview` | Show hunks with indices |
| `jju sh --preview --file F` | Preview hunks in specific file |
| `jju sh -m "msg" --hunks 0,2` | Split by hunk index |
| `jju sh -m "msg" --lines 10-50` | Split by line range |
| `jju sh -m "msg" --pattern "regex"` | Split by pattern match |
| `jju sh -m "msg" --invert --hunks 0` | Exclude matched hunks |
| `jju sh --dry-run -m "msg" --hunks 0` | Preview without committing |

## Rebasing

`-o` is short for `--onto` (NOT `--to`)

| Command | Purpose |
|---------|---------|
| `jj rebase -r X -o Y` | Move commit X onto Y |
| `jj rebase -s X -o Y` | Move X and descendants onto Y |
| `jj rebase -o master@origin` | Rebase current work onto latest master |

## Bookmarks

| Command | Purpose |
|---------|---------|
| `jj bookmark create X -r Y` | Create bookmark at commit |
| `jj bookmark set X -r Y` | Move bookmark |
| `jj bookmark delete X` | Delete bookmark |
| `jj bookmark list` | List all bookmarks |

## Remote Operations

| Command | Purpose |
|---------|---------|
| `jj git fetch` | Fetch from remote |
| `jj git push` | Push all tracking bookmarks |
| `jj git push --bookmark X` | Push specific bookmark |

## Undo & Recovery

| Command | Purpose |
|---------|---------|
| `jj undo` | Undo last operation |
| `jj op log` | Operation history |
| `jj op restore X` | Restore to operation X |

## Conflicts

| Command | Purpose |
|---------|---------|
| `jj resolve` | Open merge tool |
| `jj resolve --list` | List conflicted files |

---

## Revset Expressions

### Symbols

| Expression | Meaning |
|------------|---------|
| `@` | Working copy commit |
| `@-` | Parent of @ |
| `@--` | Grandparent |
| `master` | Main bookmark |
| `master@origin` | Remote master |

### Set Operators

| Operator | Meaning | Example |
|----------|---------|---------|
| `x \| y` | Union (in x OR y) | `main \| feature` |
| `x & y` | Intersection (in x AND y) | `author(me) & files(src)` |
| `x ~ y` | Difference (in x but NOT y) | `master..@ ~ @` |
| `~x` | Complement (NOT in x) | `~empty()` |

### Range Operators

| Operator | Meaning |
|----------|---------|
| `x..y` | Ancestors of y that aren't ancestors of x |
| `::x` | All ancestors of x (including x) |
| `x::` | All descendants of x (including x) |
| `x-` | Parents of x |
| `x+` | Children of x |

### Functions

| Function | Meaning |
|----------|---------|
| `roots(X)` | Root commits of set X |
| `heads(X)` | Head commits of set X |
| `ancestors(X)` | All ancestors including X |
| `descendants(X)` | All descendants including X |
| `author(pattern)` | Commits by author |
| `description(pattern)` | Commits with message matching pattern |
| `files(pattern)` | Commits touching files |
| `conflicts()` | Commits with merge conflicts |
| `empty()` | Empty commits |
| `bookmarks()` | Commits with bookmarks |

---

## Common Patterns

```bash
# see what you'll push
jj log -r 'master..@-'

# get change IDs for scripting
jj log -r 'master..@-' --no-graph -T 'change_id.short() ++ " " ++ description.first_line() ++ "\n"'

# update and rebase in one go
jj git fetch && jj rebase -o master@origin
```
