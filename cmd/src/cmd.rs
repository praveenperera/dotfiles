use std::path::PathBuf;

use crate::{bootstrap, Tool};
use eyre::{eyre, Result};
use xshell::Shell;

const TOOLS: &[Tool] = &[("bootstrap", bootstrap::run), ("config", bootstrap::config)];

pub fn run(_sh: &Shell) -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let program: PathBuf = args.first().cloned().unwrap_or_default().into();

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
