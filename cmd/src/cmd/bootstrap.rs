use std::path::{Path, PathBuf};

use eyre::{Context as _, Result};
use sailfish::TemplateOnce;
use xshell::{cmd, Shell};

use crate::{command_exists, os::Os, CMD_TOOLS};
use colored::Colorize;

#[derive(TemplateOnce)]
#[template(path = "zshrc.stpl")]
struct Zshrc {
    os: Os,
}

#[derive(TemplateOnce)]
#[template(path = "osx_defaults.zsh.stpl")]
struct OsxDefaults {}

const TOOLS: &[&str] = &[
    "bat",
    "coreutils",
    "diff-so-fancy",
    "difftastic",
    "fd",
    "fzf",
    "git",
    "git-delta",
    "gnupg",
    "direnv",
    "thefuck",
    "htop",
    "jq",
    "neovim",
    "ripgrep",
    "sccache",
    "starship",
    "sk",
    "tmux",
    "fnm",
    "bottom",
    "htop",
    "eza",
    "pkg-config",
    "zoxide",
    "kubectl",
    "gpg",
    "tree",
    "shellcheck",
    "elixir",
    "topgrade",
    "go",
    "atuin",
    "mcfly",
    "zsh",
    "neovim",
    "pinentry-mac",
    "just",
    "rust-analyzer",
];

const MAC_ONLY_TOOLS: &[&str] = &[
    "1password-cli",
    "xcode-build-server",
    "gpg-suite",
    "pinentry-mac",
];

const BREW_CASKS: &[&str] = &[
    "alacritty",
    "google-cloud-sdk",
    "visual-studio-code",
    "bettertouchtool",
    "github",
    "signal",
    "sublime-text",
    "raycast",
    "font-jetbrains-mono-nerd-font",
    "font-recursive-mono-nerd-font",
    "brave-browser",
    "appcleaner",
    "iterm2",
];

const LINUX_TOOLS: &[&str] = &[
    "libssl-dev",
    "pkg-config",
    "xsel",
    "ca-certificates",
    "curl",
    "unzip",
    "gcc",
    "python3-dev",
    "python3-pip",
    "python3-setuptools",
];

const CARGO_PKGS: &[&str] = &["cargo-watch", "bacon"];

const DOTFILES: &[&str] = &[
    "zshrc",
    "gitconfig",
    "wezterm.lua",
    "zsh_plugins.zsh",
    "gitignore",
    "direnvrc",
    "tmux.conf",
];

const CONFIG_FILE_OR_DIR: &[&str] = &["starship.toml", "zellij", "twm", "topgrade", "alacritty"];

const CUSTOM_CONFIG_OR_DIR: &[(&str, &str)] = &[("nvim", ".config/nvim")];
const MAC_ONLY_CUSTOM_CONFIG_OR_DIR: &[(&str, &str)] =
    &[("gpg-agent.conf", ".gnupg/gpg-agent.conf")];

pub fn run(sh: &Shell, args: &[&str]) -> Result<()> {
    // install rust components
    cmd!(sh, "rustup component add rustfmt clippy").run()?;

    match Os::current() {
        Os::Linux => {
            cmd!(sh, "sudo apt-get update").run()?;
            println!("{}", "installing linux tools".green());
            cmd!(sh, "sudo apt-get install")
                .args(LINUX_TOOLS)
                .arg("-y")
                .run()?;

            let nix_tools = TOOLS
                .iter()
                .filter(|tool| !LINUX_TOOLS.contains(tool))
                .map(|tool| map_brew_tool_names_to_nix(tool))
                .filter(|tool| !tool.is_empty())
                .map(|tool| format!("nixpkgs.{tool}"))
                .collect::<Vec<_>>();

            println!("{}", "installing tools using nix".green());
            cmd!(sh, "nix-env -iA").args(nix_tools).run()?;

            println!("{}", "installing cargo plugins".green());
            cmd!(sh, "cargo install").args(CARGO_PKGS).run()?;
        }
        Os::MacOS => {
            install_brew_and_tools(sh)?;
        }
    }

    // run config
    config(sh, args)?;

    Ok(())
}

pub fn config(sh: &Shell, _args: &[&str]) -> Result<()> {
    let path = crate::dotfiles_dir().join("zshrc");
    let zshrc = Zshrc { os: Os::current() };

    println!("writing zshrc to {}", path.display().to_string().green());
    sh.write_file(&path, zshrc.render_once()?)?;

    if let Os::MacOS = Os::current() {
        println!("{}", "installing osx defaults".green());
        let osx_defaults = OsxDefaults {}.render_once()?;
        create_and_run_file(sh, &osx_defaults, "osx_defaults.zsh")?;
    }

    // setup dotfiles and config dirs
    setup_config_and_dotfiles(sh)?;

    Ok(())
}

