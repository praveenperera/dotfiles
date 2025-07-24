use bpaf::Bpaf;

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
pub struct Vault {
    #[bpaf(external(vault_cmd))]
    pub subcommand: VaultCmd,
}

#[derive(Debug, Clone, Bpaf)]
pub enum VaultCmd {
    /// Encrypt file
    #[bpaf(command("encrypt"))]
    Encrypt {
        #[bpaf(positional("FILE"))]
        file: String,
    },

    /// Encrypt file (alias)
    #[bpaf(command("enc"))]
    Enc {
        #[bpaf(positional("FILE"))]
        file: String,
    },

    /// Decrypt file
    #[bpaf(command("decrypt"))]
    Decrypt {
        #[bpaf(positional("FILE"))]
        file: String,
    },

    /// Decrypt file (alias)
    #[bpaf(command("dec"))]
    Dec {
        #[bpaf(positional("FILE"))]
        file: String,
    },
}

impl Vault {
    pub fn help() -> &'static str {
        "Vault operations\n\nCommands:\n  encrypt   Encrypt file\n  enc       Encrypt file (alias)\n  decrypt   Decrypt file\n  dec       Decrypt file (alias)"
    }

    pub fn from_args(args: &[std::ffi::OsString]) -> eyre::Result<Self> {
        match vault().fallback_to_usage().run_inner(args) {
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
