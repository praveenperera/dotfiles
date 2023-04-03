#!/bin/sh

# exit on error
set -e

# unset RUSTC_WRAPPER if sccache is not installed
if [ ! -x "$(command -v sccache)" ]; then
    unset RUSTC_WRAPPER
fi

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

# Install nix
if [ ! -x "$(command -v nix-env)" ]; then
    curl -L https://nixos.org/nix/install | sh
fi

cd cmd
sudo ./release

cmd bootstrap
sudo chsh -s $(which zsh)
