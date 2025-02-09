pub mod bootstrap;
pub mod gcloud;
pub mod generate;
pub mod secrets;
pub mod terraform;
pub mod vault;

use std::path::PathBuf;

use crate::Tool;
use colored::Colorize as _;
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
    // generate
    ("gen", generate::run),
    ("generate", generate::run),
];

fn tools_str() -> String {
    let mut tools = TOOLS.iter().map(|(name, _run)| *name).collect::<Vec<_>>();

    tools.sort_unstable();
    tools.join(", ")
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
            let did_you_mean = did_you_mean(program).join(", ");
            println!(
                "unknown tool: `{}`, did you mean one of: {}",
                program.red(),
                did_you_mean.green()
            );
            println!("all tools: {}", tools_str().yellow());
            eyre!("unknown tool: {program}")
        })?;

    let sh = Shell::new()?;
    tool_run(&sh, &args[1..])
}

fn did_you_mean(user_text: &str) -> Vec<&str> {
    use textdistance::nstr::damerau_levenshtein;

    let mut suggestions = TOOLS
        .iter()
        .map(|(name, _run)| name)
        .filter(|name| !name.starts_with(user_text))
        .map(|name| (*name, damerau_levenshtein(user_text, name)))
        .map(|(name, distance)| (name, distance * 100.0))
        .map(|(name, distance)| (name, distance as usize))
        .filter(|(_, distance)| *distance <= 90)
        .collect::<Vec<_>>();

    suggestions.sort_unstable_by(|a, b| a.1.cmp(&b.1));

    let starts_with = TOOLS
        .iter()
        .map(|(name, _run)| *name)
        .filter(|name| name.starts_with(user_text));

    let suggestions = suggestions.into_iter().map(|(name, _)| name).take(3);

    starts_with.into_iter().chain(suggestions).collect()
}
