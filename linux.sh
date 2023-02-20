#!/bin/sh

# exit on error
set -e

# unset before installing sscache
unset RUSTC_WRAPPER

# Install Rust
if [ ! -x "$(command -v cargo)" ]; then
    echo "Installing Rust..."
    export CARGO_HOME=$HOME/.cargo
    export RUSTUP_HOME=$HOME/.rustup
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o rustup-init.sh
    sh rustup-init.sh -y --no-modify-path --default-toolchain stable
    rm rustup-init.sh
    . $HOME/.cargo/env
fi


# add clippy
rustup component add clippy

# deps from apt
sudo apt-get update
sudo apt-get install unzip \
    zsh \
    gcc \
    python3-dev \
    python3-pip \
    python3-setuptools \
    fzf \
    libssl-dev \
    xsel \
    tmux \
    ca-certificates \
    curl \
    emacs \
    unzip \
    pkg-config \
    -y

# install sccache
which sccache || cargo install sccache

# spacemacs
[ -d $HOME/.emacs.d ] || git clone https://github.com/syl20bnr/spacemacs $HOME/.emacs.d

# exports
export RUSTC_WRAPPER=sccache 

# deps from cargo
cargo install exa ripgrep git-delta cargo-watch fd-find bat skim bottom topgrade

# install cargo plugins
cargo install cargo-watch cargo-sweep cargo-edit cargo-update

# docker
which docker || (curl -fsSL https://get.docker.com -o get-docker.sh \
&& sh get-docker.sh && rm get-docker.sh)

#antidote
[ -e ~/.antidote ] || git clone https://github.com/mattmc3/antidote.git ~/.antidote

# starship
sudo sh -c "$(curl -fsSL https://starship.rs/install.sh)" -- --force

# thefuck
which thefuck || pip3 install thefuck --user

# direnv
sudo curl -sfL https://direnv.net/install.sh | bash
mv $(which direnv) $HOME/.local/bin

# zoxide
sudo curl -sS https://webinstall.dev/zoxide | bash

# mcfly
sudo curl -LSfs https://raw.githubusercontent.com/cantino/mcfly/master/ci/install.sh | sh -s -- --git cantino/mcfly --to $HOME/.local/bin --force
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

# aws cli
which aws || curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip" && \
unzip awscliv2.zip && sudo ./aws/install && rm -rf awscliv2.zip aws

# cleanup
sudo apt-get autoremove -y

export PATH="$PATH:$HOME/.local/bin"
export PATH="$PATH:$HOME/.cargo/bin"
