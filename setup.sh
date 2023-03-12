#!/bin/sh

set -e

export PATH="$PATH:$HOME/.local/bin"
export PATH="$PATH:$HOME/.cargo/bin"

# generate zsh plugins
[ -f $HOME/.zsh_plugins.sh ] && rm $HOME/.zsh_plugins.sh
antibody bundle < zsh_plugins.txt > zsh_plugins.sh

# make folders
mkdir -p ~/.config

# dotfiles directory
dir=~/code/dotfiles                    

# list of files/folders to symlink in homedir
files="zshrc gitconfig spacemacs zsh_plugins.sh gitignore direnvrc gitignore alacritty.yml tmux.conf"   
config_files="starship.toml zellij"

# change to the dotfiles directory
echo "Changing to the $dir directory"

cd $dir
echo "...done\n"

echo "Creating all symlinks \n"
for file in $files; do
    [ -f ~/.$file ] && rm ~/.$file

    echo "Creating symlink to $file in home directory."
    ln -s $dir/$file ~/.$file
done

echo "Creating Config Files and Dirs \n"
for file_or_dir in $config_file_or_dirs; do
    [ -f ~/.config/$file_or_dir ] && rm ~/.config/$file_or_dir

    echo "Creating symlink to $file_or_dir in config directory."
    ln -s $dir/config/$file_or_dir ~/.config/$file_or_dir
done

echo "Installing zsh plugins"
antibody update

if [ ! -d ~/.config/nvim ]; then
    echo "Installing neovim"
    rm -rf ~/.vim/
    git clone --depth 1 https://github.com/AstroNvim/AstroNvim ~/.config/nvim
    ln -s $dir/nvim ~/.config/nvim/user 
    nvim  --headless -c 'autocmd User LazyDone quitall'
fi

# restart zsh
exec zsh
