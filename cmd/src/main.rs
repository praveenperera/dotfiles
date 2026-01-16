pub mod cmd;
pub mod encrypt;
pub mod github;
pub mod os;
pub mod pr_context;
pub mod util;

use cmd::{jj, terraform, vault};
use eyre::{eyre, Result};
use include_dir::{include_dir, Dir};
use log::debug;
use std::{env, ffi::OsString, path::PathBuf};
use xshell::Shell;

pub type Tool = (&'static str, fn(&Shell, &[OsString]) -> Result<()>);
pub const CMD_TOOLS: &[Tool] = &[
    ("cmd", cmd::run),
    ("jju", jj::run),
    ("notf", cmd::tmux::notify_run),
    ("pr-context", pr_context::run),
    ("prc", pr_context::run),
    ("tf", terraform::run),
    ("vault", vault::run),
];

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
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = std::env::args_os().collect::<Vec<_>>();

    debug!("run args: {args:?}");

    // check for --version flag
    if args.len() > 1 && args[1] == "--version" {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let program: PathBuf = args.first().expect("not enough args").into();
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

    let sh = Shell::new()?;

    let args = match program {
        "cmd" => &args[1..],
        _ => &args,
    };

    run(&sh, args)?;
    Ok(())
}
