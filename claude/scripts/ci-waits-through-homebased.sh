#!/usr/bin/env bash
# PreToolUse hook for Bash: a CI or release wait run in the turn, such as `gh run
# watch` or a sleep loop around `gh pr checks`, holds the thread for as long as CI
# takes, and agents chain 10-minute foreground chunks to get around the Bash timeout.
# CI time is unknown up front, so unlike builds there is no foreground carve-out
set -euo pipefail

input=$(cat)
command=$(jq -r '.tool_input.command // empty' <<<"$input")
[[ -z "$command" ]] && exit 0

# a heredoc body is data, such as a commit message or a file that mentions these commands
command=$(awk -v q="'" '
  BEGIN { opener = "<<-?[[:space:]]*[\"" q "]?" }
  skip { if ($0 ~ "^[[:space:]]*" term "[[:space:]]*$") skip = 0; next }
  match($0, opener "[A-Za-z_][A-Za-z0-9_]*") {
    term = substr($0, RSTART, RLENGTH)
    sub(opener, "", term)
    skip = 1
  }
  { print }
' <<<"$command")

# submitting the wait to homebased is the fix, not the problem
grep -qE 'homebased\b.*\btask (run|submit)\b' <<<"$command" && exit 0

# an explicit opt-in for a wait the agent must hold in the turn
grep -qE '#[[:space:]]*foreground-wait:' <<<"$command" && exit 0

# match only where a command runs, so `git add`, `ls`, or `grep` naming one is fine
at='(^|[;&|(`]|\b(do|then|else|timeout[[:space:]]+[^[:space:]]+|sh|bash)[[:space:]])[[:space:]]*'

waits=no
grep -qE "${at}(gh run watch|gh pr checks[^|;&]*--watch|[^[:space:];&|]*watch_(run|pr)\.sh)" <<<"$command" && waits=yes
grep -qE "${at}sleep\b" <<<"$command" && grep -qE "${at}gh (run|pr|api)\b" <<<"$command" && waits=yes
[[ "$waits" == yes ]] || exit 0

jq -n '{
  hookSpecificOutput: {
    hookEventName: "PreToolUse",
    permissionDecision: "deny",
    permissionDecisionReason: "Do not wait on CI or a release run inside the turn. Submit the wait as a homebased task and end the turn: `homebased --json task run --name \"CI for #<n>\" --timeout 4h -- ~/.agents/skills/ship/scripts/watch_pr.sh <n> [--greptile]` for a PR, or `homebased --json task run --name \"<workflow> run\" --timeout 2h -- ~/.agents/skills/ship/scripts/watch_run.sh <workflow> <sha>` for a workflow run such as a release. In a Claude thread, then arm `Monitor` with `homebased task wait <id>`. Both scripts print one line when done; read it with `homebased task log <id> --tail 5`. For a one-shot status check, drop the watch or sleep. To hold a wait in the turn anyway, add a `# foreground-wait: <why>` comment."
  }
}'
