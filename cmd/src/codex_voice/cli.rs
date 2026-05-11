use std::ffi::OsString;

use clap::{Parser, Subcommand};

#[derive(Debug, Clone, Parser)]
#[command(
    name = "codex-voice",
    about = "Discuss the latest Codex response by voice"
)]
pub struct Cli {
    /// Tmux pane target, usually "#{pane_id}" from a binding
    #[arg(long)]
    pub pane: Option<String>,

    /// Codex thread id to read instead of resolving from tmux
    #[arg(long)]
    pub thread: Option<String>,

    /// Print diagnostic logs to stderr
    #[arg(long)]
    pub debug: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Print detected tmux and Codex thread context
    Context,
    /// Print the selected Codex message without starting audio
    Read {
        #[command(flatten)]
        selection: MessageSelection,

        /// List readable Codex messages instead of printing one message
        #[arg(long)]
        list: bool,

        /// Print structured JSON for the selected item
        #[arg(long)]
        json: bool,
    },
    /// Read the latest Codex item aloud, discuss it, and produce a prompt
    Ask {
        #[command(flatten)]
        selection: MessageSelection,
    },
    /// Start a voice session focused only on producing a prompt
    Prompt,
}

#[derive(Debug, Clone, Parser, Default)]
pub struct MessageSelection {
    /// Readable item offset, where 0 is the latest selected message
    #[arg(long, default_value_t = 0)]
    pub item: usize,
}

impl Cli {
    pub fn parse_args(args: &[OsString]) -> Self {
        Self::parse_from(args)
    }

    pub fn command(&self) -> Command {
        self.command.clone().unwrap_or_default()
    }
}

impl Default for Command {
    fn default() -> Self {
        Self::Ask {
            selection: MessageSelection::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::{Cli, Command};

    #[test]
    fn defaults_to_ask_when_only_pane_option_is_present() {
        let cli = Cli::parse_args(&[
            OsString::from("codex-voice"),
            OsString::from("--pane"),
            OsString::from("%10"),
        ]);
        assert!(matches!(cli.command(), Command::Ask { .. }));
        assert_eq!(cli.pane.as_deref(), Some("%10"));
    }
}
