# Path to your oh-my-zsh configuration.
ZSH=$HOME/.oh-my-zsh
alias pp='cd && cd sites/praveenperera'
alias cpubex='cd && cd code/public/Elixir'
alias cpubr='cd && cd code/public/ruby'
alias cpriv='cd && cd code/private'
alias docs='cd && cd sites/doctors_of_srilanka'
alias emacsg="/usr/local/Cellar/emacs/24.5/Emacs.app/Contents/MacOS/Emacs -nw"

eval $(thefuck --alias)
# Set name of the theme to load.
# Look in ~/.oh-my-zsh/themes/
# Optionally, if you set this to "random", it'll load a random theme each
# time that oh-my-zsh is loaded.
ZSH_THEME="excid3"

# Example aliases
# alias zshconfig="mate ~/.zshrc"
# alias ohmyzsh="mate ~/.oh-my-zsh"

# Set to this to use case-sensitive completion
# CASE_SENSITIVE="true"

# Comment this out to disable weekly auto-update checks
# DISABLE_AUTO_UPDATE="true"

# Uncomment following line if you want to disable colors in ls
# DISABLE_LS_COLORS="true"

# Uncomment following line if you want to disable autosetting terminal title.
# DISABLE_AUTO_TITLE="true"

# Uncomment following line if you want red dots to be displayed while waiting for completion
# COMPLETION_WAITING_DOTS="true"

# Which plugins would you like to load? (plugins can be found in ~/.oh-my-zsh/plugins/*)
# Custom plugins may be added to ~/.oh-my-zsh/custom/plugins/
# Example format: plugins=(rails git textmate ruby lighthouse)
plugins=(git osx ruby rails bundler brew rake cap elixir vi-mode)

export PATH="$HOME/.rbenv/bin:$HOME/.rbenv/shims:$PATH"
export PATH="/Applications/Postgres.app/Contents/Versions/9.4/bin:$PATH"

source $ZSH/oh-my-zsh.sh
export EDITOR=vim

bindkey -v

# vi mode indicator for zsh
precmd() { RPROMPT="" }
function zle-line-init zle-keymap-select {
   VIM_PROMPT="%{$fg_bold[yellow]%} [% NORMAL]%  %{$reset_color%}"
   RPS1="${${KEYMAP/vicmd/$VIM_PROMPT}/(main|viins)/} $EPS1"
   zle reset-prompt
}

zle -N zle-line-init
zle -N zle-keymap-select
export KEYTIMEOUT=1

#added by iterm2 v3
test -e "${HOME}/.iterm2_shell_integration.zsh" && source "${HOME}/.iterm2_shell_integration.zsh"
