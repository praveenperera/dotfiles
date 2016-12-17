#Path to your oh-my-zsh configuration.
ZSH=$HOME/.oh-my-zsh
alias pp='cd && cd sites/praveenperera'
alias cpubex='cd && cd code/public/Elixir'
alias cpubr='cd && cd code/public/ruby'
alias cpriv='cd && cd code/private'
alias docs='cd && cd sites/doctors_of_srilanka'
alias zreload="source ~/.zshrc"

alias emt=launch_emacs_terminal
launch_emacs_terminal(){
  emacs -nw $1
}

alias emtc=launch_emacs_client_terminal
launch_emacs_client_terminal(){
  emacsclient -nw $1 -a=emacs -nw $1
}
alias em=launch_emacs_client
launch_emacs_client() {
  emacsclient $1 $2 -a=emacs $1 $2
}

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
source $ZSH/oh-my-zsh.sh
plugins=(git osx ruby rails bundler brew rake cap elixir)

export PATH="$HOME/.rbenv/bin:$HOME/.rbenv/shims:$PATH"
export PATH=$PATH:/Applications/Postgres.app/Contents/Versions/latest/bin
export PATH=$PATH:"$HOME/Library/Android/sdk/platform-tools"
export PATH="$HOME/.yarn/bin:$PATH"
export ANDROID_HOME="$HOME/Library/Android/sdk"
export EDITOR=emacsclient
export ALTERNATE_EDITOR=emacs
export VISUAL=emacsclient

#added by iterm2 v3
test -e "${HOME}/.iterm2_shell_integration.zsh" && source "${HOME}/.iterm2_shell_integration.zsh"
export PATH="/usr/local/sbin:$PATH"
