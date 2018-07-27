#Path to your oh-my-zsh configuration.
alias pp='cd && cd sites/praveenperera'
alias cpubex='cd && cd code/public/Elixir'
alias cpubr='cd && cd code/public/ruby'
alias cpriv='cd && cd code/private'
alias docs='cd && cd sites/doctors_of_srilanka'
alias zreload="source ~/.zshrc"
alias ip="ifconfig | sed -En 's/127.0.0.1//;s/.*inet (addr:)?(([0-9]*\.){3}[0-9]*).*/\2/p'"
alias pu="pushd"
alias po="popd"
alias gbb="git for-each-ref --sort=-committerdate refs/heads/ --format='%(HEAD) %(color:yellow)%(refname:short)%(color:reset) - %(color:red)%(objectname:short)%(color:reset) - %(authorname) (%(color:green)%(committerdate:relative)%(color:reset))'"
alias gbbb="git for-each-ref --sort=-committerdate refs/heads/ --format='%(HEAD) %(color:yellow)%(refname:short)%(color:reset) - %(color:red)%(objectname:short)%(color:reset) - %(contents:subject) - %(authorname) (%(color:green)%(committerdate:relative)%(color:reset))'"

certbot-aws(){
    mkdir $HOME/.letsencrypt

    docker run -it --rm --name certbot \
    -v "$HOME/.letsencrypt:/etc/letsencrypt" \
    -v "$HOME/.letsencrypt:/var/lib/letsencrypt" \
    -e AWS_SECRET_ACCESS_KEY=$AWS_SECRET_ACCESS_KEY \
    -e AWS_ACCESS_KEY_ID=$AWS_ACCESS_KEY_ID \
    certbot/dns-route53 certonly \
    --dns-route53 \
    --server https://acme-v02.api.letsencrypt.org/directory -d *.$1 -d $1; \

    mkdir -p ~/code/certs/$1
    cp -R ~/.letsencrypt/archive/$1/* ~/code/certs/$1 ;\
    cd ~/code/certs/$1 ;\
    openssl rsa -inform pem -in privkey*.pem -out privkeyrsa.key
}

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
export PATH="$HOME/.cargo/bin:$PATH"
export ELIXIR_EDITOR=emacsclient
export EDITOR=emacsclient
export ALTERNATE_EDITOR=emacs
export VISUAL=emacsclient
export NVM_DIR="$HOME/.nvm"
. "/usr/local/opt/nvm/nvm.sh"

#added by iterm2 v3
test -e "${HOME}/.iterm2_shell_integration.zsh" && source "${HOME}/.iterm2_shell_integration.zsh"
export PATH="/usr/local/sbin:$PATH"

if [[ -s "${ZDOTDIR:-$HOME}/.zprezto/init.zsh" ]]; then
  source "${ZDOTDIR:-$HOME}/.zprezto/init.zsh"
fi

TERM=xterm-256color

## Add direnv 
eval "$(direnv hook zsh)"
source /usr/local/share/zsh-history-substring-search/zsh-history-substring-search.zsh
export PATH="/usr/local/opt/imagemagick@6/bin:$PATH"

. /Users/praveen/.opam/opam-init/init.zsh > /dev/null 2> /dev/null || true
export PATH="/usr/local/opt/mysql@5.5/bin:$PATH"
