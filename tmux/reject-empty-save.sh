#!/usr/bin/env bash
# Keep an empty save from becoming `last`. Continuum restore then kills session 0.

set -euo pipefail

file="${1:-}"
[[ -n "$file" ]] || exit 0
[[ -s "$file" ]] && exit 0

last="${HOME}/.tmux/resurrect/last"
if [[ -s "$last" ]]; then
	cat "$last" > "$file"
	exit 0
fi

# make this save match `last` so resurrect deletes it instead of promoting it
: > "$file"
rm -f "$last"
: > "$last"
