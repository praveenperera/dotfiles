use bpaf::Bpaf;

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
pub struct Terraform {
    #[bpaf(external(terraform_cmd))]
    pub subcommand: TerraformCmd,
}

#[derive(Debug, Clone, Bpaf)]
pub enum TerraformCmd {
    /// Run terraform command (default)
    #[bpaf(command)]
    Run {
        #[bpaf(positional("COMMAND"))]
        command: String,

        #[bpaf(positional("ARGS"), many)]
        args: Vec<String>,
    },

    /// Initialize terraform state
    #[bpaf(command)]
    Init {
        #[bpaf(positional("ARGS"), many)]
        args: Vec<String>,
    },

    /// Encrypt terraform state file
    #[bpaf(command("encrypt"))]
    Encrypt {
        #[bpaf(positional("FILE"), optional)]
        file: Option<String>,
    },

    /// Encrypt terraform state file (alias)
    #[bpaf(command("enc"))]
    Enc {
        #[bpaf(positional("FILE"), optional)]
        file: Option<String>,
    },

    /// Decrypt terraform state file
    #[bpaf(command("decrypt"))]
    Decrypt {
        #[bpaf(positional("FILE"), optional)]
        file: Option<String>,
    },

    /// Decrypt terraform state file (alias)
    #[bpaf(command("dec"))]
    Dec {
        #[bpaf(positional("FILE"), optional)]
        file: Option<String>,
    },
}

impl Terraform {
    pub fn help() -> &'static str {
        "Terraform operations\n\nCommands:\n  run       Run terraform command (default)\n  init      Initialize terraform state\n  encrypt   Encrypt terraform state file\n  enc       Encrypt terraform state file (alias)\n  decrypt   Decrypt terraform state file\n  dec       Decrypt terraform state file (alias)"
    }

    pub fn from_args(args: &[std::ffi::OsString]) -> eyre::Result<Self> {
        match terraform().fallback_to_usage().run_inner(args) {
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
