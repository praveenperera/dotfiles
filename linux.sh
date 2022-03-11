#!/bin/sh

# unset before installing sscache
unset RUSTC_WRAPPER

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sudo sh -s -- -y
. $HOME/.cargo/env

# add clippy
rustup component add clippy

# deps from apt
sudo apt-get update
sudo apt-get install unzip \
    zsh \
    gcc \
    crcmod \
    python3-dev \
    python3-pip \
    python3-setuptools \
    fzf \
    xsel \
    tmux \
    ca-certificates \
    curl \
    lsb-release --no-cache-dir  -y

# install sccache
which sccache || cargo install sccache

# exports
export RUSTC_WRAPPER=sccache 

# deps from cargo
cargo install exa ripgrep git-delta cargo-watch fd-find bat skim bottom

# install cargo plugins
cargo install cargo-watch cargo-sweep

# docker
which docker || curl -fsSL https://get.docker.com -o get-docker.sh \
&& sh get-docker.sh && rm get-docker.sh

# antibody
sudo curl -sfL git.io/antibody | sh -s - -b $HOME/.local/bin

# starship
sudo sh -c "$(curl -fsSL https://starship.rs/install.sh)" -- --force

# thefuck
which thefuck || sudo pip3 install thefuck --user

# direnv
sudo curl -sfL https://direnv.net/install.sh | bash
mv $(which direnv) $HOME/.local/bin

# zoxide
sudo curl -sS https://webinstall.dev/zoxide | bash

# mcfly
sudo curl -LSfs https://raw.githubusercontent.com/cantino/mcfly/master/ci/install.sh | sh -s -- --git cantino/mcfly --to $HOME/.local/bin
if [ ! -f $HOME/.zsh_history ]; then
    touch $HOME/.zsh_history
fi

# fnm
sudo curl -fsSL https://fnm.vercel.app/install | bash -s -- --skip-shell

# install kubectl
curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"
sudo install -o "$(whoami)" -g "$(whoami)" -m 0755 kubectl /usr/local/bin/kubectl

# gcloud cli
which gcloud || sudo curl https://sdk.cloud.google.com | bash -s -- --disable-prompts

# change shell to ZSH
sudo chsh -s /bin/zsh

# cleanup
sudo apt-get autoremove -y

export PATH="$PATH:$HOME/.local/bin"
export PATH="$PATH:$HOME/.cargo/bin"
