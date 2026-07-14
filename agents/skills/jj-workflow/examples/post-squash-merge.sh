#!/usr/bin/env bash
# Template for rebasing B and descendants after A was squash-merged

set -eu

B=${B_CHANGE_ID:-}
C=${C_CHANGE_ID:-}
MERGED_FEATURE=${MERGED_FEATURE:-}
FEATURE_B=${FEATURE_B:-}
FEATURE_C=${FEATURE_C:-}
TRUNK_BOOKMARK=${TRUNK_BOOKMARK:-}
PR_NUMBER=${PR_NUMBER:-}

require_value() {
  local name=$1
  if [[ -z ${!name:-} ]]; then
    echo "Set $name before enabling this phase" >&2
    exit 2
  fi
}

# inspect local state before any optional network or mutation phase
jj status
jj diff --stat
jj log -r 'trunk() | trunk()..@ | bookmarks()'
jj bookmark list --all-remotes
jj git remote list
jj log -r 'trunk()..@ & conflicts()'

if [[ ${JJ_AUTHORIZE_FETCH:-0} == 1 ]]; then
  jj git fetch
  jj log -r 'trunk() | trunk()..@ | bookmarks()'
fi

if [[ ${JJ_AUTHORIZE_REWRITE:-0} == 1 ]]; then
  require_value B_CHANGE_ID
  jj rebase -s "$B" -o 'trunk()'
fi

if [[ -n $B ]]; then
  jj log -r "trunk() | $B::"
  jj diff -r "$B"
  jj log -r "($B::) & conflicts()"
fi

if [[ ${JJ_AUTHORIZE_BOOKMARKS:-0} == 1 ]]; then
  require_value B_CHANGE_ID
  require_value C_CHANGE_ID
  require_value MERGED_FEATURE
  require_value FEATURE_B
  require_value FEATURE_C
  jj bookmark set "$FEATURE_B" -r "$B"
  jj bookmark set "$FEATURE_C" -r "$C"
  jj bookmark delete "$MERGED_FEATURE"
  jj bookmark list --all-remotes
fi

if [[ ${JJ_AUTHORIZE_PUSH:-0} == 1 ]]; then
  require_value FEATURE_B
  require_value FEATURE_C
  jj git push --bookmark "$FEATURE_B"
  jj git push --bookmark "$FEATURE_C"
  jj bookmark list --all-remotes
fi

if [[ ${JJ_AUTHORIZE_PR_EDIT:-0} == 1 ]]; then
  require_value PR_NUMBER
  require_value TRUNK_BOOKMARK
  gh pr edit "$PR_NUMBER" --base "$TRUNK_BOOKMARK"
fi