pub fn release(sh: &Shell, _: &[&str]) -> Result<()> {
    let home = std::env::var("HOME").expect("HOME env var not set");

    let current_path = std::env::current_exe().wrap_err("failed to get current path")?;
    let current_exe_rename = format!("{}.old", current_path.display());

    std::fs::rename(&current_path, &current_exe_rename)
        .wrap_err("failed to rename current binary")?;

    sh.change_dir(crate::dotfiles_dir());
    sh.change_dir("cmd");

    cmd!(sh, "./release").run()?;
    let cmd = format!("{home}/.local/bin/cmd");

    for (tool, _) in CMD_TOOLS {
        if *tool == "cmd" {
            continue;
        }

        let tool_path = format!("{home}/.local/bin/{tool}");

        if sh.path_exists(&tool_path) {
            sh.remove_path(&tool_path)?;
        }

        sh.hard_link(&cmd, tool_path)?;
    }

    sh.remove_path(&current_exe_rename)?;

    Ok(())
}

fn setup_config_and_dotfiles(sh: &Shell) -> Result<()> {
    let home: PathBuf = std::env::var("HOME").expect("HOME env var not set").into();

    // setup zsh plugins
    println!("{}", "setting up zsh_plugins.zsh file...".green());
    let antidote_script = include_str!("../../scripts/antidote.zsh");

    {
        let _dir = sh.push_env(
            "DOTFILES_DIR",
            crate::dotfiles_dir().to_str().expect("invalid path"),
        );

        cmd!(sh, "zsh -c {antidote_script}").quiet().run()?;
    }

    let mut path_and_target = vec![];

    for filename in DOTFILES {
        let path = crate::dotfiles_dir().join(filename);
        let target = home.join(format!(".{filename}"));

        path_and_target.push((path, target));
    }

    let config_dir = home.join(".config");
    if !sh.path_exists(&config_dir) {
        sh.create_dir(&config_dir)?;
    }

    for filename in CONFIG_FILE_OR_DIR {
        let path = crate::dotfiles_dir().join("config").join(filename);
        let target = home.join(format!(".config/{filename}"));

        path_and_target.push((path, target));
    }

    // mac only config
    if let Os::MacOS = Os::current() {
        for (src, dest) in MAC_ONLY_CUSTOM_CONFIG_OR_DIR {
            let path = crate::dotfiles_dir().join(src);
            let target = home.join(dest);

            path_and_target.push((path, target));
        }
    }

    for (src, dest) in CUSTOM_CONFIG_OR_DIR {
        let path = crate::dotfiles_dir().join(src);
        let target = home.join(dest);

        path_and_target.push((path, target));
    }

    for (path, target) in path_and_target.iter() {
        sh.remove_path(target)?;

        if let Some(parent) = PathBuf::from(target).parent() {
            if !sh.path_exists(parent) {
                cmd!(sh, "mkdir -p {parent}").quiet().run()?;
            }
        }

        cmd!(sh, "ln -s {path} {target}").run()?;
    }

    install_tpm(sh, &home)?;

    Ok(())
}

fn install_tpm(sh: &Shell, home: &Path) -> Result<()> {
    let target = home.join(".tmux/plugins/tpm");

    if !sh.path_exists(&target) {
        println!("{}", "tmux package manager not foud".blue());
        println!("{}", "install tmux package manager (TPM)".green());

        cmd!(sh, "git clone https://github.com/tmux-plugins/tpm {target}").run()?;
    };

    Ok(())
}

fn install_brew_and_tools(sh: &Shell) -> Result<()> {
    if !command_exists(sh, "brew") {
        println!("{} {}", "brew not found".blue(), "installing...".green());

        cmd!(sh, "/bin/bash -c '$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)'")
        .run()?;
    }

    cmd!(sh, "brew update").run()?;

    println!("{}", "installing brew tools".green());
    cmd!(sh, "brew install").args(TOOLS).run()?;
    cmd!(sh, "brew install").args(MAC_ONLY_TOOLS).run()?;

    let cask_list = cmd!(sh, "brew list --cask").read().unwrap_or_default();

    let already_installed_casks = cask_list
        .split('\n')
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    let casks_to_install = BREW_CASKS
        .iter()
        .filter(|cask| !already_installed_casks.contains(cask))
        .cloned()
        .collect::<Vec<_>>();

    if !casks_to_install.is_empty() {
        println!(
            "{}: {}",
            "installing brew casks".green(),
            casks_to_install.join(", ").blue()
        );

        cmd!(sh, "brew install --cask")
            .args(casks_to_install)
            .run()?;
    }

    println!("{}", "installing cargo plugins".green());
    std::env::set_var("RUSTC_WRAPPER", "sccache");

    // install cargo-binstall
    cmd!(sh, "cargo install cargo-binstall").run()?;

    // install cargo packages  using cargo-bininstall
    cmd!(sh, "cargo binstall")
        .args(CARGO_PKGS)
        .arg("-y")
        .run()?;

    cmd!(sh, "brew cleanup").run()?;
    cmd!(sh, "brew autoremove").run()?;

    Ok(())
}

fn map_brew_tool_names_to_nix(tool_name: &str) -> &str {
    match tool_name {
        "git-delta" => "delta",
        "sk" => "skim",
        "gpg" => "gnupg",
        other => other,
    }
}

fn create_and_run_file(sh: &Shell, contents: &str, file: &str) -> Result<()> {
    let tmp_dir = sh.create_temp_dir()?;
    let tmp_path = tmp_dir.path().join(file);
    sh.write_file(&tmp_path, contents)?;

    println!("running {}", file.green());
    cmd!(sh, "zsh {tmp_path}").quiet().run()?;

    Ok(())
}
