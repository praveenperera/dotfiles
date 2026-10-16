#!/usr/bin/env bash
# PreToolUse hook for Agent: deny calls that leave out `model`, because an
# unnamed subagent silently inherits the parent's model (usually Opus)
set -euo pipefail

input=$(cat)
model=$(jq -r '.tool_input.model // empty' <<<"$input")
agent_type=$(jq -r '.tool_input.subagent_type // empty' <<<"$input")

if [[ -n "$model" ]]; then
  exit 0
fi

# forks always inherit the parent model and ignore the parameter
if [[ "$agent_type" == "fork" ]]; then
  exit 0
fi

# a custom agent whose frontmatter pins a model already made the choice
if [[ -n "$agent_type" ]]; then
  for dir in "$HOME/.claude/agents" "${CLAUDE_PROJECT_DIR:-.}/.claude/agents"; do
    for file in "$dir"/*.md; do
      [[ -f "$file" ]] || continue

      frontmatter=$(awk 'NR == 1 && $0 != "---" { exit } NR > 1 && $0 == "---" { exit } NR > 1 { print }' "$file")
      if grep -qx "name: *$agent_type" <<<"$frontmatter" && grep -q '^model: *[^ ]' <<<"$frontmatter"; then
        exit 0
      fi
    done
  done
fi

reason="Name the model on this Agent call (haiku, sonnet, or opus), chosen for what the task needs. Leaving it out inherits the parent model."
jq -n --arg reason "$reason" '{
  hookSpecificOutput: {
    hookEventName: "PreToolUse",
    permissionDecision: "deny",
    permissionDecisionReason: $reason
  }
}'
