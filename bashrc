# Claude Code's Bash tool runs bash (`CLAUDE_CODE_SHELL` in claude/settings.json) and
# snapshots this file at startup; interactive shells stay on zsh
. "$HOME/.cargo/env"

# zshrc defines these, and Ubuntu ships only python3 and pip3
alias python=python3
alias pip=pip3
