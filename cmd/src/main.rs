mod cmd;

use eyre::{eyre, Result};
use std::path::PathBuf;
use xshell::Shell;

const TOOLS: &[(&str, fn(&Shell) -> Result<()>)] = &[("cmd", cmd::run)];

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
        .ok_or_else(|| eyre!("unknown tool: `{program}`"))?;

    let sh = Shell::new()?;
    run(&sh)
}
