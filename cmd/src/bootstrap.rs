use eyre::Result;
use sailfish::TemplateOnce;
use xshell::{cmd, Shell};

use crate::{command_exists, os::Os};
use colored::Colorize;

#[derive(TemplateOnce)]
#[template(path = "zshrc.stpl")]
struct Zshrc {
    os: Os,
}

#[derive(TemplateOnce)]
#[template(path = "osx_defaults.zsh.stpl")]
struct OsxDefaults {}

const BREW_TOOLS: &[&str] = &[
    "bat",
    "coreutils",
    "diff-so-fancy",
    "fd",
    "fzf",
    "git",
    "git-delta",
    "gnupg",
    "htop",
    "jq",
    "neovim",
    "ripgrep",
    "starship",
    "sk",
    "tmux",
    "bottom",
    "htop",
    "antibody",
    "zoxide",
    "kubectl",
    "gpg",
    "tree",
    "shellcheck",
    "elixir",
    "topgrade",
    "pnpm",
    "antibody",
    "zsh",
];

const BREW_CASKS: &[&str] = &[
    "alacritty",
    "google-cloud-sdk",
    "visual-studio-code",
    "bettertouchtool",
    "github",
    "signal",
    "sublime-text",
    "rectangle",
    "font-fira-code-nerd-font",
];

const CARGO_PLUGINS: &[&str] = &[
    "cargo-watch",
    "cargo-sweep",
    "cargo-edit",
    "cargo-udeps",
    "zellij-runner",
];

pub fn run(sh: &Shell) -> Result<()> {
    let path = crate::dotfiles_dir().join("zshrc");
    let zshrc = Zshrc { os: Os::current() };

    println!("writing zshrc to {}", path.display().to_string().green());
    sh.write_file(&path, zshrc.render_once()?)?;

    match Os::current() {
        Os::Linux => {}
        Os::MacOS => {
            let osx_defaults = OsxDefaults {}.render_once()?;
            create_and_run_file(sh, &osx_defaults, "osx_defaults.zsh")?;

            install_brew_and_tools(sh)?;
        }
    }

    Ok(())
}

fn install_brew_and_tools(sh: &Shell) -> Result<()> {
    if !command_exists(sh, "brew") {
        println!("{} {}", "brew not found".red(), "installing...".green());

        cmd!(sh, "/bin/bash -c '$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)'")
        .run()?;
    }

    cmd!(sh, "brew update").run()?;
    cmd!(sh, "brew tap homebrew/cask-fonts").run()?;

    println!("{}", "installing brew tools".green());
    cmd!(sh, "brew install").args(BREW_TOOLS).run()?;

    println!("{}", "installing brew casks".green());
    cmd!(sh, "brew install --cask").args(BREW_CASKS).run()?;

    println!("{}", "installing cargo plugins".green());
    cmd!(sh, "cargo install").args(CARGO_PLUGINS).run()?;

    Ok(())
}

fn create_and_run_file(sh: &Shell, contents: &str, file: &str) -> Result<()> {
    let tmp_dir = sh.create_temp_dir()?;
    let tmp_path = tmp_dir.path().join(file);
    sh.write_file(&tmp_path, contents)?;

    println!("running {}", file.green());
    cmd!(sh, "zsh {tmp_path}").quiet().run()?;

    Ok(())
}
