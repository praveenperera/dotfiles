use colored::Colorize;
use eyre::Result;
use xshell::{cmd, Shell};

use crate::{command_exists, fsutil};

use super::{
    BootstrapMode, BREW_CASKS, CARGO_PKGS, LINUX_TOOLS_FULL, LINUX_TOOLS_MINIMAL, MAC_ONLY_TOOLS,
    TOOLS_FULL, TOOLS_MINIMAL, TOOLS_VIA_SHELL_SCRIPT,
};

pub(super) fn install_linux_tools(sh: &Shell, mode: BootstrapMode) -> Result<()> {
    cmd!(sh, "sudo apt-get update").run()?;
    println!("{}", "installing linux tools".green());

    match mode {
        BootstrapMode::Minimal => {
            cmd!(sh, "sudo apt-get install")
                .args(LINUX_TOOLS_MINIMAL)
                .args(TOOLS_MINIMAL)
                .arg("-y")
                .run()?;

            let home = fsutil::home_dir()?;
            let bat = home.join(".local/bin/bat");
            cmd!(sh, "ln -s /usr/bin/batcat {bat}").run()?;

            for (url, tool, args) in TOOLS_VIA_SHELL_SCRIPT {
                install_via_shell_script(sh, url, tool, args)?;
            }
        }
        BootstrapMode::Full => {
            cmd!(sh, "sudo apt-get install")
                .args(LINUX_TOOLS_FULL)
                .arg("-y")
                .run()?;

            let nix_tools = TOOLS_FULL
                .iter()
                .map(|tool| map_brew_tool_names_to_nix(tool))
                .filter(|tool| !tool.is_empty())
                .map(|tool| format!("nixpkgs.{tool}"))
                .collect::<Vec<_>>();

            println!("{}", "installing tools using nix".green());
            cmd!(sh, "nix-env -iA").args(nix_tools).run()?;
            println!("{}", "installing cargo plugins".green());
            cmd!(sh, "cargo binstall").args(CARGO_PKGS).run()?;
        }
    }

    Ok(())
}

/// mac only tools
pub(super) fn install_brew_and_tools(sh: &Shell) -> Result<()> {
    if !command_exists(sh, "brew") {
        println!("{} {}", "brew not found".blue(), "installing...".green());

        cmd!(sh, "/bin/bash -c '$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)'")
            .run()?;
    }

    cmd!(sh, "brew update").run()?;

    println!("{}", "installing brew tools".green());
    cmd!(sh, "brew install").args(TOOLS_FULL).run()?;
    cmd!(sh, "brew install").args(MAC_ONLY_TOOLS).run()?;

    let cask_list = cmd!(sh, "brew list --cask").read().unwrap_or_default();

    let already_installed_casks = cask_list
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    let casks_to_install = BREW_CASKS
        .iter()
        .filter(|cask| !already_installed_casks.contains(cask))
        .copied()
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
    cmd!(sh, "cargo install cargo-binstall").run()?;
    cmd!(sh, "cargo binstall")
        .args(CARGO_PKGS)
        .arg("-y")
        .run()?;
    cmd!(sh, "brew cleanup").run()?;
    cmd!(sh, "brew autoremove").run()?;

    Ok(())
}

pub(super) fn map_brew_tool_names_to_nix(tool_name: &str) -> &str {
    match tool_name {
        "git-delta" => "delta",
        "sk" => "skim",
        "gpg" => "gnupg",
        other => other,
    }
}

pub(super) fn install_via_shell_script(
    sh: &Shell,
    url: &str,
    tool_name: &str,
    args: &[&str],
) -> Result<()> {
    println!(
        "{} {}",
        format!("installing {tool_name}").green(),
        "via shell script".blue()
    );

    let tmp_dir = sh.create_temp_dir()?;
    let script_path = tmp_dir.path().join(format!("{tool_name}_install.sh"));

    cmd!(
        sh,
        "curl --proto '=https' --tlsv1.2 -LsSf {url} -o {script_path}"
    )
    .run()?;
    cmd!(sh, "chmod +x {script_path}").run()?;

    let args = args.join(" ");
    cmd!(sh, "sh {script_path} {args}").run()?;

    Ok(())
}
