pub mod bootstrap;
pub mod cmd;
pub mod os;

use eyre::{eyre, Result};
use std::{env, path::PathBuf};
use xshell::Shell;

pub type Tool = (&'static str, fn(&Shell) -> Result<()>);
const TOOLS: &[Tool] = &[("cmd", cmd::run)];

fn tools_str() -> String {
    TOOLS
        .into_iter()
        .map(|(name, _run)| *name)
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn dotfiles_dir() -> PathBuf {
    let home = env::var("HOME").expect("HOME env var must be set");

    PathBuf::new().join(home).join("code/dotfiles")
}

pub fn command_exists(sh: &Shell, command: &str) -> bool {
    xshell::cmd!(sh, "command -v {command}").read().is_ok()
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let program: PathBuf = std::env::args_os().next().unwrap_or_default().into();
    let program = program
        .file_stem()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default();

    let (_name, run) = TOOLS
        .iter()
        .find(|&&(name, _run)| name == program)
        .ok_or_else(|| {
            eyre!(
                "unknown tool: `{program}`, possible values are {}",
                tools_str()
            )
        })?;

    let sh = Shell::new()?;
    run(&sh)
}
