//! Hooks that agent harnesses run around tool calls

pub mod claude;

use clap::Subcommand;

#[derive(Debug, Clone, Subcommand)]
pub enum HookCmd {
    /// Run a Claude Code hook: read the hook input on stdin and print its decision
    #[command(visible_alias = "c")]
    Claude {
        #[arg(value_enum)]
        event: claude::Event,
    },
}

pub fn run(subcommand: HookCmd) {
    match subcommand {
        HookCmd::Claude { event } => claude::run(event),
    }
}
