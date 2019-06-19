#!/bin/bash

# dotfiles directory
dir=~/code/dotfiles                    

# list of files/folders to symlink in homedir
files="vimrc zsh_profile zshrc-e gvimrc.after zshrc zpreztorc zlogin zlogout zprofile zshenv gitconfig"    

# change to the dotfiles directory
echo "Changing to the $dir directory"
cd $dir
echo "...done\n"

echo "Creating all symlinks \n"
# move any existing dotfiles in homedir to dotfiles_old directory, then create symlinks
for file in $files; do
    echo "Creating symlink to $file in home directory."
    ln -s $dir/$file ~/.$file
done

echo "\nAdding zprezto custom theme"
cp prompt_praveen_setup ~/.zprezto/modules/prompt/functions
