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

use flags::{Cmd, MainCmd};
use std::ffi::OsString;

pub fn run(_sh: &Shell, args: &[OsString]) -> Result<()> {
    debug!("cmd run args: {args:?}");

    let flags = Cmd::from_args(args)?;
    if flags.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let sh = Shell::new()?;
    match flags.subcommand {
        MainCmd::Release => bootstrap::release(&sh),
        MainCmd::Config => bootstrap::config(&sh),

        MainCmd::Bootstrap { mode } => {
            let bootstrap_flags = crate::cmd::bootstrap::flags::Bootstrap { mode };
            bootstrap::run_with_flags(&sh, bootstrap_flags)
        }

        MainCmd::Gcloud { subcommand } => {
            let gcloud_flags = crate::cmd::gcloud::flags::Gcloud { subcommand };
            gcloud::run_with_flags(&sh, gcloud_flags)
        }
        MainCmd::Secret { subcommand } => {
            let secret_flags = crate::cmd::secrets::flags::Secrets { subcommand };
            secrets::run_with_flags(&sh, secret_flags)
        }
        MainCmd::Terraform { subcommand } => {
            let terraform_flags = crate::cmd::terraform::flags::Terraform { subcommand };
            terraform::run_with_flags(&sh, terraform_flags)
        }
        MainCmd::Vault { subcommand } => {
            let vault_flags = crate::cmd::vault::flags::Vault { subcommand };
            vault::run_with_flags(&sh, vault_flags)
        }
        MainCmd::Generate { subcommand } => {
            let generate_flags = crate::cmd::generate::flags::Generate { subcommand };
            generate::run_with_flags(&sh, generate_flags)
        }
    }
}
