#!/usr/bin/env bash
# PreToolUse hook for Monitor: every stdout line of a Monitor wakes the agent for a
# full turn that re-reads the whole context, so waits that stream progress, per-job
# updates, or redrawn `gh run watch` output cost hundreds of thousands of tokens per
# line. Allow only `homebased task wait`, which prints once per task event and survives
# a T3 restart, and streams the agent opts into
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

# tail and head hold every event until the wait exits, so the thread learns nothing while it runs
if grep -qE 'homebased task wait' <<<"$command" && grep -qE '\|[[:space:]]*(tail|head)([[:space:]]|$)' <<<"$command"; then
  deny "\`tail\` and \`head\` hold every event until \`homebased task wait\` exits. Run \`homebased task wait <id> [<id> ...]\` with no pipe."
fi

# an explicit opt-in for a stream the agent wants, such as each new error in a log
grep -qE '#[[:space:]]*stream:' <<<"$command" && exit 0

streams=$(grep -cE -- '--line-buffered|tail[[:space:]].*-[fF]|gh run watch|--watch|comm -13' <<<"$command" || true)
if [[ "$streams" == 0 ]] && grep -qE 'homebased task wait' <<<"$command"; then
  exit 0
fi

deny "A Monitor wakes you for a full turn on every stdout line, so a wait must print once, when the job ends. Submit the command as a homebased task and arm the Monitor with \`homebased task wait <id>\`, or run it as foreground Bash if it takes under about 10 minutes. To stream on purpose, for example each new error in a log, add a \`# stream: <why>\` comment to the command and filter it to the lines you would act on."
