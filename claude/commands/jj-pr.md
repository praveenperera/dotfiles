---
description: Create PRs from jj commits - stacked, independent, or hybrid mode
argument-hint: <mode> [stacked|independent|hybrid|split|update|sync]
---

# jj PR Workflow Assistant

You are helping the user manage jj (Jujutsu) commits and GitHub PRs.

**Mode requested: $ARGUMENTS**

---

## Core Concepts (Always Remember)

- **Working copy (@) is always a commit** - no staging area
- **Change IDs persist across rebases** - use them to reference commits
- **Bookmarks don't auto-move** - must explicitly `jj bookmark set`
- **Descendants auto-rebase** - editing commit A automatically rebases B, C, D
- **Conflicts are first-class** - can exist in commits; resolve when ready
- **Force-push is automatic** - `jj git push` handles rewritten commits safely

---

## Step 1: Assess Current State

First, run these commands:

```bash
jj git fetch                    # Get latest from remote
jj log -r 'master..@'             # See commits since master
jj status                       # Working copy status
```

Check for:
- Number of commits between master and @
- Which commits have descriptions vs are empty
- Any commits with conflicts (shown in log)
- Whether master@origin has new commits

---

## Step 2: Execute Based on Mode

### MODE: "stacked" (or empty/default)

**Stacked PRs** - commits depend on each other, PRs target previous PR's branch.

```
master ─── A ─── B ─── C
         │     │     │
         │     │     └── PR #3 (base: B)
         │     └──────── PR #2 (base: A)
         └────────────── PR #1 (base: master)
```

**Steps:**

1. **Ensure up-to-date with master:**
   ```bash
   jj git fetch
   jj rebase -d master@origin  # If needed
   ```

2. **If work needs splitting, split first:**
   ```bash
   jj split "glob:src/feature-a/*"
   jj describe -r @- -m "feat: feature A"
   # Repeat for each feature...
   ```

3. **Identify commits (get change IDs):**
   ```bash
   jj log -r 'master..@-' --no-graph -T 'change_id.short() ++ " " ++ description.first_line() ++ "\n"'
   ```

4. **Create bookmarks at each commit:**
   ```bash
   jj bookmark create pr/<feature-a> -r <change-id-A>
   jj bookmark create pr/<feature-b> -r <change-id-B>
   jj bookmark create pr/<feature-c> -r <change-id-C>
   ```

5. **Push all bookmarks:**
   ```bash
   jj git push
   ```

6. **Create PRs with correct base targeting:**
   ```bash
   gh pr create --head pr/<feature-a> --base master --title "<title A>"
   gh pr create --head pr/<feature-b> --base pr/<feature-a> --title "<title B>"
   gh pr create --head pr/<feature-c> --base pr/<feature-b> --title "<title C>"
   ```

---

### MODE: "independent"

**Independent PRs** - each commit rebased onto master, PRs all target master.

```
         ┌── A ──────── PR #1 (base: master)
         │
master ────┼── B ──────── PR #2 (base: master)
         │
         └── C ──────── PR #3 (base: master)
```

**Steps:**

1. **Start with commits (may be stacked initially):**
   ```bash
   jj log -r 'master..@-' --no-graph -T 'change_id.short() ++ " " ++ description.first_line() ++ "\n"'
   ```

2. **Rebase each commit onto master (except first, which is already on master):**
   ```bash
   jj rebase -r <change-id-B> -d master
   jj rebase -r <change-id-C> -d master
   ```

3. **Create bookmarks:**
   ```bash
   jj bookmark create pr/<feature-a> -r <change-id-A>
   jj bookmark create pr/<feature-b> -r <change-id-B>
   jj bookmark create pr/<feature-c> -r <change-id-C>
   ```

4. **Push and create PRs (all target master):**
   ```bash
   jj git push
   gh pr create --head pr/<feature-a> --base master --title "<title>"
   gh pr create --head pr/<feature-b> --base master --title "<title>"
   gh pr create --head pr/<feature-c> --base master --title "<title>"
   ```

5. **Optional - create dev merge to work on all together:**
   ```bash
   jj new pr/<feature-a> pr/<feature-b> pr/<feature-c> -m "dev: combined"
   # Now @ contains all features merged for testing
   ```

---

### MODE: "hybrid"

**Hybrid** - some commits stacked, some independent.

Example: A→B stacked, C independent, D depends on B

```
         ┌── A ─── B ─── D   (stacked: A→B→D)
master ────┤
         └── C               (independent)
```

**Steps:**

1. **Identify change IDs:**
   ```bash
   jj log -r 'master..@-' --no-graph -T 'change_id.short() ++ " " ++ description.first_line() ++ "\n"'
   ```

2. **Ask user which should be stacked vs independent**

3. **Rebase independent commits onto master:**
   ```bash
   jj rebase -r <change-id-C> -d master
   ```

4. **Rebase dependent commits to correct parent (if needed):**
   ```bash
   jj rebase -r <change-id-D> -d <change-id-B>
   ```

