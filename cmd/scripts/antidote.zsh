export ANTIDOTE_HOME=~/.cache/antidote

if [ -z "$DOTFILES_DIR" ]; then
    echo "DOTFILES_DIR env var not set"
    exit 1
fi

# detect platform and source antidote accordingly
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS - use brew
    source $(brew --prefix)/opt/antidote/share/antidote/antidote.zsh
elif [[ -f "/usr/share/zsh-antidote/antidote.zsh" ]]; then
    # linux - try common system path
    source /usr/share/zsh-antidote/antidote.zsh
elif [[ -f "$HOME/.local/share/antidote/antidote.zsh" ]]; then
    # linux - try user local path
    source $HOME/.local/share/antidote/antidote.zsh
else
    echo "antidote not found - install it first"
    exit 1
fi

antidote bundle < $DOTFILES_DIR/zsh_plugins.txt >| $DOTFILES_DIR/zsh_plugins.zsh
