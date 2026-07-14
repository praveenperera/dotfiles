#!/usr/bin/env bash
# Template for verifying and optionally publishing an existing A → B → C stack

set -eu

A=${A_CHANGE_ID:-}
B=${B_CHANGE_ID:-}
C=${C_CHANGE_ID:-}
FEATURE_A=${FEATURE_A:-}
FEATURE_B=${FEATURE_B:-}
FEATURE_C=${FEATURE_C:-}
TRUNK_BOOKMARK=${TRUNK_BOOKMARK:-}

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

if [[ -n $A && -n $B && -n $C ]]; then
  jj log -r "trunk() | $A | $B | $C"
  jj log -r "($A | $B | $C) & conflicts()"
fi

if [[ ${JJ_AUTHORIZE_BOOKMARKS:-0} == 1 ]]; then
  require_value A_CHANGE_ID
  require_value B_CHANGE_ID
  require_value C_CHANGE_ID
  require_value FEATURE_A
  require_value FEATURE_B
  require_value FEATURE_C
  jj bookmark create "$FEATURE_A" -r "$A"
  jj bookmark create "$FEATURE_B" -r "$B"
  jj bookmark create "$FEATURE_C" -r "$C"
  jj bookmark list --all-remotes
fi

if [[ ${JJ_AUTHORIZE_PUSH:-0} == 1 ]]; then
  require_value FEATURE_A
  require_value FEATURE_B
  require_value FEATURE_C
  jj git push --bookmark "$FEATURE_A"
  jj git push --bookmark "$FEATURE_B"
  jj git push --bookmark "$FEATURE_C"
  jj bookmark list --all-remotes
fi

if [[ ${JJ_AUTHORIZE_PRS:-0} == 1 ]]; then
  require_value FEATURE_A
  require_value FEATURE_B
  require_value FEATURE_C
  require_value TRUNK_BOOKMARK
  gh pr create --head "$FEATURE_A" --base "$TRUNK_BOOKMARK" --title "feat: feature A"
  gh pr create --head "$FEATURE_B" --base "$FEATURE_A" --title "feat: feature B"
  gh pr create --head "$FEATURE_C" --base "$FEATURE_B" --title "feat: feature C"
fi
