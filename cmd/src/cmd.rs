use std::path::PathBuf;

use crate::{bootstrap, gcloud, secrets, Tool};
use eyre::{eyre, Result};
use xshell::Shell;

const TOOLS: &[Tool] = &[
    ("bootstrap", bootstrap::run),
    ("config", bootstrap::config),
    ("cfg", bootstrap::config),
    ("switch-gcloud", gcloud::switch),
    ("release", bootstrap::release),
    ("secrets", secrets::run),
];

fn tools_str() -> String {
    TOOLS
        .iter()
        .map(|(name, _run)| *name)
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn run(_sh: &Shell, args: &[&str]) -> Result<()> {
    let program: PathBuf = args.first().cloned().unwrap_or_default().into();

    let program = program
        .file_stem()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default();

    let (_name, tool_run) = TOOLS
        .iter()
        .find(|&&(name, _run)| name == program)
        .ok_or_else(|| {
            eyre!(
                "unknown tool: `{program}`, possible values are: {}",
                tools_str()
            )
        })?;

    let sh = Shell::new()?;
    tool_run(&sh, &args[1..])
}
