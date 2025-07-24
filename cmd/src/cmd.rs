pub mod bootstrap;
pub mod flags;
pub mod gcloud;
pub mod generate;
pub mod secrets;
pub mod terraform;
pub mod vault;

use colored::Colorize as _;
use eyre::{eyre, Result};
use log::debug;
use xshell::Shell;

use crate::util::did_you_mean;
use flags::{Cmd, CmdCmd};


pub fn run(_sh: &Shell, args: &[&str]) -> Result<()> {
    debug!("cmd run args: {args:?}");

    // convert args to Vec<OsString> for xflags parsing
    let os_args = args
        .iter()
        .map(|s| std::ffi::OsString::from(*s))
        .collect::<Vec<_>>();

    let flags = match Cmd::from_vec(os_args) {
        Ok(flags) => flags,
        Err(_err) => {
            let unknown_cmd = crate::util::extract_unknown_command_from_args(args);
            match unknown_cmd.as_deref() {
                Some("help" | "-h" | "--help") => {
                    println!("{}", Cmd::help());
                    return Ok(());
                }

                Some(unknown_cmd) => {
                    let suggestions = did_you_mean(unknown_cmd, Cmd::help());
                    if !suggestions.is_empty() {
                        println!("\ndid you mean: {}\n", suggestions.join(", ").yellow());
                    }

                    println!("{}", Cmd::help());
                    return Err(eyre!("failed to parse arguments"));
                }
                None => {
                    println!("{}", Cmd::help());
                    return Err(eyre!("failed to parse arguments"));
                }
            }
        }
    };

    if flags.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let sh = Shell::new()?;
    match flags.subcommand {
        CmdCmd::Bootstrap(cmd) => bootstrap::run(
            &sh,
            &cmd.args.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        ),
        CmdCmd::Release(cmd) => bootstrap::release(
            &sh,
            &cmd.args.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        ),
        CmdCmd::Config(cmd) => bootstrap::config(
            &sh,
            &cmd.args.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        ),
        CmdCmd::Gcloud(cmd) => {
            let mut args = Vec::new();
            match &cmd.subcommand {
                flags::GcloudCmd::Login(login_cmd) => {
                    args.push("login");
                    args.extend(login_cmd.args.iter().map(|s| s.as_str()));
                }
                flags::GcloudCmd::SwitchProject(sp_cmd) => {
                    args.push("switch-project");
                    args.extend(sp_cmd.args.iter().map(|s| s.as_str()));
                }
                flags::GcloudCmd::SwitchCluster(sc_cmd) => {
                    args.push("switch-cluster");
                    args.extend(sc_cmd.args.iter().map(|s| s.as_str()));
                }
            }
            gcloud::run(&sh, &args)
        }
        CmdCmd::Secret(cmd) => {
            let mut args = Vec::new();
            match &cmd.subcommand {
                flags::SecretCmd::Gen(gen_cmd) => {
                    args.push("gen");
                    args.extend(gen_cmd.args.iter().map(|s| s.as_str()));
                }
                flags::SecretCmd::Get(get_cmd) => {
                    args.push("get");
                    args.extend(get_cmd.args.iter().map(|s| s.as_str()));
                }
                flags::SecretCmd::Save(save_cmd) => {
                    args.push("save");
                    args.extend(save_cmd.args.iter().map(|s| s.as_str()));
                }
                flags::SecretCmd::Update(update_cmd) => {
                    args.push("update");
                    args.extend(update_cmd.args.iter().map(|s| s.as_str()));
                }
            }
            secrets::run(&sh, &args)
        }
        CmdCmd::Terraform(cmd) => terraform::run(
            &sh,
            &cmd.args.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        ),
        CmdCmd::Vault(cmd) => vault::run(
            &sh,
            &cmd.args.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        ),
        CmdCmd::Generate(cmd) => generate::run(
            &sh,
            &cmd.args.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        ),
    }
}
