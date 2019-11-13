#Path to your oh-my-zsh configuration.
alias pp='cd && cd sites/praveenperera'
alias cpubex='cd && cd code/public/Elixir'
alias cpubr='cd && cd code/public/ruby'
alias cpriv='cd && cd code/private'
alias docs='cd && cd sites/doctors_of_srilanka'
alias zreload=exec zsh
alias ip="ifconfig | sed -En 's/127.0.0.1//;s/.*inet (addr:)?(([0-9]*\.){3}[0-9]*).*/\2/p'"
alias pu="pushd"
alias po="popd"
alias pwd2=$(pwd | awk -F\/ '{print $(NF-1),$(NF)}' | sed "s/ /\\//" )
alias gbb="git for-each-ref --sort=-committerdate refs/heads/ --format='%(HEAD) %(color:yellow)%(refname:short)%(color:reset) - %(color:red)%(objectname:short)%(color:reset) - %(authorname) (%(color:green)%(committerdate:relative)%(color:reset))'"
alias gbbb="git for-each-ref --sort=-committerdate refs/heads/ --format='%(HEAD) %(color:yellow)%(refname:short)%(color:reset) - %(color:red)%(objectname:short)%(color:reset) - %(contents:subject) - %(authorname) (%(color:green)%(committerdate:relative)%(color:reset))'"
alias pip=pip3
alias rc=rsync -avzhe ssh --progress $1 $2
alias oni="/Applications/Onivim2.App/Contents/MacOS/Oni2"
alias la="exa -lha"

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

alias e2h=eex2haml
eex2haml(){
  REPLACE_WITH="haml"
  OUTPUT="$(echo $1 | sed -e "s/eex/$REPLACE_WITH/g")"
  html2haml $1 -e --ruby19-attributes $OUTPUT
  rm $1
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

## ZSH Completions from Brew
fpath=(/usr/local/share/zsh-completions $fpath)

eval $(thefuck --alias)

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
export ELIXIR_EDITOR="code -w"
export EDITOR="code -w"
export ALTERNATE_EDITOR=vim
export VISUAL="code -w"
export PATH="/usr/local/opt/coreutils/libexec/gnubin:$PATH"

#added by iterm2 v3
test -e "${HOME}/.iterm2_shell_integration.zsh" && source "${HOME}/.iterm2_shell_integration.zsh"
export PATH="/usr/local/sbin:$PATH"

if [[ -s "${ZDOTDIR:-$HOME}/.zprezto/init.zsh" ]]; then
  source "${ZDOTDIR:-$HOME}/.zprezto/init.zsh"
fi

TERM=xterm-256color

## Add direnv 
eval "$(direnv hook zsh)"
export PATH="/usr/local/opt/imagemagick@6/bin:$PATH"

. /Users/praveen/.opam/opam-init/init.zsh > /dev/null 2> /dev/null || true
export PATH="/usr/local/opt/mysql@5.5/bin:$PATH"
export PATH="$HOME/.cargo/bin:$PATH"


#kubectl autocompletions
if [ $commands[kubectl] ]; then
  source <(kubectl completion zsh)
fi

#enable recursive i search
bindkey "^R" history-incremental-pattern-search-backward

. $HOME/.asdf/asdf.sh

. $HOME/.asdf/completions/asdf.bash

export ANT_HOME=/usr/local/opt/ant
export MAVEN_HOME=/usr/local/opt/maven
export GRADLE_HOME=/usr/local/opt/gradle
export ANDROID_HOME=/usr/local/share/android-sdk
export ANDROID_NDK_HOME=/usr/local/share/android-ndk
export INTEL_HAXM_HOME=/usr/local/Caskroom/intel-haxm

export PATH=$ANT_HOME/bin:$PATH
export PATH=$MAVEN_HOME/bin:$PATH
export PATH=$GRADLE_HOME/bin:$PATH
export PATH=$ANDROID_HOME/tools:$PATH
export PATH=$ANDROID_HOME/platform-tools:$PATH
export PATH=$ANDROID_HOME/build-tools/23.0.1:$PATH

# Enable history in iex through Erlang(OTP)
export ERL_AFLAGS="-kernel shell_history enabled"

export GEM_HOME="$HOME/.gem"
