pub mod cli;
pub mod clipboard;
pub mod codex;
pub mod config;
pub mod history;
pub mod realtime;
pub mod tmux;

use std::ffi::OsString;

use cli::{Cli, Command};
use color_eyre::eyre::{Result, WrapErr};
use xshell::Shell;

use crate::runtime;

pub fn run(sh: &Shell, args: &[OsString]) -> Result<()> {
    let cli = Cli::parse_args(args);

    if cli.debug {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_writer(std::io::stderr)
            .try_init()
            .ok();
    }

    runtime::block_on(run_async(sh, cli))?
}

async fn run_async(sh: &Shell, cli: Cli) -> Result<()> {
    let context = codex::thread_resolver::ThreadResolver::new(sh)?
        .resolve(cli.pane.as_deref(), cli.thread.as_deref())
        .wrap_err("Failed to resolve Codex context")?;

    match cli.command() {
        Command::Context => {
            println!("{}", context.format_human());
            Ok(())
        }
        Command::Read {
            selection,
            list,
            json,
        } => {
            let selector = codex::latest_message::LatestMessageSelector::new();
            if list {
                for (index, item) in selector
                    .list(&context)
                    .wrap_err("Failed to list readable Codex messages")?
                    .iter()
                    .enumerate()
                {
                    println!(
                        "{index}\t{kind:?}\t{turn}\t{preview}",
                        kind = item.kind,
                        turn = item.turn_id,
                        preview = codex::latest_message::preview(&item.text)
                    );
                }
                return Ok(());
            }

            let item = selector
                .select(&context, selection.item)
                .wrap_err("Failed to read latest Codex message")?;
            if json {
                println!("{}", serde_json::to_string_pretty(&item)?);
            } else {
                println!("{}", item.text);
            }
            Ok(())
        }
        Command::Ask { selection } => {
            let item = codex::latest_message::LatestMessageSelector::new()
                .select(&context, selection.item)
                .wrap_err("Failed to read latest Codex message")?;
            let prompt = realtime::session::VoiceSession::from_env()?
                .ask_with_readout(&item)
                .await?;
            clipboard::copy(&prompt)?;
            history::record_prompt(&prompt).ok();
            println!("{prompt}");
            Ok(())
        }
        Command::Prompt => {
            let prompt = realtime::session::VoiceSession::from_env()?
                .prompt_only(context.thread.id.as_str())
                .await?;
            clipboard::copy(&prompt)?;
            history::record_prompt(&prompt).ok();
            println!("{prompt}");
            Ok(())
        }
    }
}
