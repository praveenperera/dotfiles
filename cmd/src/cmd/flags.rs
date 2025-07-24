use bpaf::Bpaf;

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
pub struct Cmd {
    /// Print version information
    #[bpaf(switch)]
    pub version: bool,

    #[bpaf(external(main_cmd))]
    pub subcommand: MainCmd,
}

#[derive(Debug, Clone, Bpaf)]
pub enum MainCmd {
    /// Bootstrap dotfiles
    #[bpaf(command)]
    Bootstrap {
        /// Bootstrap mode: 'minimal' or 'full'
        #[bpaf(positional("MODE"))]
        mode: crate::cmd::bootstrap::BootstrapMode,
    },

    /// Release/update cmd binary
    #[bpaf(command)]
    Release,

    /// Configure dotfiles
    #[bpaf(command("config"))]
    Config,

    /// Configure dotfiles (alias)
    #[bpaf(command("cfg"))]
    Cfg,

    /// Google Cloud operations
    #[bpaf(command)]
    Gcloud {
        #[bpaf(external(crate::cmd::gcloud::flags::gcloud_cmd))]
        subcommand: crate::cmd::gcloud::flags::GcloudCmd,
    },

    /// Secret operations
    #[bpaf(command)]
    Secret {
        #[bpaf(external(crate::cmd::secrets::flags::secrets_cmd))]
        subcommand: crate::cmd::secrets::flags::SecretsCmd,
    },

    /// Terraform operations
    #[bpaf(command)]
    Terraform {
        #[bpaf(external(crate::cmd::terraform::flags::terraform_cmd))]
        subcommand: crate::cmd::terraform::flags::TerraformCmd,
    },

    /// Terraform operations (alias)
    #[bpaf(command("tf"))]
    Tf {
        #[bpaf(external(crate::cmd::terraform::flags::terraform_cmd))]
        subcommand: crate::cmd::terraform::flags::TerraformCmd,
    },

    /// Vault operations
    #[bpaf(command)]
    Vault {
        #[bpaf(external(crate::cmd::vault::flags::vault_cmd))]
        subcommand: crate::cmd::vault::flags::VaultCmd,
    },

    /// Generate code/files
    #[bpaf(command)]
    Generate {
        #[bpaf(external(crate::cmd::generate::flags::generate_cmd))]
        subcommand: crate::cmd::generate::flags::GenerateCmd,
    },

    /// Generate code/files (alias)
    #[bpaf(command("gen"))]
    Gen {
        #[bpaf(external(crate::cmd::generate::flags::generate_cmd))]
        subcommand: crate::cmd::generate::flags::GenerateCmd,
    },
}

impl Cmd {
    pub fn from_args(args: &[std::ffi::OsString]) -> eyre::Result<Self> {
        match cmd().fallback_to_usage().run_inner(args) {
            Ok(result) => Ok(result),
            Err(bpaf::ParseFailure::Stdout(doc, _)) => {
                println!("{}", doc);
                std::process::exit(0);
            }
            Err(bpaf::ParseFailure::Stderr(doc)) => {
                eprintln!("{}", doc);
                std::process::exit(1);
            }
            Err(bpaf::ParseFailure::Completion(completion)) => {
                println!("{}", completion);
                std::process::exit(0);
            }
        }
    }
}

