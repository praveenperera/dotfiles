#!/usr/bin/env bash
# PostToolUse hook for Bash: T3 lists a thread under Waiting only after the background
# command that runs `homebased wait` is registered with `t3_background_wait`; the
# command alone leaves it running. Agents kept skipping that step, and a hook cannot
# call an MCP tool, so remind the agent with the task id right after it starts
set -euo pipefail

input=$(cat)
command=$(jq -r '.tool_input.command // empty' <<<"$input")
# checked before in-t3.sh, which walks the process tree, since this runs on every Bash call
grep -qE 'homebased[[:space:]]+wait([[:space:]]|$)' <<<"$command" || exit 0

task_id=$(jq -r '.tool_response.backgroundTaskId // empty' <<<"$input")
[[ -z "$task_id" ]] && exit 0

"$(dirname "$0")/in-t3.sh" || exit 0

jq -n --arg id "$task_id" '{
  hookSpecificOutput: {
    hookEventName: "PostToolUse",
    additionalContext: ("Call `t3_background_wait` with taskIds [\"" + $id + "\"] now, before ending the turn, so T3 lists the thread under Waiting.")
  }
}'
