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
alias yrn="yarn_in_phoenix"
alias rex="evcxr"
alias k="kubectl"

alias flush='dscacheutil -flushcache'

eval "$(starship init zsh)"
eval $(thefuck --alias)
eval "$(direnv hook zsh)"
source <(navi widget zsh)

yarn_in_phoenix() {
  if [ ! -f package.json ] && [ -f mix.exs ]; then
    printf "phoenix project detected...\n\n"
    yarn --cwd assets "$@"
  else
    yarn "$@"
  fi
}

# converts ocaml code into reason
alias mlre="pbpaste | refmt --parse ml --print re --interface false | pbcopy"
# converts reason code into ocaml
alias reml="pbpaste | refmt --parse re --print ml --interface false | pbcopy"

alias alle2h=convert_all_eex_to_haml
convert_all_eex_to_haml(){
  for i in $(find_eex_files); do
    e2h $i
  done
}

find_eex_files(){
  find ./lib -name *.eex -print
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

function kmerge() {
  KUBECONFIG=~/.kube/config:$1 kubectl config view --flatten > ~/.kube/mergedkub && \
   mv ~/.kube/config ~/.kube/config.bak &&\
   mv ~/.kube/mergedkub ~/.kube/config
}

### Git Alias
alias gcm="git commit -a -S -m $1"

# enable fuzzy searching in mcfly
export MCFLY_FUZZY=true

# Setup path for imagemagick 6
export PATH="/usr/local/opt/imagemagick@6/bin:$PATH"

# elixir escripts
export PATH=$PATH:$HOME/.mix/escripts

export LANG='en_US.UTF-8'
export PATH=$PATH:/Applications/Postgres.app/Contents/Versions/latest/bin
export PATH="$HOME/.yarn/bin:$PATH"
export ELIXIR_EDITOR="code -w"
export EDITOR="code -w"
export ALTERNATE_EDITOR=vim
export VISUAL="code -w"
export PATH="/usr/local/opt/coreutils/libexec/gnubin:$PATH"
export PATH="/usr/local/sbin:$PATH"
export PATH="/usr/local/opt/imagemagick@6/bin:$PATH"
export PATH="/usr/local/opt/mysql@5.5/bin:$PATH"
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

export PATH=$PATH:~/Library/Python/3.7/bin

# rvm
export PATH=$PATH:~/.gem/bin

# added by travis gem
[ -f /Users/praveen/.travis/travis.sh ] && source /Users/praveen/.travis/travis.sh
export PATH="/usr/local/opt/mysql@5.7/bin:$PATH"

#added by iterm2 v3
test -e "${HOME}/.iterm2_shell_integration.zsh" && source "${HOME}/.iterm2_shell_integration.zsh"

. $HOME/.opam/opam-init/init.zsh > /dev/null 2> /dev/null || true

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

setopt histreduceblanks
setopt histignorespace
setopt autocd autopushd pushdminus pushdsilent pushdtohome pushdignoredups
setopt extendedglob
setopt EXTENDED_HISTORY
setopt HIST_EXPIRE_DUPS_FIRST
setopt HIST_IGNORE_DUPS
setopt HIST_IGNORE_ALL_DUPS
setopt HIST_IGNORE_SPACE
setopt HIST_FIND_NO_DUPS
setopt HIST_SAVE_NO_DUPS
setopt HIST_BEEP

# enable sccache for rust projects
export RUSTC_WRAPPER=sccache 
export PATH="/usr/local/opt/node@10/bin:$PATH"

# The next line updates PATH for the Google Cloud SDK.
if [ -f '/Users/praveen/code/bin/google-cloud-sdk/path.zsh.inc' ]; then . '/Users/praveen/code/bin/google-cloud-sdk/path.zsh.inc'; fi

# The next line enables shell command completion for gcloud.
if [ -f '/Users/praveen/code/bin/google-cloud-sdk/completion.zsh.inc' ]; then . '/Users/praveen/code/bin/google-cloud-sdk/completion.zsh.inc'; fi

# Add RVM to PATH for scripting. Make sure this is the last PATH variable change.
export PATH="$PATH:$HOME/.rvm/bin"

export SKIM_DEFAULT_COMMAND="fd --type f || rg --files || find ."

# mcfly
if [[ -r "/usr/local/opt/mcfly/mcfly.zsh" ]]; then
  source "/usr/local/opt/mcfly/mcfly.zsh"
  setopt autocd
fi
