use clap::{Parser, Subcommand};

#[derive(Debug, Clone, Parser)]
pub struct Terraform {
    #[command(subcommand)]
    pub subcommand: TerraformCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum TerraformCmd {
    /// Run terraform command (default)
    Run {
        command: String,
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Initialize terraform state
    Init {
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Encrypt terraform state file
    #[command(visible_alias = "enc")]
    Encrypt {
        file: Option<String>,
    },

    /// Decrypt terraform state file
    #[command(visible_alias = "dec")]
    Decrypt {
        file: Option<String>,
    },
}
