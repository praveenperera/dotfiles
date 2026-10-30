#!/usr/bin/env bash
# PreToolUse hook for Bash: T3 Code files a background Bash under the `command` kind,
# which holds a thread in Waiting only once it is registered with `t3_background_wait`,
# and a long command run that way still dies with the 2-hour Bash timeout. Long work
# goes to homebased, and the one background command allowed by default is the
# `homebased wait --next` that waits on it. Keep other background Bash for processes
# nobody waits on, such as dev servers, behind an opt-in
set -euo pipefail

input=$(cat)
[[ $(jq -r '.tool_input.run_in_background // false' <<<"$input") == true ]] || exit 0

command=$(jq -r '.tool_input.command // empty' <<<"$input")

grep -qE '^[[:space:]]*homebased[[:space:]]+wait[[:space:]]' <<<"$command" && exit 0

# an explicit opt-in for a process the agent will not wait on, such as a dev server
grep -qE '#[[:space:]]*long-lived:' <<<"$command" && exit 0

jq -n '{
  hookSpecificOutput: {
    hookEventName: "PreToolUse",
    permissionDecision: "deny",
    permissionDecisionReason: "For a command you will wait on, such as a build, test suite, download, or data pull, submit it as a homebased task and run `homebased wait --next <id>` as a background Bash command with `timeout: 7200000`, then pass its task id to `t3_background_wait`, or run it as foreground Bash if it takes under about 10 minutes. For a process you will not wait on, such as a dev server, add a `# long-lived: <why>` comment to the command."
  }
}'