5. **Create bookmarks and push:**
   ```bash
   jj bookmark create pr/feat-a -r <A>
   jj bookmark create pr/feat-b -r <B>
   jj bookmark create pr/feat-c -r <C>
   jj bookmark create pr/feat-d -r <D>
   jj git push
   ```

6. **Create PRs with correct bases:**
   ```bash
   gh pr create --head pr/feat-a --base master --title "A"
   gh pr create --head pr/feat-b --base pr/feat-a --title "B"
   gh pr create --head pr/feat-c --base master --title "C (independent)"
   gh pr create --head pr/feat-d --base pr/feat-b --title "D"
   ```

---

### MODE: "split"

**Split mixed work** into logical commits.

**Mental model:** You select what goes into the FIRST commit; remasterder stays in SECOND.

**Steps:**

1. **Show current changes:**
   ```bash
   jj diff
   jj status
   ```

2. **Help identify logical groupings** (by file pattern, feature area, etc.)

3. **Split interactively or by pattern:**
   ```bash
   # Interactive TUI:
   jj split

   # By file pattern:
   jj split "glob:src/auth/*"

   # Specific files:
   jj split path/to/file1.ts path/to/file2.ts
   ```

4. **Describe the split-off commit:**
   ```bash
   jj describe -r @- -m "feat: description of first feature"
   ```

5. **Repeat until all changes are logically separated**

6. **To split an older commit:**
   ```bash
   jj split -r <change-id>
   # Descendants auto-rebase
   ```

---

### MODE: "update"

**Update commits after PR review feedback.**

**Steps:**

1. **Find the commit to edit:**
   ```bash
   jj log -r 'master..@'
   ```

2. **Edit the commit:**
   ```bash
   jj edit <change-id>
   ```

3. **Make your changes** (files modified are auto-tracked)

4. **IMPORTANT - Create new working copy when done:**
   ```bash
   jj new
   ```
   Without this, future changes keep amending the edited commit!

5. **All descendants auto-rebase!** Verify:
   ```bash
   jj log -r 'master..@'
   ```

6. **Update bookmarks if needed:**
   ```bash
   jj bookmark set pr/feature -r <change-id>
   ```

7. **Push updates (force-push automatic):**
   ```bash
   jj git push
   ```

---

### MODE: "sync"

**Sync with remote and handle merged PRs.**

**After a PR merges:**

1. **Fetch latest:**
   ```bash
   jj git fetch
   ```

2. **Rebase remastering stack onto new master:**
   ```bash
   jj rebase -d master@origin -s pr/<next-feature>
   ```

3. **Delete merged bookmark:**
   ```bash
   jj bookmark delete pr/<merged-feature>
   ```

4. **Update PR base on GitHub** (if PR #2 was targeting PR #1, now target master)

5. **Push updated stack:**
   ```bash
   jj git push
   ```

---

## Quick Reference

| Command | Purpose |
|---------|---------|
| `jj log -r 'master..@'` | See commits since master |
| `jj status` | Working copy status |
| `jj diff` | Changes in working copy |
| `jj split` | Interactive split |
| `jj split "glob:pattern"` | Split by file pattern |
| `jj split -r X` | Split older commit |
| `jj describe -r X -m "msg"` | Set commit message |
| `jj edit X` | Edit older commit |
| `jj new` | Create new working copy |
| `jj new A B C` | Create merge commit |
| `jj rebase -r X -d Y` | Move X onto Y |
| `jj rebase -s X -d Y` | Move X and descendants onto Y |
| `jj bookmark create X -r Y` | Create bookmark |
| `jj bookmark set X -r Y` | Move bookmark |
| `jj bookmark delete X` | Delete bookmark |
| `jj git fetch` | Fetch from remote |
| `jj git push` | Push all tracking bookmarks |
| `jj git push --bookmark X` | Push specific bookmark |
| `jj undo` | Undo last operation |
| `jj op log` | Operation history |
| `jj op restore X` | Restore to operation |
| `jj resolve` | Open merge tool for conflicts |

### Revset Shortcuts

| Expression | Meaning |
|------------|---------|
| `@` | Working copy commit |
| `@-` | Parent of @ |
| `@--` | Grandparent |
| `master` | Main bookmark |
| `master@origin` | Remote master |
| `master..@` | Commits since master |

---

## Common Pitfalls

1. **Bookmark didn't move**: Bookmarks don't auto-move! Use `jj bookmark set`.

2. **Forgot `jj new` after edit**: Future changes will amend the edited commit.

3. **Conflicts in descendants**: After editing, descendants may conflict. Check `jj log`.

4. **Independent PRs aren't independent**: If B depends on A's types/exports, they're not truly independent.

5. **Lost PR comments**: Force-push can lose GitHub review comments. Communicate with reviewers.

6. **Can't push conflicts**: jj won't push conflicted commits. Resolve first with `jj resolve`.

7. **Undo doesn't undo push**: `jj undo` is local only. To undo a push, move bookmark back and force-push.

---

## Recovery

Made a mistake? jj tracks all operations:

```bash
jj op log          # See what happened
jj undo            # Undo last operation
jj op restore X    # Go back to operation X
```
