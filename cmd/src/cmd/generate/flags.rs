use clap::{Parser, Subcommand};

#[derive(Debug, Clone, Parser)]
pub struct Generate {
    #[command(subcommand)]
    pub subcommand: GenerateCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum GenerateCmd {
    /// Rust multi platform
    #[command(arg_required_else_help = true)]
    Rmp {
        /// either `swift` or `rs`
        lang: String,
        
        /// name of the module name ex: `MyModule`
        module_name: String,
        
        /// the name of the app, default to `cove`
        #[arg(short, long)]
        app: Option<String>,
    },

    /// Swift related generators
    #[command(arg_required_else_help = true)]
    Swift {
        name: String,
        identifier: String,
        path: Option<String>,
        #[arg(trailing_var_arg = true)]
        rest: Vec<String>,
    },

    /// Swift Colors
    #[command(arg_required_else_help = true)]
    SwiftColor {
        name: String,
        light_hex: String,
        dark_hex: Option<String>,
    },
}
