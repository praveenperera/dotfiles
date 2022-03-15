alias t="tmux attach | tmux"
alias zreload2=exec zsh
alias zreload='source ~/.zshrc'
alias ip="ifconfig | sed -En 's/127.0.0.1//;s/.*inet (addr:)?(([0-9]*\.){3}[0-9]*).*/\2/p'"
alias pu="pushd"
alias po="popd"
alias pwd2=$(pwd | awk -F\/ '{print $(NF-1),$(NF)}' | sed "s/ /\\//" )
alias gbb="git for-each-ref --sort=-committerdate refs/heads/ --format='%(HEAD) %(color:yellow)%(refname:short)%(color:reset) - %(color:red)%(objectname:short)%(color:reset) - %(authorname) (%(color:green)%(committerdate:relative)%(color:reset))'"
alias gb="git for-each-ref --sort=-committerdate refs/heads/ --format='%(HEAD) %(color:yellow)%(refname:short)%(color:reset) - %(color:red)%(objectname:short)%(color:reset) - %(contents:subject) - %(authorname) (%(color:green)%(committerdate:relative)%(color:reset))'"
alias pip=pip3
alias python=python3
alias rc=rsync -avzhe ssh --progress $1 $2
alias oni="/Applications/Onivim2.App/Contents/MacOS/Oni2"
alias la="exa -lha --icons"
alias rex="evcxr"
alias k="kubectl"
alias clippy-fix="rustup run nightly cargo clippy --fix -Z unstable-options"
alias k="kubectl"
alias flush="dscacheutil -flushcache"
alias td="tmux detach"
alias agee=agee_func
alias aged=aged_func

export SHELL=$(which zsh)

# linux
export PATH="$PATH:$HOME/.local/bin"
export PATH="$PATH:$HOME/.cargo/bin"

# setup history
export HISTSIZE=2000
export SAVEHIST=2000
[[ -n $HISTFILE ]] || export HISTFILE=~/.zsh_history

# subl
export PATH="/Applications/Sublime Text.app/Contents/SharedSupport/bin:$PATH"

# git fzf shortcuts
is_in_git_repo() {
  git rev-parse HEAD > /dev/null 2>&1
}

gco() {
  is_in_git_repo && gcop
}

gcop() {
  local branch=$(git branch -vv --color=always | rg -v '/HEAD\s' |
    fzf --height 60% --reverse --border --ansi --multi --tac | sed 's/^..//' | awk '{print $1}')

  git checkout $branch
  zle reset-prompt
}

gbd() {
  local branch=$(git branch -vv --color=always | rg -v '/HEAD\s' |
    fzf --height 60% --reverse --border --ansi --multi --tac | sed 's/^..//' | awk '{print $1}')

  git bd $branch
}

gclean() {
  git fetch -p && for branch in $(git for-each-ref --format '%(refname) %(upstream:track)' refs/heads | awk '$2 == "[gone]" {sub("refs/heads/", "", $1); print $1}'); do git branch -D $branch; done
}

zle -N gco
bindkey "^F" gco

eval "$(starship init zsh)"
eval $(thefuck --alias)
eval "$(direnv hook zsh)"

local ARCH=$(uname -m)

agee_func() {
  age -e -r $AGE -o "$1".age "$1"
}

aged_func() {
  age --decrypt -i ~/.config/sops/key.txt "$1"
}

alias killp=kill_port
kill_port(){
  KILL_PID=`lsof -i:$1 | awk '{print $2}' | xargs | awk '{print $2}'`

  if [ $KILL_PID ]; then
    NAME=`lsof -i:$1 | awk '{print $1}' | xargs | awk '{print $2}'`
    kill -9 $KILL_PID
    echo "PID: $KILL_PID on PORT: $1 has been terminated ($NAME)"
  else
    echo "No process running on PORT: $1"
  fi
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
  else # there is already a visible frame besides the daemon, so
    change_focus
    # -n $@ errors if there are no args
    test  "$#" -ne "0" && emacsclient -n "$@"
  fi
}

function chr() {
  local cleaned="$(echo ${1}|xargs)"
  local site="https://$cleaned"
  open -a 'google chrome' ${site}
}

function changeMac() {
  local mac=$(openssl rand -hex 6 | sed 's/\(..\)/\1:/g; s/.$//')
  sudo ifconfig en0 ether $mac
  sudo ifconfig en0 down
  sudo ifconfig en0 up
  echo "Your new physical address is $mac"
}

# enable fuzzy searching in mcfly
export MCFLY_FUZZY=true

# Setup path for imagemagick 6
export PATH="/usr/local/opt/imagemagick@6/bin:$PATH"

# elixir escripts
export PATH=$PATH:$HOME/.mix/escripts

export LANG='en_US.UTF-8'
export PATH=$PATH:/Applications/Postgres.app/Contents/Versions/latest/bin
export EDITOR="code -w"
export ALTERNATE_EDITOR=vim
export VISUAL="code -w"
export PATH="/usr/local/sbin:$PATH"
export PATH="$HOME/.cargo/bin:$PATH"
export ANT_HOME=/usr/local/opt/ant
export MAVEN_HOME=/usr/local/opt/maven
export GRADLE_HOME=/usr/local/opt/gradle

