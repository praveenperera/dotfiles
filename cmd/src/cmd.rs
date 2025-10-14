pub mod bootstrap;
pub mod gcloud;
pub mod generate;
pub mod main_cmd;
pub mod secrets;
pub mod terraform;
pub mod vault;

use eyre::Result;
use log::debug;
use xshell::Shell;

use main_cmd::{Cmd, MainCmd};
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
            let bootstrap_flags = bootstrap::Bootstrap { mode };
            bootstrap::run_with_flags(&sh, bootstrap_flags)
        }

        MainCmd::Gcloud { subcommand } => {
            let gcloud_flags = gcloud::Gcloud { subcommand };
            gcloud::run_with_flags(&sh, gcloud_flags)
        }
        MainCmd::Secret { subcommand } => {
            let secret_flags = secrets::Secrets { subcommand };
            secrets::run_with_flags(&sh, secret_flags)
        }
        MainCmd::Terraform { subcommand } => {
            let terraform_flags = terraform::Terraform { subcommand };
            terraform::run_with_flags(&sh, terraform_flags)
        }
        MainCmd::Vault { subcommand } => {
            let vault_flags = vault::Vault { subcommand };
            vault::run_with_flags(&sh, vault_flags)
        }
        MainCmd::Generate { subcommand } => {
            let generate_flags = generate::Generate { subcommand };
            generate::run_with_flags(&sh, generate_flags)
        }
        MainCmd::PrContext {
            repo_or_url,
            pr_number,
            token,
            code_only,
            compact,
        } => {
            let args = crate::pr_context::Args {
                repo_or_url,
                pr_number,
                token,
                code_only,
                compact,
            };
            crate::pr_context::run_with_args(&sh, args)
        }
    }
}
