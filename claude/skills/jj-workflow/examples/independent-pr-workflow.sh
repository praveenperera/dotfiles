#!/bin/bash
# Independent PR Workflow
# Creates parallel PRs where each targets master directly
#
# Result:
#          ┌── A ──────── PR #1 (base: master)
#          │
#   master ┼── B ──────── PR #2 (base: master)
#          │
#          └── C ──────── PR #3 (base: master)

set -e

# === SETUP ===
jj git fetch
jj new master -m "working"

# === MAKE YOUR CHANGES ===
# ... edit files ...

# === SPLIT INTO COMMITS (creates a stack initially) ===
jj split "glob:src/feature-a/*"
jj describe @- -m "feat: feature A"

jj split "glob:src/feature-b/*"
jj describe @- -m "feat: feature B"

jj describe -m "feat: feature C"

# === GET CHANGE IDs ===
echo "=== Your commits (currently stacked) ==="
jj log -r 'master..@-' --no-graph -T 'change_id.short() ++ " " ++ description.first_line() ++ "\n"'

# === MAKE INDEPENDENT (rebase each onto master) ===
# Replace <change-id-X> with actual change IDs
# A is already on master, only rebase B and C
jj rebase -r <change-id-B> -o master
jj rebase -r <change-id-C> -o master

# === CREATE BOOKMARKS ===
jj bookmark create feature-a -r <change-id-A>
jj bookmark create feature-b -r <change-id-B>
jj bookmark create feature-c -r <change-id-C>

# === PUSH ===
jj git push

# === CREATE PRs (all target master) ===
gh pr create --head feature-a --base master --title "feat: feature A"
gh pr create --head feature-b --base master --title "feat: feature B"
gh pr create --head feature-c --base master --title "feat: feature C"

echo "=== Done! Independent PRs created ==="

# === OPTIONAL: Dev merge to work on all together ===
# jj new feature-a feature-b feature-c -m "dev: combined"
