#!/usr/bin/env bash
# PreToolUse hook for Monitor: every stdout line of a Monitor wakes the agent for a
# full turn that re-reads the whole context, so waits that stream progress, per-job
# updates, or redrawn `gh run watch` output cost hundreds of thousands of tokens per
# line. Waits on homebased work go to a background `homebased wait --next` instead,
# because a Monitor also expires after 30 minutes and wakes the agent each time, so
# allow only streams the agent opts into
set -euo pipefail

input=$(cat)
command=$(jq -r '.tool_input.command // empty' <<<"$input")

deny() {
  jq -n --arg reason "$1" '{
    hookSpecificOutput: {
      hookEventName: "PreToolUse",
      permissionDecision: "deny",
      permissionDecisionReason: $reason
    }
  }'
  exit 0
}

# a WebSocket source has no command to inspect
[[ -z "$command" ]] && exit 0

# `2>&1 > file` sends stderr, where cargo and most build tools write progress, to the Monitor
if grep -qE '2>&1[[:space:]]*>[[:space:]]*[^&[:space:]]' <<<"$command"; then
  deny "This command redirects in the wrong order: \`2>&1 > file\` sends stderr to the Monitor. Write \`> file 2>&1\`."
fi

# `task wait` is the old name, still accepted until avencera/homebased#6
if grep -qE 'homebased[[:space:]]+(task[[:space:]]+)?wait([[:space:]]|$)' <<<"$command"; then
  deny "A Monitor expires after 30 minutes and wakes you each time with nothing to report. Run \`homebased wait --next <id> [<id> ...]\` as a background Bash command with \`timeout: 7200000\` instead, pass its task id to \`t3_background_wait\`, and end the turn."
fi

# an explicit opt-in for a stream the agent wants, such as each new error in a log
grep -qE '#[[:space:]]*stream:' <<<"$command" && exit 0

deny "A Monitor wakes you for a full turn on every stdout line, so a wait must print once, when the job ends. Submit the command as a homebased task and run \`homebased wait --next <id>\` as a background Bash command with \`timeout: 7200000\`, or run it as foreground Bash if it takes under about 10 minutes. To stream on purpose, for example each new error in a log, add a \`# stream: <why>\` comment to the command and filter it to the lines you would act on."
