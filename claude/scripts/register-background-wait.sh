#!/usr/bin/env bash
# PostToolUse hook for Monitor: T3 lists a thread under Waiting only after its
# Monitor is registered with `t3_background_wait`; a Monitor alone leaves it
# running. Agents kept skipping that step, and a hook cannot call an MCP tool, so
# remind the agent with the Monitor's task id right after it starts
set -euo pipefail

input=$(cat)
command=$(jq -r '.tool_input.command // empty' <<<"$input")
grep -qE 'homebased task wait' <<<"$command" || exit 0

monitor_id=$(jq -r '.tool_response.taskId // empty' <<<"$input")
[[ -z "$monitor_id" ]] && exit 0

jq -n --arg id "$monitor_id" '{
  hookSpecificOutput: {
    hookEventName: "PostToolUse",
    additionalContext: ("In a T3 thread (the t3-code tools are present), call `t3_background_wait` with taskIds [\"" + $id + "\"] now, before ending the turn, so T3 lists the thread under Waiting. Outside T3, ignore this.")
  }
}'
