use bpaf::Bpaf;

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
pub struct Gcloud {
    #[bpaf(external(gcloud_cmd))]
    pub subcommand: GcloudCmd,
}

#[derive(Debug, Clone, Bpaf)]
pub enum GcloudCmd {
    /// Google Cloud login
    #[bpaf(command)]
    Login {
        #[bpaf(positional("PROJECT"))]
        project: String,
    },

    /// Google Cloud switch project
    #[bpaf(command("switch-project"))]
    SwitchProject {
        #[bpaf(positional("PROJECT"))]
        project: String,
    },

    /// Google Cloud switch project (alias)
    #[bpaf(command("sp"))]
    Sp {
        #[bpaf(positional("PROJECT"))]
        project: String,
    },

    /// Google Cloud switch cluster
    #[bpaf(command("switch-cluster"))]
    SwitchCluster {
        #[bpaf(positional("PROJECT"))]
        project: String,

        #[bpaf(positional("CLUSTER"))]
        cluster: String,
    },

    /// Google Cloud switch cluster (alias)
    #[bpaf(command("sc"))]
    Sc {
        #[bpaf(positional("PROJECT"))]
        project: String,

        #[bpaf(positional("CLUSTER"))]
        cluster: String,
    },
}

impl Gcloud {
    pub fn help() -> &'static str {
        "Google Cloud operations\n\nCommands:\n  login            Google Cloud login\n  switch-project   Google Cloud switch project\n  sp               Google Cloud switch project (alias)\n  switch-cluster   Google Cloud switch cluster\n  sc               Google Cloud switch cluster (alias)"
    }

    pub fn from_args(args: &[std::ffi::OsString]) -> eyre::Result<Self> {
        match gcloud().fallback_to_usage().run_inner(args) {
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
