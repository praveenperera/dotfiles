pub mod bootstrap;
pub mod gcloud;
pub mod secrets;
pub mod terraform;
pub mod vault;

use std::path::PathBuf;

use crate::Tool;
use eyre::{eyre, Result};
use log::debug;
use xshell::Shell;

const TOOLS: &[Tool] = &[
    ("bootstrap", bootstrap::run),
    ("release", bootstrap::release),
    // config
    ("config", bootstrap::config),
    ("cfg", bootstrap::config),
    // gcloud login
    ("gl", gcloud::login),
    ("gcloud-login", gcloud::login),
    // gcloud switch project
    ("gcloud-switch-project", gcloud::switch_project),
    ("gsp", gcloud::switch_project),
    // gcloud switch cluster
    ("gcloud-switch-cluster", gcloud::switch_cluster),
    ("gsc", gcloud::switch_cluster),
    // secret-gen
    ("secret-gen", secrets::gen),
    ("secret-generate", secrets::gen),
    ("sgen", secrets::gen),
    // secrets get
    ("secret-get", secrets::get),
    ("sg", secrets::get),
    // secrets save
    ("secret-save", secrets::save),
    ("ss", secrets::save),
    // secrets update
    ("secret-update", secrets::update),
    ("su", secrets::update),
    // terraform
    ("tf", terraform::run),
    ("terraform", terraform::run),
    // vault
    ("vault", vault::run),
];

fn tools_str() -> String {
    TOOLS
        .iter()
        .map(|(name, _run)| *name)
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn run(_sh: &Shell, args: &[&str]) -> Result<()> {
    debug!("cmd run args: {args:?}");
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
