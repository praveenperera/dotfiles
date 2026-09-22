# remote tmux stays on the default server so direct SSH sees the same sessions
_fleet_tmux() {
  emulate -L zsh
  local destination=$1
  shift

  if [[ $# == 1 && ($1 == -h || $1 == --help) ]]; then
    print -r -- 'Usage: ttc|ttt [SESSION | -a SESSION | --list]'
    print -r -- 'No arguments: list sessions. SESSION: create or attach. -a: attach only.'
    return 0
  fi

  if [[ $# == 0 || ($# == 1 && ($1 == -l || $1 == --list)) ]]; then
    ssh -- "$destination" 'tmux list-sessions'
    return $?
  fi

  local attach_only=false
  if [[ $1 == -a || $1 == --attach ]]; then
    attach_only=true
    shift
  fi

  if [[ $# != 1 || -z $1 || $1 == -* || $1 == *[.:]* || $1 == *$'\n'* || $1 == *$'\r'* ]]; then
    print -u2 -r -- 'Usage: ttc|ttt [SESSION | -a SESSION | --list]'
    print -u2 -r -- 'Use a nonempty session name without dots, colons, line breaks, or a leading dash.'
    return 2
  fi

  local session=$1 remote_command
  if [[ $attach_only == true ]]; then
    # exact targets prevent attaching to a different session with a matching prefix
    local target="=$session"
    remote_command="tmux attach-session -t ${(qq)target}"
  else
    # SSH joins command arguments into shell text; quote for the remote shell too
    remote_command="tmux new-session -A -s ${(qq)session}"
  fi

  ssh -t -- "$destination" "$remote_command"
}

# Connect to or list tmux sessions on the coding container
ttc() { _fleet_tmux praveen@code.local "$@"; }

# Connect to or list tmux sessions on the training container
ttt() { _fleet_tmux praveen@training.local "$@"; }
