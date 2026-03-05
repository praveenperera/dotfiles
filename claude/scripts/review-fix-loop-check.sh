#!/bin/bash
INPUT=$(cat)

# Prevent infinite loop - if stop hook already active, allow stop
STOP_HOOK_ACTIVE=$(echo "$INPUT" | jq -r '.stop_hook_active // false')
if [ "$STOP_HOOK_ACTIVE" = "true" ]; then
  exit 0
fi

# Find the tracking file in cwd
CWD=$(echo "$INPUT" | jq -r '.cwd')
TRACKING_FILE="$CWD/PR_REVIEW_FIXES.md"

# If tracking file doesn't exist yet, allow stop (skill hasn't started)
if [ ! -f "$TRACKING_FILE" ]; then
  exit 0
fi

# If cycle is complete, allow stop
if grep -q "^## STATUS: COMPLETE" "$TRACKING_FILE"; then
  exit 0
fi

# Safety cap: if 5+ iterations, allow stop
ITERATION_COUNT=$(grep -c "^## Iteration" "$TRACKING_FILE")
if [ "$ITERATION_COUNT" -ge 5 ]; then
  echo "REVIEW-FIX CYCLE HIT 5-ITERATION SAFETY CAP. Set STATUS: COMPLETE in PR_REVIEW_FIXES.md and note any remaining issues." >&2
  exit 0
fi

# Cycle not complete - block stop and tell Claude to continue
echo "REVIEW-FIX CYCLE NOT COMPLETE. Read PR_REVIEW_FIXES.md for context on what has been fixed so far, then run another PR review toolkit iteration. Pass the tracking doc contents to the reviewer so it doesn't re-flag already-fixed issues. If no new issues are found, set STATUS: COMPLETE." >&2
exit 2
