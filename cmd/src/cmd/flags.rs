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
    Gcloud {
        #[command(subcommand)]
        subcommand: crate::cmd::gcloud::flags::GcloudCmd,
    },

    /// Secret operations
    Secret {
        #[command(subcommand)]
        subcommand: crate::cmd::secrets::flags::SecretsCmd,
    },

    /// Terraform operations
    #[command(visible_alias = "tf")]
    Terraform {
        #[command(subcommand)]
        subcommand: crate::cmd::terraform::flags::TerraformCmd,
    },

    /// Vault operations
    Vault {
        #[command(subcommand)]
        subcommand: crate::cmd::vault::flags::VaultCmd,
    },

    /// Generate code/files
    #[command(visible_alias = "gen")]
    Generate {
        #[command(subcommand)]
        subcommand: crate::cmd::generate::flags::GenerateCmd,
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
