#!/bin/bash

dir=~/code/dotfiles                    # dotfiles directory
olddir=~/code/dotfiles_old            # old dotfiles backup directory
files="vimrc zsh_profile zshrc-e gvimrc.after zshrc zpreztorc zlogin zlogout zprofile zshenv"    # list of files/folders to symlink in homedir

##########
# change to the dotfiles directory
echo "Changing to the $dir directory"
cd $dir
echo "...done"

echo "Creating all symlinks"
# move any existing dotfiles in homedir to dotfiles_old directory, then create symlinks
for file in $files; do
    echo "Moving any existing dotfiles from ~ to $olddir"
    mv ~/.$file $olddir
    echo "Creating symlink to $file in home directory."
    ln -s $dir/$file ~/.$file
done

echo "Adding zprezto custom theme"
cp prompt_praveen_setup ~/.zprezto/modules/prompts
