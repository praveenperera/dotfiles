#!/bin/bash
# Stacked PR Workflow
# Creates dependent PRs where each targets the previous PR's branch
#
# Result:
#   master ─── A ─── B ─── C
#            │     │     │
#            │     │     └── PR #3 (base: B)
#            │     └──────── PR #2 (base: A)
#            └────────────── PR #1 (base: master)

set -e

# === SETUP ===
jj git fetch
jj new master -m "working"

# === MAKE YOUR CHANGES ===
# ... edit files ...

# === SPLIT INTO COMMITS ===
# Split by file pattern (repeat for each feature)
jj split "glob:src/feature-a/*"
jj describe @- -m "feat: feature A"

jj split "glob:src/feature-b/*"
jj describe @- -m "feat: feature B"

jj describe -m "feat: feature C"

# === GET CHANGE IDs ===
echo "=== Your commits ==="
jj log -r 'master..@-' --no-graph -T 'change_id.short() ++ " " ++ description.first_line() ++ "\n"'

# === CREATE BOOKMARKS ===
# Replace <change-id-X> with actual change IDs from above
jj bookmark create pr/feature-a -r <change-id-A>
jj bookmark create pr/feature-b -r <change-id-B>
jj bookmark create pr/feature-c -r <change-id-C>

# === PUSH ===
jj git push

# === CREATE PRs (stacked bases) ===
gh pr create --head pr/feature-a --base master --title "feat: feature A"
gh pr create --head pr/feature-b --base pr/feature-a --title "feat: feature B"
gh pr create --head pr/feature-c --base pr/feature-b --title "feat: feature C"

echo "=== Done! PRs created with stacked bases ==="
