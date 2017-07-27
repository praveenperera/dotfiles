#Path to your oh-my-zsh configuration.
ZSH=$HOME/.oh-my-zsh
alias pp='cd && cd sites/praveenperera'
alias cpubex='cd && cd code/public/Elixir'
alias cpubr='cd && cd code/public/ruby'
alias cpriv='cd && cd code/private'
alias docs='cd && cd sites/doctors_of_srilanka'
alias zreload="source ~/.zshrc"
alias ip="ifconfig | sed -En 's/127.0.0.1//;s/.*inet (addr:)?(([0-9]*\.){3}[0-9]*).*/\2/p'"

alias epi=elm-package-install
elm-package-install(){
  elm-package install -y $1
}

alias em=launch_emacs_client
launch_emacs_client() {
  # emacsclient options for reference
  # -a "" starts emacs daemon and reattaches
  # -c creates a new frame
  # -n returns control back to the terminal
  # -e eval the script
  # -nw no window (launch in terminal)
  visible_frames() {
    emacsclient -a "" -e '(length (visible-frame-list))'
  }

  change_focus() {
    emacsclient -n -e "(select-frame-set-input-focus (selected-frame))" > /dev/null
  }

  test "$(visible_frames)" -eq "1" && change_focus

  if [ "$(visible_frames)" -lt  "2" ]; then # need to create a frame
    # -c $@ with no args just opens the scratch buffer
    emacsclient -n -c "$@" && change_focus
    emacsclient -n -e "(apm-graphic-frame-init)" > /dev/null
  else # there is already a visible frame besides the daemon, so
    change_focus
    # -n $@ errors if there are no args
    test  "$#" -ne "0" && emacsclient -n "$@"
  fi
}

### Git Alias
alias gcm="git commit -a -S -m $1"

## ZSH Completions from Brew
fpath=(/usr/local/share/zsh-completions $fpath)

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


# Setup go path
export PATH=$PATH:$(go env GOPATH)/bin
# Setup path for imagemagick 6
export PATH="/usr/local/opt/imagemagick@6/bin:$PATH"

export LANG='en_US.UTF-8'
export PATH="$HOME/.rbenv/bin:$HOME/.rbenv/shims:$PATH"
export PATH=$PATH:/Applications/Postgres.app/Contents/Versions/latest/bin
export PATH="$HOME/.yarn/bin:$PATH"
export ANDROID_HOME="$HOME/Library/Android/sdk"
export PATH=${PATH}:${ANDROID_HOME}/tools
export PATH=${PATH}:${ANDROID_HOME}/platform-tools
export EDITOR=emacsclient
export ALTERNATE_EDITOR=emacs
export VISUAL=emacsclient
export NVM_DIR="$HOME/.nvm"
. "/usr/local/opt/nvm/nvm.sh"

#added by iterm2 v3
test -e "${HOME}/.iterm2_shell_integration.zsh" && source "${HOME}/.iterm2_shell_integration.zsh"
export PATH="/usr/local/sbin:$PATH"

## Add direnv 
eval "$(direnv hook zsh)"
