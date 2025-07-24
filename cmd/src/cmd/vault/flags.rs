use clap::{Parser, Subcommand};

#[derive(Debug, Clone, Parser)]
pub struct Vault {
    #[command(subcommand)]
    pub subcommand: VaultCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum VaultCmd {
    /// Encrypt file
    #[command(visible_alias = "enc")]
    Encrypt {
        file: String,
    },

    /// Decrypt file
    #[command(visible_alias = "dec")]
    Decrypt {
        file: String,
    },
}
