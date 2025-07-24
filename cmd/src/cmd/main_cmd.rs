use clap::{Parser, Subcommand};

#[derive(Debug, Clone, Parser)]
#[command(
    name = "cmd",
    about = "Command line utilities",
    arg_required_else_help = true
)]
pub struct Cmd {
    /// Print version information
    #[arg(long, short = 'V')]
    pub version: bool,

    #[command(subcommand)]
    pub subcommand: MainCmd,
}

#[derive(Debug, Clone, Subcommand)]
#[command(subcommand_value_name = "COMMAND")]
pub enum MainCmd {
    /// Bootstrap dotfiles
    #[command(arg_required_else_help = true)]
    Bootstrap {
        mode: crate::cmd::bootstrap::BootstrapMode,
    },

    /// Release/update cmd binary
    Release,

    /// Configure dotfiles
    #[command(visible_alias = "cfg")]
    Config,

    /// Google Cloud operations
    #[command(arg_required_else_help = true)]
    Gcloud {
        #[command(subcommand)]
        subcommand: crate::cmd::gcloud::GcloudCmd,
    },

    /// Secret operations
    #[command(arg_required_else_help = true)]
    Secret {
        #[command(subcommand)]
        subcommand: crate::cmd::secrets::SecretsCmd,
    },

    /// Terraform operations
    #[command(visible_alias = "tf", arg_required_else_help = true)]
    Terraform {
        #[command(subcommand)]
        subcommand: crate::cmd::terraform::TerraformCmd,
    },

    /// Vault operations
    #[command(arg_required_else_help = true)]
    Vault {
        #[command(subcommand)]
        subcommand: crate::cmd::vault::VaultCmd,
    },

    /// Generate code/files
    #[command(visible_alias = "gen", arg_required_else_help = true)]
    Generate {
        #[command(subcommand)]
        subcommand: crate::cmd::generate::GenerateCmd,
    },
}

impl Cmd {
    pub fn from_args(args: &[std::ffi::OsString]) -> eyre::Result<Self> {
        use clap::Parser;
        let mut full_args = vec![std::ffi::OsString::from("cmd")];
        full_args.extend_from_slice(args);
        match Self::try_parse_from(full_args) {
            Ok(cmd) => Ok(cmd),
            Err(err) => {
                err.print().unwrap();
                std::process::exit(err.exit_code());
            }
        }
    }
}
