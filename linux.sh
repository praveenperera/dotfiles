#!/bin/sh

# unset before installing sscache
unset RUSTC_WRAPPER

# install sscache
cargo install sscache

# exports
export RUSTC_WRAPPER=sccache 

# deps from cargo
cargo install exa ripgrep

# install cargo plugins
cargo install cargo-watch

# deps from apt
apt update
apt install unzip \
    zsh \
    python3-dev \
    python3-pip \
    python3-setuptools \
    tmux \
    ca-certificates \
    curl \
    gnupg \
    lsb-release -y

# docker
curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh

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

# install kubectl
curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"
sudo install -o root -g root -m 0755 kubectl /usr/local/bin/kubectl

# gcloud cli
curl https://sdk.cloud.google.com | bash

# change shell to ZSH
chsh -s /bin/zsh
