#!/bin/sh

# unset before installing sscache
unset RUSTC_WRAPPER

# Install Rust
cargo || curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sudo sh -s -- -y

# install sccache
which sccache || cargo install sccache

# exports
export RUSTC_WRAPPER=sccache 

# deps from brew
brew install \
    starship thefuck direnv mcfly fnm exa ripgrep git-delta \
    fd bat sk bottom antidote zoxide kubectl gpg fzf shellcheck elixir \
    topgrade pnpm antibody

# cask deps from brew
brew install \
    alacritty google-cloud-sdk visual-studio-code bettertouchtool \
    github signal sublime-text rectangle --cask

# install fonts from brew
brew tap homebrew/cask-fonts
brew install font-fira-code-nerd-font --cask

# install cargo plugins
cargo install cargo-watch cargo-sweep cargo-edit topgrade cargo-udeps
