use super::BootstrapMode;
use bpaf::Bpaf;

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
pub struct Bootstrap {
    /// Bootstrap mode: 'minimal' or 'full'
    #[bpaf(positional("MODE"))]
    pub mode: BootstrapMode,
}

impl Bootstrap {
    pub fn help() -> &'static str {
        "Bootstrap dotfiles\n\nUsage: bootstrap <MODE>\n\nArguments:\n  <MODE>  Bootstrap mode: 'minimal' or 'full'"
    }

    pub fn from_args(args: &[std::ffi::OsString]) -> eyre::Result<Self> {
        match bootstrap().fallback_to_usage().run_inner(args) {
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
