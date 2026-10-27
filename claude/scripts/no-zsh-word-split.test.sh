#!/usr/bin/env bash
# Feeds Bash tool inputs to no-zsh-word-split.sh and checks which it denies; each denied
# case is a shape Claude got wrong in zsh, each allowed case a nearby shape that works
set -Eeuo pipefail

SCRIPT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/no-zsh-word-split.sh"
readonly SCRIPT

# the hook stays out of the way unless the shell is zsh
export SHELL=/bin/zsh

failures=0

run() {
    jq -n --arg command "$1" '{tool_input: {command: $command}}' | "${SCRIPT}"
}

expect_deny() {
    local out
    out=$(run "$1")
    if [[ $(jq -r '.hookSpecificOutput.permissionDecision // empty' <<<"${out}") != deny ]]; then
        echo "FAIL (expected deny): $1" >&2
        failures=$((failures + 1))
    fi
}

expect_allow() {
    local out
    out=$(run "$1")
    if [[ -n ${out} ]]; then
        echo "FAIL (expected allow): $1" >&2
        failures=$((failures + 1))
    fi
}

expect_deny 'for u in $left; do echo "$u"; done'
expect_deny 'set -- $mods; echo $#'
expect_deny 'git checkout HEAD~1 -- $files'
expect_deny 'cmd --list ${prev:+--after $prev}'
expect_deny 'xargs rm < /dev/null; rm $paths'

expect_allow 'for u in "${left[@]}"; do echo "$u"; done'
expect_allow 'for f in *.rs; do wc -l "$f"; done'
expect_allow 'files=(a b); git checkout HEAD~1 -- "${files[@]}"'
expect_allow 'files=(a b); git checkout HEAD~1 -- $files'
expect_allow 'git checkout HEAD~1 -- ${=files}'
expect_allow 'cmd "${prev:+--after $prev}"'
expect_allow 'cmd ${prev:+--after} ${prev:+$prev}'
expect_allow 'bash -c '\''for u in $left; do echo "$u"; done'\'''
expect_allow 'git checkout HEAD~1 -- $files # zsh-ok: single path'
expect_allow 'echo $HOME $count'

# outside zsh the splitting works, so the hook has nothing to say
if [[ -n $(SHELL=/bin/bash run 'for u in $left; do :; done') ]]; then
    echo "FAIL (expected allow under bash)" >&2
    failures=$((failures + 1))
fi

if ((failures > 0)); then
    echo "${failures} failure(s)" >&2
    exit 1
fi

echo "ok"
