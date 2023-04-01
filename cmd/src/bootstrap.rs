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
    "fd",
    "fzf",
    "git",
    "git-delta",
    "gnupg",
    "htop",
    "jq",
    "neovim",
    "ripgrep",
    "sccache",
    "starship",
    "sk",
    "tmux",
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

const LINUX_TOOLS: &[&str] = &[
    "libssl-dev",
    "xsel",
    "ca-certificates",
    "curl",
    "unzip",
    "gcc",
    "python3-dev",
    "python3-pip",
    "python3-setuptools",
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

    cmd!(sh, "chsh -s $(which zsh)").run()?;
    cmd!(sh, "rustup component add rustfmt clippy").run()?;

    match Os::current() {
        Os::Linux => {
            cmd!(sh, "sudo apt-get update").run()?;
            println!("{}", "installing linux tools".green());
            cmd!(sh, "sudo apt-get install").args(LINUX_TOOLS).run()?;

            let nix_tools = TOOLS
                .iter()
                .filter(|tool| !LINUX_TOOLS.contains(tool))
                .map(|tool| format!("nixpkgs.{tool}"))
                .collect::<Vec<_>>();
            println!("{}", "installing tools using nix".green());
            cmd!(sh, "nix-env -iA").args(nix_tools).run()?;

            println!("{}", "installing cargo plugins".green());
            cmd!(sh, "cargo install").args(CARGO_PLUGINS).run()?;
        }
        Os::MacOS => {
            let osx_defaults = OsxDefaults {}.render_once()?;
            create_and_run_file(sh, &osx_defaults, "osx_defaults.zsh")?;

            install_brew_and_tools(sh)?;
        }
    }

    // TODO: convert setup.sh file

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
    cmd!(sh, "brew install").args(TOOLS).run()?;

    println!("{}", "installing brew casks".green());
    cmd!(sh, "brew install --cask").args(BREW_CASKS).run()?;

    println!("{}", "installing cargo plugins".green());
    std::env::set_var("RUSTC_WRAPPER", "sccache");
    cmd!(sh, "cargo install").args(CARGO_PLUGINS).run()?;

    cmd!(sh, "brew cleanup").run()?;
    cmd!(sh, "brew autoremove").run()?;

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