export ANDROID_HOME=$HOME/Library/Android/sdk
export ANDROID_SDK_ROOT=$ANDROID_HOME
export PATH=$PATH:$ANDROID_HOME/emulator
export PATH=$PATH:$ANDROID_HOME/tools
export PATH=$PATH:$ANDROID_HOME/tools/bin
export PATH=$PATH:$ANDROID_HOME/platform-tools

export PATH=$ANT_HOME/bin:$PATH
export PATH=$MAVEN_HOME/bin:$PATH
export PATH=$GRADLE_HOME/bin:$PATH

# Enable history in iex through Erlang(OTP)
export ERL_AFLAGS="-kernel shell_history enabled"

# python3 bin path
export PATH=$PATH:~/Library/Python/3.9/bin

# gstreamer
export PKG_CONFIG_PATH="/Library/Frameworks/GStreamer.framework/Versions/Current/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"

TERM=xterm-256color

#kubectl autocompletions
if [ $commands[kubectl] ]; then
  autoload -U +X compinit && compinit
  source <(kubectl completion zsh)
fi

source $HOME/.zsh_plugins.sh

# zsh-history-substring-search
bindkey '^[[A' history-substring-search-up
bindkey '^[[B' history-substring-search-down

# enable recursive i search
bindkey "^R" history-incremental-pattern-search-backward

# opt-left arrow and opt-right arrow move by word
bindkey "[D" backward-word
bindkey "[C" forward-word

# shift-left arrow and shift-right arrow move by word
bindkey ";2D" beginning-of-line
bindkey ";2C" end-of-line

# opt-left arrow and opt-right arrow move by word
bindkey ";3C" forward-word
bindkey ";3D" backward-word

# mcfly
eval "$(mcfly init zsh)"

setopt histreduceblanks
setopt histignorespace
setopt autocd autopushd pushdminus pushdsilent pushdtohome pushdignoredups
setopt extendedglob
setopt EXTENDED_HISTORY          # Write the history file in the ":start:elapsed;command" format.
setopt INC_APPEND_HISTORY        # Write to the history file immediately, not when the shell exits.
setopt SHARE_HISTORY             # Share history between all sessions.
setopt HIST_EXPIRE_DUPS_FIRST    # Expire duplicate entries first when trimming history.
setopt HIST_IGNORE_DUPS          # Don't record an entry that was just recorded again.
setopt HIST_IGNORE_ALL_DUPS      # Delete old recorded entry if new entry is a duplicate.
setopt HIST_FIND_NO_DUPS         # Do not display a line previously found.
setopt HIST_IGNORE_SPACE         # Don't record an entry starting with a space.
setopt HIST_SAVE_NO_DUPS         # Don't write duplicate entries in the history file.
setopt HIST_REDUCE_BLANKS        # Remove superfluous blanks before recording entry.
setopt HIST_VERIFY               # Don't execute immediately upon history expansion.
setopt HIST_BEEP                 # Beep when accessing nonexistent history.

## autocomplete settings
# disable sort when completing `git checkout`
zstyle ':completion:*:git-checkout:*' sort false
# set descriptions format to enable group support
zstyle ':completion:*:descriptions' format '[%d]'
# set list-colors to enable filename colorizing
zstyle ':completion:*' list-colors ${(s.:.)LS_COLORS}
# preview directory's content with exa when completing cd
zstyle ':fzf-tab:complete:cd:*' fzf-preview 'exa -1 --color=always $realpath'
# switch group using `,` and `.`
zstyle ':fzf-tab:*' switch-group ',' '.'
# fallback to filename autocomplete when others fail
zstyle ':completion:*' completer _complete _ignored _files
## / autocomplete settings

# The next line updates PATH for the Google Cloud SDK.
if [ -f '/usr/local/Caskroom/google-cloud-sdk/latest/google-cloud-sdk/path.zsh.inc' ]; then . '/usr/local/Caskroom/google-cloud-sdk/latest/google-cloud-sdk/path.zsh.inc'; fi

# The next line enables shell command completion for gcloud.
if [ -f '/usr/local/Caskroom/google-cloud-sdk/latest/google-cloud-sdk/completion.zsh.inc' ]; then . '/usr/local/Caskroom/google-cloud-sdk/latest/google-cloud-sdk/completion.zsh.inc'; fi

# enable sccache for rust projects
export RUSTC_WRAPPER=sccache 
export SKIM_DEFAULT_COMMAND="fd --type f || rg --files || find ."
export HOMEBREW_NO_AUTO_UPDATE=1
export PATH="/usr/local/opt/openssl@1.1/bin:$PATH"

# fnm
export PATH="$PATH:$HOME/.fnm/"
eval "$(fnm env)"

# age
export AGE=age16du95zg8vcerpjrj7n9xaj2a7hs0kcjukpguveg3xna8nd48yyzqc4k3kx
