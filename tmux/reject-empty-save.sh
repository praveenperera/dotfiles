#!/usr/bin/env bash
# Exclude fleet presentation sessions from resurrect snapshots

set -euo pipefail

file="${1:-}"
[[ -n "$file" ]] || exit 0

declare -A managed_sessions=()
while IFS=$'\t' read -r session role; do
	case "$role" in
	view|service|display) managed_sessions["$session"]=1 ;;
	esac
done < <(tmux list-sessions -F $'#{session_name}\t#{@fleet_role}' 2>/dev/null || true)

declare -A managed_windows=()
declare -A managed_launchers=()
declare -A window_pane_counts=()
while IFS=$'\t' read -r session window_index pane_index pane_id view_key; do
	[[ -n "$view_key" ]] || continue
	window="${session}:${window_index}"
	managed_windows["$window"]=1
	window_pane_counts["$window"]=$(( ${window_pane_counts[$window]:-0} + 1 ))
	launcher="${managed_launchers[$window]:-}"
	launcher_id="${launcher%%:*}"
	if [[ -z "$launcher" || ${pane_id#%} -lt ${launcher_id#%} ]]; then
		managed_launchers["$window"]="${pane_id}:${pane_index}"
	fi
done < <(tmux list-panes -a -F $'#{session_name}\t#{window_index}\t#{pane_index}\t#{pane_id}\t#{@fleet_view_key}' 2>/dev/null || true)

# keep the complete input line while also splitting its fields for decisions
filter_snapshot() {
	local source="$1"
	local destination="$2"
	local line line_type session related_session window pane_index launcher

	: > "$destination"
	while IFS= read -r line; do
		IFS=$'\t' read -r line_type session related_session _rest <<< "$line"
		case "$line_type" in
		pane)
			[[ -n "${managed_sessions[$session]:-}" ]] && continue
			window="${session}:${related_session}"
			if [[ -n "${managed_windows[$window]:-}" ]]; then
				[[ ${window_pane_counts[$window]} -gt 1 ]] || continue
				IFS=$'\t' read -r _ _ _ _ _ pane_index _ <<< "$line"
				launcher="${managed_launchers[$window]##*:}"
				if [[ "$pane_index" == "$launcher" ]]; then
					printf '%s\n' "${line%$'\t'*}"$'\t:exit' >> "$destination"
					continue
				fi
			fi
			;;
		window)
			[[ -n "${managed_sessions[$session]:-}" ]] && continue
			window="${session}:${related_session}"
			[[ -z "${managed_windows[$window]:-}" || ${window_pane_counts[$window]} -gt 1 ]] || continue
			;;
		grouped_session|state)
			[[ -n "${managed_sessions[$session]:-}" || -n "${managed_sessions[$related_session]:-}" ]] && continue
			;;
		esac
		printf '%s\n' "$line" >> "$destination"
	done < "$source"
}

has_restorable_layout() {
	awk -F '\t' '$1 == "pane" || $1 == "window" { found = 1; exit } END { exit !found }' "$1"
}

filtered="$(mktemp "${file}.filtered.XXXXXX")"
trap 'rm -f "$filtered"' EXIT
filter_snapshot "$file" "$filtered"
if has_restorable_layout "$filtered"; then
	mv "$filtered" "$file"
	exit 0
fi

last="${HOME}/.tmux/resurrect/last"
if [[ -s "$last" ]]; then
	filter_snapshot "$last" "$filtered"
	if has_restorable_layout "$filtered"; then
		mv "$filtered" "$file"
		exit 0
	fi
fi

# make this save match `last` so resurrect deletes it instead of promoting it
: > "$file"
rm -f "$last"
: > "$last"
