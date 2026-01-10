#!/bin/bash
# Post Squash-Merge Workflow
# Rebase remaining stack after GitHub squash-merged your first PR
#
# Before: master had A, you have A → B → C locally
# After squash-merge: master has squashed-A, you need to rebase B → C onto it

set -e

# === FETCH UPDATED MASTER ===
jj git fetch

# === SEE CURRENT STATE ===
echo "=== Commits after merged PR ==="
jj log -r 'master@origin..@'

# Find B (first commit after the merged PR)
# Replace <B-change-id> with actual change ID

# === REBASE REMAINING STACK ===
# -s (source) moves B and all descendants (C, D, etc.)
jj rebase -s <B-change-id> -o master@origin

# === VERIFY CLEAN HISTORY ===
echo "=== After rebase ==="
jj log -r 'master@origin..@'

# === HANDLE CONFLICTS (if any) ===
# If B conflicts with squashed changes:
#   jj status              # show conflicted files
#   # edit files to resolve
#   jj status              # verify resolved

# === UPDATE BOOKMARKS ===
jj bookmark set pr/feature-b -r <B-change-id>
jj bookmark set pr/feature-c -r <C-change-id>

# === DELETE MERGED BOOKMARK ===
jj bookmark delete pr/feature-a

# === PUSH (force-push happens automatically) ===
jj git push

# === UPDATE PR BASE ON GITHUB ===
# PR #2 was targeting pr/feature-a, now should target master
# Do this manually on GitHub or:
# gh pr edit <pr-number> --base master

echo "=== Done! Stack rebased onto squash-merged master ==="
