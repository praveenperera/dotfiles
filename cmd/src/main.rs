pub mod bootstrap;
pub mod cmd;
pub mod os;

use eyre::{eyre, Result};
use std::{env, path::PathBuf};
use xshell::Shell;

const TOOLS: &[(&str, fn(&Shell) -> Result<()>)] = &[("cmd", cmd::run)];

pub fn dotfiles_dir() -> PathBuf {
    let home = env::var("HOME").expect("HOME env var must be set");

    PathBuf::new().join(home).join("code/dotfiles")
}

fn logging_setup() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }
    env_logger::init();
}

fn main() -> Result<()> {
    color_eyre::install()?;
    logging_setup();

    let program: PathBuf = std::env::args_os().next().unwrap_or_default().into();
    let program = program
        .file_stem()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default();

    let (_name, run) = TOOLS
        .iter()
        .find(|&&(name, _run)| name == program)
        .ok_or_else(|| eyre!("unknown tool: `{program}`"))?;

    let sh = Shell::new()?;
    run(&sh)
}
