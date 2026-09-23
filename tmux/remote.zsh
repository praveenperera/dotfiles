# Remote tmux stays on each machine's default server so direct and managed
# views refer to the same sessions
_fleet_tmux() {
  emulate -L zsh
  local machine=$1
  shift

  command cmd tmux remote "$machine" "$@"
}

# Connect to, import, or list tmux sessions on the coding container
ttc() { _fleet_tmux code "$@"; }
