pub mod bootstrap;
pub mod flags;
pub mod gcloud;
pub mod generate;
pub mod secrets;
pub mod terraform;
pub mod vault;

use eyre::Result;
use log::debug;
use xshell::Shell;

use std::ffi::OsString;
use crate::util::handle_xflags_error;
use flags::{Cmd, CmdCmd};


pub fn run(_sh: &Shell, args: &[OsString]) -> Result<()> {
    debug!("cmd run args: {args:?}");

    let flags = handle_xflags_error(Cmd::from_vec(args.to_vec()), args, Cmd::help())?;

    if flags.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let sh = Shell::new()?;
    match flags.subcommand {
        CmdCmd::Bootstrap(cmd) => bootstrap::run(
            &sh,
            &cmd.args.iter().map(|s| OsString::from(s)).collect::<Vec<_>>(),
        ),
        CmdCmd::Release(cmd) => bootstrap::release(
            &sh,
            &cmd.args.iter().map(|s| OsString::from(s)).collect::<Vec<_>>(),
        ),
        CmdCmd::Config(cmd) => bootstrap::config(
            &sh,
            &cmd.args.iter().map(|s| OsString::from(s)).collect::<Vec<_>>(),
        ),
        CmdCmd::Gcloud(cmd) => gcloud::run(
            &sh,
            &cmd.args.iter().map(|s| OsString::from(s)).collect::<Vec<_>>(),
        ),
        CmdCmd::Secret(cmd) => secrets::run(
            &sh,
            &cmd.args.iter().map(|s| OsString::from(s)).collect::<Vec<_>>(),
        ),
        CmdCmd::Terraform(cmd) => terraform::run(
            &sh,
            &cmd.args.iter().map(|s| OsString::from(s)).collect::<Vec<_>>(),
        ),
        CmdCmd::Vault(cmd) => vault::run(
            &sh,
            &cmd.args.iter().map(|s| OsString::from(s)).collect::<Vec<_>>(),
        ),
        CmdCmd::Generate(cmd) => generate::run(
            &sh,
            &cmd.args.iter().map(|s| OsString::from(s)).collect::<Vec<_>>(),
        ),
    }
}
