pub mod better_context;
pub mod bootstrap;
pub mod crate_versions;
pub mod gcloud;
pub mod generate;
pub mod main_cmd;
pub mod secrets;
pub mod terraform;
pub mod tmux;
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
        MainCmd::Release { project } => bootstrap::release(&sh, project),
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
        MainCmd::Tmux { subcommand } => {
            let tmux_flags = tmux::Tmux { subcommand };
            tmux::run_with_flags(&sh, tmux_flags)
        }
        MainCmd::PrContext(args) => crate::pr_context::run_with_flags(&sh, args),
        MainCmd::BetterContext(args) => {
            let flags = better_context::BetterContext {
                repo: args.repo,
                fresh: args.fresh,
                r#ref: args.r#ref,
                full: args.full,
                quiet: args.quiet,
            };
            better_context::run_with_flags(&sh, flags)
        }
        MainCmd::Crate { subcommand } => match subcommand {
            main_cmd::CrateCmd::Versions(flags) => crate_versions::run_with_flags(&sh, flags),
        },
    }
}
