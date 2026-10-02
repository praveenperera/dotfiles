#!/usr/bin/env bash
set -Eeuo pipefail

usage() {
    cat <<'EOF_USAGE'
Usage: notify.sh <blocked|important|done> <title> <message>

Send a run-auto push notification through ntfy.sh. The topic comes from
RUN_AUTO_NTFY_TOPIC, or from ~/.secrets.zsh when the process environment does
not have it.
EOF_USAGE
}

resolve_topic() {
    if [[ -n "${RUN_AUTO_NTFY_TOPIC:-}" ]]; then
        printf '%s' "${RUN_AUTO_NTFY_TOPIC}"
        return
    fi

    # services such as T3 Code start agents without sourcing ~/.zshrc
    if [[ -f "${HOME}/.secrets.zsh" ]] && command -v zsh >/dev/null; then
        zsh -c 'source ~/.secrets.zsh >/dev/null 2>&1; print -rn -- "${RUN_AUTO_NTFY_TOPIC:-}"'
    fi
}

main() {
    if [[ "$#" -ne 3 ]]; then
        usage >&2
        exit 2
    fi

    local event="$1"
    # a newline in a header value would split the request
    local title="${2//$'\n'/ }"
    local message="$3"
    local priority
    local tags

    case "${event}" in
        blocked) priority=high tags=warning ;;
        important) priority=high tags=exclamation ;;
        done) priority=default tags=white_check_mark ;;
        *)
            usage >&2
            exit 2
            ;;
    esac

    local topic
    topic="$(resolve_topic)"
    if [[ -z "${topic}" ]]; then
        echo "RUN_AUTO_NTFY_TOPIC is not set in the environment or ~/.secrets.zsh" >&2
        exit 1
    fi

    curl -fsS --max-time 15 --retry 2 \
        -H "Title: ${title}" \
        -H "Priority: ${priority}" \
        -H "Tags: ${tags}" \
        --data-binary "${message}" \
        "https://ntfy.sh/${topic}" >/dev/null
}

main "$@"
