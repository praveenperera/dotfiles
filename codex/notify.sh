#!/usr/bin/env bash
set -euo pipefail

payload="${1:-}"
title="Codex"
message="Turn complete"

if [[ "$payload" != *'"agent-turn-complete"'* ]]; then
  message="Needs attention"
fi

if [[ -n "${TMUX:-}" ]] && command -v cmd >/dev/null 2>&1; then
  cmd tmux notify --type bell --force "$message" >/dev/null 2>&1 || true
fi

/usr/bin/osascript - "$title" "$message" <<'APPLESCRIPT' >/dev/null 2>&1 || true
on run argv
  display notification (item 2 of argv) with title (item 1 of argv)
end run
APPLESCRIPT
