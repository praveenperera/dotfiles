#!/bin/bash

# generate zsh plugins
antibody bundle < zsh_plugins.txt > zsh_plugins.sh
rm $HOME/.zsh_plugins.sh

# make folders
mkdir -p ~/.config

# dotfiles directory
dir=~/code/dotfiles                    

# list of files/folders to symlink in homedir
files="vimrc gvimrc.after zshrc gitconfig spacemacs zsh_plugins.sh gitignore direnvrc gitignore"   

config_files="starship.toml"

# change to the dotfiles directory
echo "Changing to the $dir directory"

cd $dir
echo "...done\n"

echo "Creating all symlinks \n"
for file in $files; do
    echo "Creating symlink to $file in home directory."
    ln -s $dir/$file ~/.$file
done

echo "Creating Config Files \n"
for file in $config_files; do
    echo "Creating symlink to $file in config directory."
    ln -s $dir/config/$file ~/.config/$file
done

antibody update
