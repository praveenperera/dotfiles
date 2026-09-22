#!/usr/bin/env bash
# tmux-resurrect restore from an empty `last` file kills the only session.

set -euo pipefail

if [[ "${1:-}" == "--rebuild-managed" ]]; then
	command="$(tmux show-option -gqv @fleet_restore_command)"
	command="${command:-cmd tmux workspace restore}"
	exec tmux run-shell -b "$command"
fi

dir="${HOME}/.tmux/resurrect"
last="${dir}/last"
restore="${HOME}/.tmux/plugins/tmux-resurrect/scripts/restore.sh"

if [[ ! -s "$last" ]]; then
	shopt -s nullglob
	newest=""
	for file in "${dir}"/tmux_resurrect_*.txt; do
		if [[ -s "$file" && ( -z "$newest" || "$file" -nt "$newest" ) ]]; then
			newest="$file"
		fi
	done
	if [[ -z "$newest" ]]; then
		exit 0
	fi
	ln -fs "$(basename "$newest")" "$last"
fi

exec "$restore" "$@"
