#Path to your oh-my-zsh configuration.
alias pp='cd && cd sites/praveenperera'
alias cpubex='cd && cd code/public/Elixir'
alias cpubr='cd && cd code/public/ruby'
alias cpriv='cd && cd code/private'
alias docs='cd && cd sites/doctors_of_srilanka'
alias zreload="source ~/.zshrc"
alias ip="ifconfig | sed -En 's/127.0.0.1//;s/.*inet (addr:)?(([0-9]*\.){3}[0-9]*).*/\2/p'"

## elixir 1.6 mix format
alias mix_format="ASDF_ELIXIR_VERSION=ref-master mix format"

alias alle2h=convert_all_eex_to_haml
convert_all_eex_to_haml(){
  for i in $(find_eex_files); do
    e2h $i
  done
}

find_eex_files(){
  find ./lib -name *.eex -print
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

# Add ASDF version manager
. $HOME/.asdf/asdf.sh
. $HOME/.asdf/completions/asdf.bash
