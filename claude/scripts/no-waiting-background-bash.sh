#!/usr/bin/env bash
# PreToolUse hook for Bash: T3 Code files a background Bash under the `command` kind,
# which never holds a thread in Waiting, so a turn that ends on one shows as finished
# while the work goes on and the thread drops out of the Working and Waiting rows.
# A homebased task watched by a Monitor holds Waiting, and so does a subagent. Keep
# background Bash for processes nobody waits on, such as dev servers, behind an opt-in
set -euo pipefail

input=$(cat)
[[ $(jq -r '.tool_input.run_in_background // false' <<<"$input") == true ]] || exit 0

command=$(jq -r '.tool_input.command // empty' <<<"$input")

# an explicit opt-in for a process the agent will not wait on, such as a dev server
grep -qE '#[[:space:]]*long-lived:' <<<"$command" && exit 0

jq -n '{
  hookSpecificOutput: {
    hookEventName: "PreToolUse",
    permissionDecision: "deny",
    permissionDecisionReason: "T3 Code does not count a background Bash as pending work, so a turn that ends on one shows as finished instead of Working or Waiting. For a command you will wait on, such as a build, test suite, download, or data pull, submit it as a homebased task and arm a Monitor with `homebased task wait <id>`, or run it as foreground Bash if it takes under about 10 minutes. For a process you will not wait on, such as a dev server, add a `# long-lived: <why>` comment to the command."
  }
}'
