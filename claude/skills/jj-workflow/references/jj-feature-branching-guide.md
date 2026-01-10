# Jujutsu (jj) Feature Branching Guide

A comprehensive, actionable guide for managing multiple features with jj, publishing as stacked PRs, independent PRs, or hybrid workflows.

## Table of Contents

1. [Why jj for This Workflow](#1-why-jj-for-this-workflow)
2. [Git Tooling Interop (Colocated Mode)](#2-git-tooling-interop-colocated-mode)
3. [Workflow Modes Overview](#3-workflow-modes-overview)
4. [Bookmarks and Grouping Commits](#4-bookmarks-and-grouping-commits)
5. [Splitting Mixed Work](#5-splitting-mixed-work)
6. [Stacked PR Workflow (Complete)](#6-stacked-pr-workflow-complete)
7. [Independent PR Workflow (Complete)](#7-independent-pr-workflow-complete)
8. [Hybrid Workflow](#8-hybrid-workflow)
9. [Interop and Collaboration](#9-interop-and-collaboration)
10. [Pitfalls and Gotchas](#10-pitfalls-and-gotchas)
11. [Quick Reference Table](#11-quick-reference-table)

---

## 1. Why jj for This Workflow

### Concrete Advantages Over Plain Git

| Aspect | Git | jj | Why It Matters |
|--------|-----|-----|----------------|
| **Working copy** | Separate from commits; requires staging | Is always a commit; auto-amends | No "dirty working copy" errors; no stash needed |
| **Splitting changes** | `git add -p` + complex rebase -i | `jj split` with native TUI | Directly split any commit, not just HEAD |
| **Rebasing descendants** | Manual; must rebase each branch | Automatic on any edit | Edit commit A, and B/C/D rebase automatically |
| **Conflicts** | Stop-the-world; must resolve immediately | First-class; can defer resolution | Rebase completes; resolve when ready |
| **Undo** | Complex; reflog archaeology | `jj undo` or `jj op restore` | Every operation is reversible |
| **Editing history** | `git rebase -i` (interactive, brittle) | `jj rebase -r`, `jj squash --into` | Direct commands for each operation |
| **Branch tracking** | Branches auto-move with HEAD | Bookmarks stay fixed; explicit moves | Clear control over what gets pushed |

### Key Mental Model Shifts

1. **Working copy = commit**: Your uncommitted changes are always in a commit. No staging area.
2. **Changes vs commits**: jj tracks changes by ID across rebases; commit hashes change, change IDs don't.
3. **Automatic rebasing**: When you edit commit X, all descendants automatically rebase.
4. **Conflicts are values**: Conflicts are stored in commits; you can have a conflicted commit and resolve later.
5. **Operations are first-class**: Every jj command creates an operation you can undo/restore.

### When jj Shines

- You're working on 3-4 features that touch overlapping files
- You want to split after-the-fact rather than commit perfectly the first time
- You frequently reorder/squash/split commits
- You maintain stacks of dependent PRs

### Tradeoffs

- **Learning curve**: Different mental model from Git
- **IDE support**: Less mature than Git integrations
- **Force-push culture**: jj encourages history rewriting; GitHub comments can get lost on force-push
- **No staging area**: Some workflows depend on staging; jj doesn't have it

**Sources**: [GitHub - jj-vcs/jj](https://github.com/jj-vcs/jj), [What I've learned from jj](https://zerowidth.com/2025/what-ive-learned-from-jj/), [jj init by Chris Krycho](https://v5.chriskrycho.com/essays/jj-init)

---

## 2. Git Tooling Interop (Colocated Mode)

### What "Colocated" Means

A **colocated workspace** has both `.jj/` and `.git/` directories. jj keeps the Git repository synchronized automatically on every command.

```bash
# Initialize colocated mode in existing Git repo
jj git init --colocate

# Or clone with colocation
jj git clone --colocate git@github.com:user/repo.git
```

### What Git Tools See

| What Git Shows | Why | Implication |
|----------------|-----|-------------|
| **Detached HEAD** | jj doesn't track a "current branch" | Normal; don't be alarmed |
| **Untracked `.jj/` dir** | jj's internal data | Add to `.gitignore` |
| **Commits match** | jj exports to Git on each command | `git log`, `tig`, IDE Git UI work normally |
| **Bookmarks as branches** | jj syncs bookmarks → Git branches | `git branch -a` shows your bookmarks |

### What Works

```bash
# These all work in colocated mode:
git log --oneline --graph    # View history
git diff HEAD~2              # View diffs
git show abc123              # Inspect commits
tig                          # TUI log viewer
# IDE Git panels             # Show history, diffs, blame
```

### What to Avoid

| Don't Do | Why | Do Instead |
|----------|-----|------------|
| `git add / git commit` | Creates commits jj doesn't know about (until next jj cmd) | Use `jj` commands |
| `git rebase -i` | Conflicts with jj's view | Use `jj rebase`, `jj squash` |
| `git merge` | jj handles merges differently | Use `jj new -d A -d B` for merge |
| `git stash` | Unnecessary; working copy is a commit | Just `jj new` to start fresh work |
| `git checkout branch` | Puts jj in confused state | Use `jj new branch-bookmark` |

### Safe Git Commands

```bash
# Read-only commands are always safe:
git status          # See what Git thinks (will show detached HEAD)
git log             # View history
git diff            # View changes
git blame           # Blame annotations
git remote -v       # Check remotes
```

**Sources**: [Git compatibility - Jujutsu docs](https://jj-vcs.github.io/jj/latest/git-compatibility/), [Using Jujutsu in a colocated git repository](https://cuffaro.com/2025-03-15-using-jujutsu-in-a-colocated-git-repository/)

---

## 3. Workflow Modes Overview

### Decision Framework

```
Do your features DEPEND on each other?
          |
    +-----+-----+
    |           |
   YES          NO
    |           |
    v           v
STACKED PRs   INDEPENDENT PRs
A → B → C      A   B   C
              ↘   ↓   ↙
               main
```

### Mode Comparison

| Aspect | Stacked PRs | Independent PRs |
|--------|-------------|-----------------|
| **Commit graph** | Linear chain: main → A → B → C | Parallel: main → A, main → B, main → C |
| **PR targets** | A→main, B→A, C→B | All → main |
| **Merge order** | Must merge A before B before C | Any order |
| **Complexity** | Simpler to create; complex to merge | More setup; simpler to merge |
| **Best for** | Truly dependent changes | Unrelated features |

### ASCII Graphs

**Stacked PRs:**
```
main ─── A ─── B ─── C ─── @ (working copy)
         │     │     │
         │     │     └── PR #3 (base: B)
         │     └──────── PR #2 (base: A)
         └────────────── PR #1 (base: main)
```

**Independent PRs:**
```
         ┌── A ──────── PR #1 (base: main)
         │
main ────┼── B ──────── PR #2 (base: main)
         │
         └── C ──────── PR #3 (base: main)
```

---

## 4. Bookmarks and Grouping Commits

### Understanding Bookmarks

Bookmarks are jj's equivalent to Git branches, but with key differences:

| Git Branch | jj Bookmark |
|------------|-------------|
| Moves with HEAD automatically | Stays fixed; must move explicitly |
| "Current branch" concept | No current bookmark; just @ |
| `git push` pushes current branch | `jj git push` pushes all tracking bookmarks |

### Creating and Managing Bookmarks

```bash
# Create bookmark at current commit's parent (the completed work)
jj bookmark create feature-a -r @-

# Create bookmark at specific revision
jj bookmark create feature-b -r xyz123

# Move bookmark to new location
jj bookmark set feature-a -r @

# Delete bookmark
jj bookmark delete feature-a

# List bookmarks
jj bookmark list
```

### Grouping Multiple Commits

To treat commits A→B→C as a unit, put a bookmark at the tip:

```bash
# After creating your stack:
# main → commit-a → commit-b → commit-c → @

# Bookmark the tip of the group
jj bookmark create pr/feature-x -r @-

# Or if @ is the last real commit:
jj bookmark create pr/feature-x -r @
```

### Naming Conventions

```bash
# Recommended patterns:
pr/feature-name         # For PR branches
pr/123-fix-bug          # With issue number
feature/short-name      # Feature branches
user/experiment         # Personal experiments
```

### Pushing Bookmarks

```bash
# Push specific bookmark
jj git push --bookmark pr/feature-a

# Push all tracking bookmarks
jj git push

# Push bookmark to specific remote
jj git push --remote origin --bookmark pr/feature-a

# Generate bookmark name from change ID and push
jj git push --change xyz123  # Creates push-xyzxyzxyz branch
```

**Sources**: [Bookmarks - Jujutsu docs](https://docs.jj-vcs.dev/latest/bookmarks/), [Understanding Jujutsu bookmarks](https://neugierig.org/software/blog/2025/08/jj-bookmarks.html)

---

## 5. Splitting Mixed Work

### Mental Model

In jj, you split changes by selecting what goes into the **first** commit. Everything else stays in the **second** commit.

```
BEFORE split:
main ─── @ (mixed changes: fileA, fileB, fileC)

AFTER split:
main ─── first (fileA only) ─── second (fileB, fileC)
```

### Basic Splitting

```bash
# Interactive split - opens TUI to select files/hunks
jj split

# Split by file pattern
jj split "glob:src/auth/*"

# Split specific files
jj split file1.ts file2.ts
```

### The Split TUI

When you run `jj split`, you get a diff editor showing all changes. The left side is the "before" state, the right side is what you're selecting for the first commit.

```
┌─────────────────────────────────────────────────┐
│ Select changes for first commit                 │
│                                                 │
│ [x] src/auth/login.ts      (entire file)        │
│ [ ] src/api/users.ts       (entire file)        │
│ [~] src/utils/helpers.ts   (some hunks)         │
│     [x] hunk 1: lines 10-20                     │
│     [ ] hunk 2: lines 50-60                     │
└─────────────────────────────────────────────────┘
```

### Splitting Into Multiple Commits

To split one big change into 4 feature commits:

```bash
# Start: main → @ (all mixed work)

# First split: extract feature A
jj split "glob:src/feature-a/*"
jj describe -r @- -m "feat: implement feature A"

# Second split: extract feature B from remainder
jj split "glob:src/feature-b/*"
jj describe -r @- -m "feat: implement feature B"

# Third split: extract feature C
jj split "glob:src/feature-c/*"
jj describe -r @- -m "feat: implement feature C"

# What remains is feature D
jj describe -m "feat: implement feature D"

# Result:
# main → A → B → C → D (current)
```

### Interactive Hunk Selection

```bash
# For fine-grained hunk selection
jj split -i

# This opens an interactive editor where you can:
# - Select entire files
# - Select individual hunks within files
# - Select specific lines within hunks
```

### Splitting an Older Commit

```bash
# To split a commit that's not @:
jj split -r xyz123

# This splits xyz123 in place; descendants auto-rebase
```

**Sources**: [jj-split man page](https://man.archlinux.org/man/extra/jujutsu/jj-split.1.en), [jj tips and tricks](https://zerowidth.com/2025/jj-tips-and-tricks/)

---

## 6. Stacked PR Workflow (Complete)

### Overview

Create a linear stack where each commit depends on the previous. Each commit becomes a PR that targets the previous PR's branch.

### Step 1: Start from Main

```bash
# Ensure you're up to date
jj git fetch
jj new main -m "starting work"
```

### Step 2: Make Changes (Mixed Work)

```bash
# Work on your features... files get modified
# jj auto-tracks everything; no add needed

# Check status
jj status
jj diff
```

### Step 3: Split Into Stacked Commits

```bash
# Split out first feature
jj split "glob:src/auth/*"
jj describe -r @- -m "feat(auth): add login flow"

# Split out second feature (builds on first)
jj split "glob:src/api/*"
jj describe -r @- -m "feat(api): add user endpoints"

# Remaining work is third feature
jj describe -m "feat(ui): add dashboard"

# Verify the stack
jj log
```

**Resulting graph:**
```
main ─── A (auth) ─── B (api) ─── C (ui) ─── @ (empty working copy)
```

### Step 4: Add Bookmarks

```bash
# Bookmark each commit for its PR
jj bookmark create pr/auth -r A      # or use change ID
jj bookmark create pr/api -r B
jj bookmark create pr/dashboard -r C

# Using change IDs (more precise):
jj log --no-graph -T 'change_id ++ "\n"' -r 'main..@-'
# Copy the change IDs and use them:
jj bookmark create pr/auth -r xyzabc
jj bookmark create pr/api -r defghi
jj bookmark create pr/dashboard -r jklmno
```

### Step 5: Push All Bookmarks

```bash
# Push all at once
jj git push

# Or push individually
jj git push --bookmark pr/auth
jj git push --bookmark pr/api
jj git push --bookmark pr/dashboard
```

### Step 6: Create PRs on GitHub

```bash
# Using gh CLI:
gh pr create --head pr/auth --base main --title "feat(auth): add login flow"
gh pr create --head pr/api --base pr/auth --title "feat(api): add user endpoints"
gh pr create --head pr/dashboard --base pr/api --title "feat(ui): add dashboard"
```

### Step 7: Update After Review Feedback

```bash
# Edit the auth commit
jj edit xyzabc  # change ID of auth commit

# Make changes...

# When done, create new working copy commit
jj new

# All descendants (api, dashboard) auto-rebased!
# Push updates (force push happens automatically)
jj git push
```

### Step 8: After First PR Merges

```bash
# Fetch the merged main
jj git fetch

# Rebase remaining stack onto new main
jj rebase -s pr/api -d main

# Update PR #2 to target main now
# (manually update on GitHub or use gh CLI)

# Delete merged bookmark
jj bookmark delete pr/auth

# Push updated stack
jj git push
```

### Complete Command Sequence (Copy-Paste)

```bash
# === SETUP ===
jj git fetch
jj new main -m "feature work"

# === WORK & SPLIT ===
# ... make your changes ...
jj split "glob:src/feature-a/*"
jj describe -r @- -m "feat: feature A"
jj split "glob:src/feature-b/*"
jj describe -r @- -m "feat: feature B"
jj describe -m "feat: feature C"

# === BOOKMARK ===
jj bookmark create pr/feature-a -r @--
jj bookmark create pr/feature-b -r @-
jj bookmark create pr/feature-c -r @

# === PUSH ===
jj git push

# === CREATE PRs ===
gh pr create --head pr/feature-a --base main --title "feat: feature A"
gh pr create --head pr/feature-b --base pr/feature-a --title "feat: feature B"
gh pr create --head pr/feature-c --base pr/feature-b --title "feat: feature C"
```

---

## 7. Independent PR Workflow (Complete)

### Overview

Each feature commit is rebased directly onto main, creating parallel branches. Each PR targets main independently.

### Step 1: Start with Stacked Work

```bash
# You might naturally create a stack first
jj git fetch
jj new main -m "working"

# ... make changes ...

# Split into commits (same as stacked workflow)
jj split "glob:src/auth/*"
jj describe -r @- -m "feat: auth feature"
jj split "glob:src/api/*"
jj describe -r @- -m "feat: api feature"
jj describe -m "feat: ui feature"
```

**Current graph:**
```
main ─── A ─── B ─── C ─── @
```

### Step 2: Duplicate/Rebase to Make Independent

The key operation is rebasing each commit directly onto main:

```bash
# Get the change IDs
jj log -r 'main..@-' --no-graph -T 'change_id ++ " " ++ description.first_line() ++ "\n"'

# Rebase B onto main (breaking it from A)
jj rebase -r B -d main

# Rebase C onto main (breaking it from B)
jj rebase -r C -d main
```

**Resulting graph:**
```
         ┌── A (auth)
         │
main ────┼── B (api)
         │
         └── C (ui)
```

### Step 3: Add Bookmarks

```bash
jj bookmark create pr/auth -r A
jj bookmark create pr/api -r B
jj bookmark create pr/ui -r C
```

### Step 4: Push and Create PRs

```bash
# Push all
jj git push

# Create PRs - all target main
gh pr create --head pr/auth --base main --title "feat: auth feature"
gh pr create --head pr/api --base main --title "feat: api feature"
gh pr create --head pr/ui --base main --title "feat: ui feature"
```

### Step 5: Working on Multiple Independent Branches

To work on all features simultaneously (seeing combined changes):

```bash
# Create a merge commit for development
jj new pr/auth pr/api pr/ui -m "dev: combined work"

# Now @ contains all three features merged
# Work here; split changes back to appropriate branches
```

### Step 6: Update After Feedback

```bash
# Edit the auth feature
jj edit A  # or use change ID

# Make changes...

# Return to your dev merge
jj new pr/auth pr/api pr/ui -m "dev: combined work"

# Push update
jj git push --bookmark pr/auth
```

### Handling True Independence

If features are truly independent, conflicts should be rare. If you get conflicts when creating the dev merge:

```bash
# jj allows conflicted commits
jj new pr/auth pr/api pr/ui -m "dev: combined"
# If conflicts exist, they'll be in @

# Resolve or work around:
jj resolve  # Opens merge tool
# Or just work with conflicts present
```

### Complete Command Sequence (Copy-Paste)

```bash
# === SETUP & SPLIT ===
jj git fetch
jj new main -m "working"
# ... make changes ...
jj split "glob:src/auth/*" && jj describe -r @- -m "feat: auth"
jj split "glob:src/api/*" && jj describe -r @- -m "feat: api"
jj describe -m "feat: ui"

# === MAKE INDEPENDENT ===
# Get change IDs
jj log -r 'main..@-' --no-graph
# Assuming: main → A → B → C → @

# Rebase B and C onto main independently
jj rebase -r @-- -d main  # B onto main
jj rebase -r @- -d main   # C onto main

# === BOOKMARK ===
jj log  # find the change IDs
jj bookmark create pr/auth -r <A-change-id>
jj bookmark create pr/api -r <B-change-id>
jj bookmark create pr/ui -r <C-change-id>

# === PUSH ===
jj git push

# === CREATE PRs ===
gh pr create --head pr/auth --base main --title "feat: auth"
gh pr create --head pr/api --base main --title "feat: api"
gh pr create --head pr/ui --base main --title "feat: ui"

# === OPTIONAL: DEV MERGE ===
jj new pr/auth pr/api pr/ui -m "dev merge"
```

---

## 8. Hybrid Workflow

### Scenario

You have a local stack A → B → C → D, but want to publish:
- A and B as stacked PRs (B depends on A)
- C as independent PR targeting main
- D depends on A+B but not C

### Before (Linear Stack)

```
main ─── A ─── B ─── C ─── D ─── @
```

### Target Structure

```
         ┌── A ─── B ─── D
main ────┤
         └── C (independent)
```

### Step-by-Step Commands

```bash
# Start with your linear stack
# main → A → B → C → D → @

# Step 1: Identify change IDs
jj log -r 'main..@-' --no-graph -T 'change_id ++ " " ++ description.first_line() ++ "\n"'
# Example output:
# abc111 feat A
# bcd222 feat B
# cde333 feat C (want independent)
# def444 feat D

# Step 2: Extract C to be independent (rebase onto main)
jj rebase -r cde333 -d main

# Step 3: Rebase D to follow B (skipping C)
jj rebase -r def444 -d bcd222

# Step 4: Verify structure
jj log
```

### Resulting Graph

```
         ┌── abc111 (A) ─── bcd222 (B) ─── def444 (D)
main ────┤
         └── cde333 (C)
```

### Add Bookmarks and Push

```bash
# Stacked PRs (A → B → D)
jj bookmark create pr/feat-a -r abc111
jj bookmark create pr/feat-b -r bcd222
jj bookmark create pr/feat-d -r def444

# Independent PR (C)
jj bookmark create pr/feat-c -r cde333

# Push all
jj git push

# Create PRs
gh pr create --head pr/feat-a --base main --title "feat A"
gh pr create --head pr/feat-b --base pr/feat-a --title "feat B"
gh pr create --head pr/feat-d --base pr/feat-b --title "feat D"
gh pr create --head pr/feat-c --base main --title "feat C (independent)"
```

### Working on the Hybrid Structure

```bash
# Create a dev merge to see everything together
jj new pr/feat-d pr/feat-c -m "dev: all features"

# Work here; when you need to update a specific commit:
jj edit abc111  # Edit A
# ... make changes ...
jj new  # Back to working copy

# D automatically rebases; C is unaffected (independent)
jj git push  # Update all affected PRs
```

### Alternative: Duplicate Instead of Rebase

If you want to keep the original stack intact and create a copy for independent PR:

```bash
# Duplicate C onto main (creates new change ID)
jj duplicate cde333 --onto main

# Original C still in stack; new copy is independent
jj log  # Shows both
```

---

## 9. Interop and Collaboration

### Scenario: You Use jj, Teammate Uses Git

Your teammate creates commits on `main` or feature branches using Git. Here's how jj handles it.

### Fetching Teammate's Changes

```bash
# Fetch from remote
jj git fetch

# See what's new
jj log -r 'main@origin'

# Their commits appear as normal commits
# jj imports Git commits automatically
```

### Rebasing Your Work onto Updated Main

```bash
# After fetch, main@origin has new commits
jj rebase -d main@origin -s 'roots(main@origin..@)'

# Or more simply, if you have one feature branch:
jj rebase -d main@origin
```

### What Happens to Your Commits

When jj rebases:
1. **Change IDs stay the same** - jj tracks your changes across rebases
2. **Commit hashes change** - new commits are created
3. **Bookmarks stay attached** - they follow the rebased commits
4. **Conflicts may appear** - jj records them; resolve when ready

### Pushing Rewritten History

```bash
# jj git push automatically force-pushes when needed
jj git push --bookmark pr/my-feature

# jj doesn't have --force-with-lease, but it's safe:
# - If remote bookmark moved unexpectedly, push fails
# - You must fetch and reconcile first
```

### Safety Mechanism

```bash
# If remote changed unexpectedly:
$ jj git push --bookmark pr/feature
Error: Refusing to push bookmark that unexpectedly moved on the remote.
Hint: Try fetching from the remote, then make the bookmark
point to where you want it to be, and push again.

# Resolution:
jj git fetch --branch pr/feature
jj log -r 'pr/feature | pr/feature@origin'  # See divergence
# Decide: rebase yours onto theirs, or override
jj bookmark set pr/feature -r <your-version>
jj git push --bookmark pr/feature
```

### Handling a Teammate's Branch

```bash
# Fetch their branch
jj git fetch --branch teammate/their-feature

# Create a commit based on it
jj new teammate/their-feature@origin -m "my additions"

# Or review it:
jj diff -r teammate/their-feature@origin
```

### Best Practices for Team Workflows

1. **Fetch frequently**: `jj git fetch` is fast; do it often
2. **Rebase before push**: Keep your branches up-to-date with main
3. **Don't rewrite shared history**: Once pushed and reviewed, minimize rewrites
4. **Communicate about force pushes**: Let reviewers know if you force-push
5. **Use descriptive bookmarks**: `user/feature` pattern helps identify ownership

**Sources**: [Working with GitHub - Jujutsu docs](https://jj-vcs.github.io/jj/latest/github/), [Syncing with remote changes](https://renerocks.ai/blog/jj-sync-remote/)

---

## 10. Pitfalls and Gotchas

### 1. Independent PRs That Aren't Actually Independent

**Problem**: You split changes into "independent" PRs, but they actually touch the same code.

```bash
# You think these are independent:
# - PR A: changes src/utils.ts lines 1-50
# - PR B: changes src/utils.ts lines 51-100

# But if B's changes depend on A's types/exports, they're not independent!
```

**Solution**: Test each PR independently before publishing:
```bash
# Check if B works without A:
jj new main -m "test B alone"
jj squash --from B --into @  # Copy B's changes here
# Run tests; if they fail, B depends on A
```

### 2. Lost PR Comments After Force Push

**Problem**: GitHub loses review comments when you force-push rewritten commits.

**Mitigation**:
- GitHub preserves comments on "outdated diffs" (collapsed but visible)
- Use smaller, more frequent updates
- Consider `gh pr comment` to summarize addressed feedback
- Some teams prefer merge commits over rebase for this reason

### 3. Bookmark Didn't Move

**Problem**: You made changes but the bookmark is still on the old commit.

```bash
# jj bookmarks DON'T auto-move like Git branches!

# After making changes:
jj bookmark set pr/feature -r @  # Explicitly move it
jj git push --bookmark pr/feature
```

### 4. Conflicts in Descendants

**Problem**: You edit commit A, and descendants B, C, D all get conflicts.

```bash
# jj auto-rebases descendants, but conflicts propagate
jj log  # Shows which commits have conflicts

# Resolution: fix conflicts in each, or...
# Consider if the edit to A was correct

# To undo:
jj undo
```

### 5. Accidentally Pushed Conflicted Commit

**Problem**: jj won't let you push commits with conflicts.

```bash
# This will fail:
$ jj git push
Error: Won't push commit abc123 since it has conflicts

# You MUST resolve before pushing:
jj edit abc123
jj resolve  # Or manually fix conflict markers
jj new
jj git push
```

### 6. Undo After Push

**Problem**: `jj undo` after `jj git push` doesn't undo the remote push.

```bash
# jj undo only affects local state
# The remote still has your pushed commits!

# To "undo" a push:
# 1. Reset local bookmark back
# 2. Force push

jj bookmark set pr/feature -r @~1  # Move back
jj git push --bookmark pr/feature  # Force push old state
```

### 7. Detached HEAD Confusion in Git Tools

**Problem**: Git tools show "detached HEAD" and it looks broken.

**Explanation**: This is normal in colocated mode. jj doesn't maintain a "current branch" like Git.

```bash
# Git shows:
$ git status
HEAD detached at abc1234

# This is fine! Use jj commands, not git checkout.
```

### 8. Forgetting to Create Working Copy Commit

**Problem**: After `jj edit`, you keep modifying that commit instead of making new changes.

```bash
# After editing an old commit:
jj edit abc123
# ... make fixes ...

# IMPORTANT: Create new working copy when done!
jj new  # Now @ is a fresh commit

# Without this, future changes keep amending abc123
```

### 9. Large Stacks Become Unwieldy

**Problem**: With 10+ stacked PRs, rebases take forever and conflicts multiply.

**Mitigation**:
- Keep stacks to 3-5 PRs max
- Consider hybrid approach: some stacked, some independent
- Merge PRs incrementally as they're approved
- Break large features into separate, parallel stacks

### 10. Bookmark Naming Conflicts

**Problem**: Bookmark already exists locally or remotely with different content.

```bash
# Error: bookmark already exists
# Solution: use different name or delete existing:
jj bookmark delete old-name
jj bookmark create pr/new-name -r @
```

---

## 11. Quick Reference Table

| Command | What It Does | When to Use | Git Equivalent |
|---------|--------------|-------------|----------------|
| **Status & Navigation** |
| `jj status` | Show working copy status | Check what's modified | `git status` |
| `jj log` | Show commit graph | View history | `git log --graph` |
| `jj log -r 'main..@'` | Show commits since main | See your work | `git log main..HEAD` |
| `jj diff` | Show changes in @ | Review before committing | `git diff HEAD` |
| `jj diff -r X` | Show changes in commit X | Review any commit | `git show X` |
| `jj show X` | Show commit X details | Inspect specific commit | `git show X` |
| **Creating & Editing Commits** |
| `jj new` | Create new empty commit on @ | Start fresh work | `git commit` (sort of) |
| `jj new X` | Create new commit on X | Start work based on X | `git checkout X && git commit` |
| `jj new A B` | Create merge commit | Combine branches | `git merge` |
| `jj commit -m "msg"` | Describe @ and create new | Finish a unit of work | `git commit -m` |
| `jj describe -m "msg"` | Set @ description | Add/change commit message | `git commit --amend` (msg only) |
| `jj edit X` | Make X the working copy | Modify an old commit | `git rebase -i` (edit) |
| **Splitting & Squashing** |
| `jj split` | Split @ into two commits | Separate mixed changes | `git add -p && git commit` + rebase |
| `jj split -r X` | Split commit X | Split older commit | `git rebase -i` (edit + split) |
| `jj squash` | Squash @ into parent | Combine with previous | `git commit --amend` |
| `jj squash --into X` | Squash @ into X | Move changes to specific commit | Complex rebase |
| `jj squash --from X` | Squash X into its parent | Combine older commits | `git rebase -i` (squash) |
| **Reordering & Rebasing** |
| `jj rebase -r X -d Y` | Move commit X onto Y | Reorder commits | `git rebase -i` (move) |
| `jj rebase -s X -d Y` | Move X and descendants onto Y | Rebase branch | `git rebase Y X` |
| `jj rebase -r X -A Y` | Insert X after Y | Insert in sequence | Complex rebase |
| `jj rebase -r X -B Y` | Insert X before Y | Insert in sequence | Complex rebase |
| **Bookmarks (Branches)** |
| `jj bookmark create X` | Create bookmark at @ | Name your work | `git branch X` |
| `jj bookmark create X -r Y` | Create bookmark at Y | Name specific commit | `git branch X Y` |
| `jj bookmark set X` | Move bookmark X to @ | Update bookmark | `git branch -f X` |
| `jj bookmark delete X` | Delete bookmark X | Clean up | `git branch -d X` |
| `jj bookmark list` | List all bookmarks | See named refs | `git branch -a` |
| **Remote Operations** |
| `jj git fetch` | Fetch from all remotes | Get latest changes | `git fetch` |
| `jj git push` | Push all tracking bookmarks | Publish your work | `git push` (all branches) |
| `jj git push --bookmark X` | Push specific bookmark | Publish specific branch | `git push origin X` |
| `jj git push --change X` | Push commit with generated name | Quick publish | (no direct equivalent) |
| **Undo & Recovery** |
| `jj undo` | Undo last operation | Made a mistake | `git reflog + reset` |
| `jj op log` | Show operation history | Debug what happened | `git reflog` |
| `jj op restore X` | Restore to operation X | Go back in time | `git reset --hard` (from reflog) |
| `jj abandon X` | Remove commit X | Delete unwanted commit | `git rebase -i` (drop) |
| **Conflicts** |
| `jj resolve` | Open merge tool | Fix conflicts | `git mergetool` |
| `jj resolve --list` | List conflicted files | See what needs fixing | `git diff --name-only --diff-filter=U` |

### Common Revset Expressions

| Expression | Meaning |
|------------|---------|
| `@` | Current working copy commit |
| `@-` | Parent of working copy |
| `@--` | Grandparent of working copy |
| `main` | The commit bookmark `main` points to |
| `main@origin` | Remote bookmark `main` on `origin` |
| `X..Y` | Commits reachable from Y but not X |
| `main..@` | Your commits since main |
| `roots(X)` | Root commits of set X |
| `heads(X)` | Head commits of set X |
| `A \| B` | Union of A and B |
| `A & B` | Intersection of A and B |
| `A ~ B` | A minus B |

---

## Appendix: Configuration Recommendations

Add to `~/.jjconfig.toml`:

```toml
[user]
name = "Your Name"
email = "your@email.com"

[ui]
# Use git-style conflict markers for compatibility
conflict-marker-style = "git"

# Pager
pager = "less -FRX"

# Better diff colors
diff.format = "git"

[git]
# Auto-import Git branches on fetch (optional)
auto-local-bookmark = false

[aliases]
# Common workflows
l = "log -r 'main..@'"
ll = "log"
d = "diff"
s = "status"
```

---

## Sources

- [GitHub - jj-vcs/jj](https://github.com/jj-vcs/jj) - Official repository
- [Jujutsu Documentation](https://jj-vcs.github.io/jj/latest/) - Official docs
- [Steve Klabnik's Jujutsu Tutorial](https://steveklabnik.github.io/jujutsu-tutorial/) - Comprehensive tutorial
- [jj init by Chris Krycho](https://v5.chriskrycho.com/essays/jj-init) - Deep dive essay
- [What I've learned from jj](https://zerowidth.com/2025/what-ive-learned-from-jj/) - Practical experience
- [Understanding Jujutsu bookmarks](https://neugierig.org/software/blog/2025/08/jj-bookmarks.html) - Bookmark mechanics
- [jj-stack](https://github.com/keanemind/jj-stack) - Tool for stacked PRs
- [Jujutsu VCS Introduction](https://kubamartin.com/posts/introduction-to-the-jujutsu-vcs/) - Patterns and workflows
- [A Better Merge Workflow](https://ofcr.se/jujutsu-merge-workflow) - Merge strategies
- [Git compatibility docs](https://jj-vcs.github.io/jj/latest/git-compatibility/) - Colocated mode details
