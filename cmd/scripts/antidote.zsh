export ANTIDOTE_HOME=~/.cache/antidote

if [ -z "$DOTFILES_DIR" ]; then
    echo "DOTFILES_DIR env var not set"
    exit 1
fi

# detect platform and source antidote accordingly
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS - use brew
    if ! command -v brew &> /dev/null; then
        echo "installing homebrew..."
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    fi

    if ! brew list antidote &> /dev/null; then
        echo "installing antidote via homebrew..."
        brew install antidote
    fi

    source $(brew --prefix)/opt/antidote/share/antidote/antidote.zsh
else
    # linux - check common paths and install if not found
    if [[ -f "/usr/share/zsh-antidote/antidote.zsh" ]]; then
        source /usr/share/zsh-antidote/antidote.zsh
    elif [[ -f "$HOME/.local/share/antidote/antidote.zsh" ]]; then
        source $HOME/.local/share/antidote/antidote.zsh
    else
        echo "installing antidote..."
        mkdir -p ~/.local/share
        git clone --depth=1 https://github.com/mattmc3/antidote.git ~/.local/share/antidote
        source $HOME/.local/share/antidote/antidote.zsh
    fi
fi

antidote bundle < $DOTFILES_DIR/zsh_plugins.txt >| $DOTFILES_DIR/zsh_plugins.zsh
