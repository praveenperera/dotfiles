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
    libssl-dev \
    xsel \
    tmux \
    ca-certificates \
    curl \
    golang \
    emacs \
    unzip \
    pkg-config \
    -y

# install sccache
which sccache || cargo install sccache

# install nix
curl -L https://nixos.org/nix/install | sh

# install nix pkgs
nix-env -iA \
    nixpkgs.fzf \
    nixpkgs.bat \
    nixpkgs.exa \
    nixpkgs.ripgrep \
    nixpkgs.delta \
    nixpkgs.cargo-watch \
    nixpkgs.bat \
    nixpkgs.skim \
    nixpkgs.fd \
    nixpkgs.bottom \
    nixpkgs.cargo-sweep \
    nixpkgs.cargo-watch \
    nixpkgs.cargo-update \
    nixpkgs.topgrade \
    nixpkgs.docker \
    nixpkgs.direnv \
    nixpkgs.zoxide \
    nixpkgs.fnm \
    nixpkgs.mcfly \
    nixpkgs.kubectl \
    nixpkgs.awscli \
    nixpkgs.antibody \
    nixpkgs.starship 


# spacemacs
[ -d $HOME/.emacs.d ] || git clone https://github.com/syl20bnr/spacemacs $HOME/.emacs.d

# exports
export RUSTC_WRAPPER=sccache 

# antidote
[ -e ~/.antidote ] || git clone https://github.com/mattmc3/antidote.git ~/.antidote

# thefuck
which thefuck || pip3 install thefuck --user

# gcloud cli
if [ ! -x "$(command -v gcloud)" ]; then
sudo curl https://sdk.cloud.google.com | bash -s -- --disable-prompts
fi

# cleanup
sudo apt-get autoremove -y

export PATH="$PATH:$HOME/.local/bin"
export PATH="$PATH:$HOME/.cargo/bin"
