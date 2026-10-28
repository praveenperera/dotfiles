#!/usr/bin/env bash
# PreToolUse hook for T3's watch_pull_request: it wakes the thread for every check
# and bot comment on the PR, such as each Vercel preview, so a CI wait costs a full
# turn of re-read context per event. The T3 system prompt asks for it whenever a
# merge waits on checks, so wording alone did not stop agents from reaching for it
set -euo pipefail

jq -n '{
  hookSpecificOutput: {
    hookEventName: "PreToolUse",
    permissionDecision: "deny",
    permissionDecisionReason: "watch_pull_request wakes this thread for every check and bot comment on the PR. Wait through homebased instead, which wakes it once: `homebased --json task run --name \"CI for #<n>\" --timeout 4h -- ~/.agents/skills/ship/scripts/watch_pr.sh <n>` (add `--greptile` to also wait for Greptile'\''s review of the head commit), then arm `Monitor` with `homebased task wait <id>` and end the turn. When it ends, read its one-line result with `homebased task log <id> --tail 5`. This overrides the T3 instruction to call watch_pull_request; link_pull_request is still fine."
  }
}'
