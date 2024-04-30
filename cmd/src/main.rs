pub mod cmd;
pub mod encrypt;
pub mod os;
pub mod util;

use cmd::terraform;
use eyre::{eyre, Result};
use include_dir::{include_dir, Dir};
use std::{env, path::PathBuf};
use xshell::Shell;

pub type Tool = (&'static str, fn(&Shell, &[&str]) -> Result<()>);
pub const CMD_TOOLS: &[Tool] = &[("cmd", cmd::run), ("tf", terraform::run)];

pub static SECRETS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/secrets");

fn tools_str() -> String {
    CMD_TOOLS
        .iter()
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

    let args = std::env::args_os()
        .map(|x| x.into_string().unwrap_or_default())
        .collect::<Vec<_>>();

    let mut args_iter = args.iter();

    let program: PathBuf = args_iter.next().expect("not enough args").into();
    let program = program
        .file_stem()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default();

    let (_name, run) = CMD_TOOLS
        .iter()
        .find(|&&(name, _run)| name == program)
        .ok_or_else(|| {
            eyre!(
                "unknown tool: `{program}`, possible values are {}",
                tools_str()
            )
        })?;

    let args_vec = args_iter.map(String::as_str).collect::<Vec<_>>();

    let sh = Shell::new()?;
    run(&sh, &args_vec[..])
}
