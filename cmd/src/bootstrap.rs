use std::path::PathBuf;

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
    "exa",
    "pkg-config",
    "antibody",
    "zoxide",
    "kubectl",
    "gpg",
    "tree",
    "shellcheck",
    "elixir",
    "topgrade",
    "go",
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
    "font-jetbrains-mono-nerd-font",
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

const CARGO_PKGS: &[&str] = &[
    "cargo-watch",
    "cargo-sweep",
    "cargo-edit",
    "cargo-udeps",
    "zellij-runner",
    "bacon",
    "twm",
];

const DOTFILES: &[&str] = &[
    "zshrc",
    "gitconfig",
    "wezterm.lua",
    "zsh_plugins.sh",
    "gitignore",
    "direnvrc",
    "alacritty.yml",
];

const CONFIG_FILE_OR_DIR: &[&str] = &["starship.toml", "zellij", "twm"];

pub fn run(sh: &Shell) -> Result<()> {
    // install rust components
    cmd!(sh, "rustup component add rustfmt clippy rust-analyzer").run()?;

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
    config(sh)?;

    Ok(())
}

pub fn config(sh: &Shell) -> Result<()> {
    let path = crate::dotfiles_dir().join("zshrc");
    let zshrc = Zshrc { os: Os::current() };

    println!("writing zshrc to {}", path.display().to_string().green());
    sh.write_file(&path, zshrc.render_once()?)?;

    if let Os::MacOS = Os::current() {
        let osx_defaults = OsxDefaults {}.render_once()?;
        create_and_run_file(sh, &osx_defaults, "osx_defaults.zsh")?;
    }

    // setup dotfiles and config dirs
    setup_config_and_dotfiles(sh)?;

    Ok(())
}

fn setup_config_and_dotfiles(sh: &Shell) -> Result<()> {
    let home: PathBuf = std::env::var("HOME").expect("HOME env var not set").into();
    let zsh_plugins = home.join(".zsh_plugins.txt");

    // setup zsh plugins
    sh.remove_path(&zsh_plugins)?;
    println!("{}", "setting up zsh_plugins.sh file...".green());

    let zsh_plugins_txt = crate::dotfiles_dir().join("zsh_plugins.txt");
    let zsh_plugins_sh = crate::dotfiles_dir().join("zsh_plugins.sh");

    let input_content = sh.read_file(zsh_plugins_txt)?;
    let output_content = cmd!(sh, "antibody bundle").stdin(input_content).read()?;
    sh.write_file(zsh_plugins_sh, &output_content)?;

    let mut path_and_target = vec![];

    for filename in DOTFILES {
        let path = crate::dotfiles_dir().join(filename);
        let target = home.join(format!(".{filename}"));

        path_and_target.push((path, target));
    }

    for filename in CONFIG_FILE_OR_DIR {
        let path = crate::dotfiles_dir().join("config").join(filename);
        let target = home.join(format!(".config/{filename}"));

        path_and_target.push((path, target));
    }

    for (path, target) in path_and_target.iter() {
        sh.remove_path(target)?;
        cmd!(sh, "ln -s {path} {target}").run()?;
    }

    install_tpm(sh, &home)?;
    install_neovim(sh, &home)?;

    Ok(())
}

fn install_tpm(sh: &Shell, home: &PathBuf) -> Result<()> {
    let target = home.join(".tmux/plugins/tpm");

    if !sh.path_exists(&target) {
        println!("{}", "tmux package manager not foud".blue());
        println!("{}", "install tmux package manager (TPM)".green());

        cmd!(sh, "git clone https://github.com/tmux-plugins/tpm {target}").run()?;
    };

    Ok(())
}

fn install_neovim(sh: &Shell, home: &PathBuf) -> Result<()> {
    let target = home.join(".config/nvim");
    let path = crate::dotfiles_dir().join("nvim");

    if !sh.path_exists(&target) {
        println!("{}", "neovim config dir not found".blue());
        println!("{}", "setting up neovim".green());

        cmd!(
            sh,
            "git clone --depth 1 https://github.com/AstroNvim/AstroNvim {target}"
        )
        .run()?;

        cmd!(sh, "ln -s {path} {target}/user").run()?;
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
    cmd!(sh, "brew tap homebrew/cask-fonts").run()?;

    println!("{}", "installing brew tools".green());
    cmd!(sh, "brew install").args(TOOLS).run()?;

    println!("{}", "installing brew casks".green());
    cmd!(sh, "brew install --cask").args(BREW_CASKS).run()?;

    println!("{}", "installing cargo plugins".green());
    std::env::set_var("RUSTC_WRAPPER", "sccache");
    cmd!(sh, "cargo install").args(CARGO_PKGS).run()?;

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
