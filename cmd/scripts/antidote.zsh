export ANTIDOTE_HOME=~/.cache/antidote

if [ -z "$DOTFILES_DIR" ]; then
    echo "DOTFILES_DIR env var not set"
    exit 1
fi

source $(brew --prefix)/opt/antidote/share/antidote/antidote.zsh 
antidote bundle < $DOTFILES_DIR/zsh_plugins.txt > | $DOTFILES_DIR/zsh_plugins.zsh
