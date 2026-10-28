#!/usr/bin/env bash
# Exit 0 when the Claude session running this hook belongs to a T3 Code thread.
# Environment variables are not enough: T3CODE_HOME and the sdk-ts entrypoint leak
# into a `claude -p` that an agent starts from a T3 thread, and that nested session
# has no t3-code tools. T3 starts its sessions directly under `t3 serve`, so check
# the parent of the nearest claude process instead
set -euo pipefail

pid=$$
while [[ "$pid" -gt 1 ]]; do
  read -r ppid comm < <(ps -o ppid=,comm= -p "$pid") || exit 1

  if [[ "${comm##*/}" == claude ]]; then
    parent=$(ps -o command= -p "$ppid") || exit 1
    [[ "$parent" =~ (^|/)t3\ serve($|\ ) ]]
    exit
  fi

  pid=$ppid
done

exit 1
