#!/usr/bin/env bash
# PreToolUse hook for Bash: the Bash tool runs zsh, which does not word-split an unquoted
# parameter the way bash does, so `for f in $files`, `set -- $x`, `git checkout $files` and
# `${prev:+--after $prev}` hand the command one joined argument. The command then fails
# loudly or, worse, silently checks the wrong thing. This catches the common shapes and
# points at the zsh spelling; a script run by bash, or a line marked `# zsh-ok:`, passes
set -euo pipefail

[[ ${SHELL##*/} == zsh ]] || exit 0

input=$(cat)
command=$(jq -r '.tool_input.command // empty' <<<"$input")
[[ -z "$command" ]] && exit 0

# an explicit opt-in for a line that is already correct, such as one inside a quoted string
grep -qE '#[[:space:]]*zsh-ok:' <<<"$command" && exit 0

# bash runs the splitting itself, and `emulate sh` / `setopt sh_word_split` restore it in zsh
grep -qE '(^|[[:space:];&|(])(bash|sh)[[:space:]]|^#!.*/(env )?(bash|sh)|emulate[[:space:]]+(-L[[:space:]]+)?sh|sh_word_split' <<<"$command" && exit 0

name='[A-Za-z_][A-Za-z0-9_]*'
# names that usually hold several words; a plain scalar name is too common to flag
listy='files|paths|flags|opts|options|args|ids|names|targets|mods|headers|list'

# an array assignment or declaration makes the unquoted expansion correct in zsh
for_array() {
  grep -qE "(^|[[:space:];&|(])$1=\(|(typeset|local|declare)[[:space:]]+-[A-Za-z]*a[A-Za-z]*[[:space:]]+$1" <<<"$command"
}

hint=""

if [[ $command =~ for[[:space:]]+$name[[:space:]]+in[[:space:]]+\$\{?($name)\}?([[:space:];]|$) ]]; then
  for_array "${BASH_REMATCH[1]}" || hint="\`for ... in \$${BASH_REMATCH[1]}\` iterates one word in zsh"
fi

if [[ -z $hint && $command =~ set[[:space:]]+--[[:space:]]+\$\{?($name) ]]; then
  for_array "${BASH_REMATCH[1]}" || hint="\`set -- \$${BASH_REMATCH[1]}\` sets one positional parameter in zsh"
fi

# `${var:+--flag $var}` unquoted expands to a single word "--flag value"
if [[ -z $hint && $command =~ (^|[^\"])\$\{$name:[-+=?][^}\"]*[[:space:]][^}]*\} ]]; then
  hint="an unquoted \`\${var:+--flag \$var}\` expands to one word in zsh"
fi

if [[ -z $hint && $command =~ [[:space:]]\$\{?($listy)\}?([[:space:];|\&\)]|$) ]]; then
  for_array "${BASH_REMATCH[1]}" || hint="an unquoted \`\$${BASH_REMATCH[1]}\` passes one joined argument in zsh"
fi

[[ -z $hint ]] && exit 0

jq -n --arg hint "$hint" '{
  hookSpecificOutput: {
    hookEventName: "PreToolUse",
    permissionDecision: "deny",
    permissionDecisionReason: ("zsh does not word-split unquoted parameters, so " + $hint + ": the command gets one argument with the spaces in it. Use an array (`files=(a b); cmd \"${files[@]}\"`), split on purpose with `${=var}`, build the arguments with `${(f)var}` for lines, or put the logic in a `#!/usr/bin/env bash` script and run it with bash. If this line is already correct, add a `# zsh-ok: <why>` comment to the command.")
  }
}'
