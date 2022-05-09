#!/bin/sh

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

config_files="starship.toml"

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

echo "Creating Config Files \n"
for file in $config_files; do
    [ -f ~/.config/$file ] && rm ~/.config/$file

    echo "Creating symlink to $file in config directory."
    ln -s $dir/config/$file ~/.config/$file
done

if [ ! -d ~/.emacs.d ]
then
    echo "Installing spacemacs"
    git clone https://github.com/syl20bnr/spacemacs ~/.emacs.d
else
    cd ~/.emacs.d 
    git pull
    cd -
fi

echo "Installing zsh plugins"
antibody update

if [ ! -d  ~/.vim/.SpaceVim.d ]
then
echo "Installing spacevim"
rm -rf ~/.vim/
curl -sLf https://spacevim.org/install.sh | bash
fi

# restart zsh
exec zsh
