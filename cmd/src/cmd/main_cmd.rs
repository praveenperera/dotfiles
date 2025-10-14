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

    /// Fetch PR comments and their code references from GitHub
    #[command(visible_alias = "prc")]
    PrContext {
        /// GitHub PR URL or repository in format "owner/repo"
        repo_or_url: String,

        /// Pull request number (optional if URL is provided)
        pr_number: Option<u64>,

        /// GitHub token (optional, for higher rate limits)
        #[arg(short, long, env = "GITHUB_TOKEN")]
        token: Option<String>,

        /// Only include comments with code references
        #[arg(short = 'c', long)]
        code_only: bool,

        /// Compact output (only author, body, and code_reference)
        #[arg(short = 'C', long)]
        compact: bool,
    },
}

impl Cmd {
    pub fn from_args(args: &[std::ffi::OsString]) -> eyre::Result<Self> {
        use clap::{CommandFactory, Parser};
        let mut full_args = vec![std::ffi::OsString::from("cmd")];
        full_args.extend_from_slice(args);

        // check if help is requested or no args provided
        let is_help_or_no_args = (full_args.iter().any(|arg| {
            arg == "--help" || arg == "-h"
        }) && full_args.len() == 2) || full_args.len() == 1;

        if is_help_or_no_args {
            // generate custom help with symlinked commands info
            let mut cmd = Self::command();
            let symlinked_help = crate::symlinked_commands_help();
            cmd = cmd.after_help(symlinked_help);
            cmd.print_help().unwrap();
            std::process::exit(0);
        }

        match Self::try_parse_from(full_args) {
            Ok(cmd) => Ok(cmd),
            Err(err) => {
                err.print().unwrap();
                std::process::exit(err.exit_code());
            }
        }
    }
}
