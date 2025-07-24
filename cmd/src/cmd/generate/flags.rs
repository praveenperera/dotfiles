use bpaf::Bpaf;

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
pub struct Generate {
    #[bpaf(external(generate_cmd))]
    pub subcommand: GenerateCmd,
}

#[derive(Debug, Clone, Bpaf)]
pub enum GenerateCmd {
    /// Show help
    #[bpaf(command)]
    Help,

    /// Rust multi platform
    #[bpaf(command)]
    Rmp {
        /// either `swift` or `rs`
        #[bpaf(positional("LANG"))]
        lang: String,

        /// name of the module name ex: `MyModule`
        #[bpaf(positional("MODULE_NAME"))]
        module_name: String,

        /// the name of the app, default to `cove`
        #[bpaf(short('a'), long("app"), argument("APP"))]
        app: Option<String>,
    },

    /// Swift related generators
    #[bpaf(command)]
    Swift {
        #[bpaf(positional("NAME"))]
        name: String,

        #[bpaf(positional("IDENTIFIER"))]
        identifier: String,

        #[bpaf(positional("PATH"), optional)]
        path: Option<String>,

        #[bpaf(positional("REST"), many)]
        rest: Vec<String>,
    },

    /// Swift Colors
    #[bpaf(command("SwiftColor"))]
    SwiftColor {
        #[bpaf(positional("NAME"))]
        name: String,

        #[bpaf(positional("LIGHT_HEX"))]
        light_hex: String,

        #[bpaf(positional("DARK_HEX"), optional)]
        dark_hex: Option<String>,
    },
}

impl Generate {
    pub const HELP: &'static str = "Generate code/files\n\nCommands:\n  help        Show help\n  rmp         Rust multi platform\n  swift       Swift related generators\n  SwiftColor  Swift Colors";

    pub fn from_args(args: &[std::ffi::OsString]) -> eyre::Result<Self> {
        match generate().fallback_to_usage().run_inner(args) {
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
