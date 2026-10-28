#!/usr/bin/env bash
# SessionStart hook: load the T3-only rules in claude/t3.md only when T3 runs this
# session, so CLAUDE.md states one default instead of asking the agent to work out
# whether it is in T3. It runs again after a resume or compaction, which keeps the
# rules in context
set -euo pipefail

"$(dirname "$0")/in-t3.sh" || exit 0

rules="$HOME/.claude/t3.md"
[[ -f "$rules" ]] || exit 0

jq -n --rawfile rules "$rules" '{
  hookSpecificOutput: {
    hookEventName: "SessionStart",
    additionalContext: $rules
  }
}'
