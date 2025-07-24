use clap::{Parser, Subcommand};

#[derive(Debug, Clone, Parser)]
pub struct Vault {
    #[command(subcommand)]
    pub subcommand: VaultCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum VaultCmd {
    /// Encrypt file
    #[command(visible_alias = "enc", arg_required_else_help = true)]
    Encrypt {
        file: String,
    },

    /// Decrypt file
    #[command(visible_alias = "dec", arg_required_else_help = true)]
    Decrypt {
        file: String,
    },
}
