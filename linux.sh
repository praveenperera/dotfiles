#!/bin/sh

# unset before installing sscache
unset RUSTC_WRAPPER

# install sscache

cargo install sscache

# re-enable sscache
export RUSTC_WRAPPER=sccache 

# deps from cargo
cargo install exa ripgrep

# deps from apt
apt update
apt install docker unzip python3-dev python3-pip python3-setuptools -y

# antibody
curl -sfL git.io/antibody | sh -s - -b /usr/local/bin

# starship
sh -c "$(curl -fsSL https://starship.rs/install.sh)"

# thefuck
pip3 install thefuck --user

# direnv
curl -sfL https://direnv.net/install.sh | bash

# zoxide
curl -sS https://webinstall.dev/zoxide | bash

# mcfly
curl -LSfs https://raw.githubusercontent.com/cantino/mcfly/master/ci/install.sh | sh -s -- --git cantino/mcfly
if [ ! -f $HOME/.zsh_history ]; then
    touch $HOME/.zsh_history
fi

# fnm
curl -fsSL https://fnm.vercel.app/install | bash
