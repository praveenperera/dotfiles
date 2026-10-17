use clap::{Parser, Subcommand};
use std::path::Path;

use eyre::Result;
use xshell::Shell;

use crate::encrypt;

#[derive(Debug, Clone, Parser)]
pub struct File {
    #[command(subcommand)]
    pub subcommand: FileCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum FileCmd {
    /// Encrypt a file
    #[command(visible_alias = "enc", arg_required_else_help = true)]
    Encrypt {
        /// Input file to encrypt
        input: String,
        /// Output file (defaults to input.enc)
        output: Option<String>,
    },

    /// Decrypt a file
    #[command(visible_alias = "dec", arg_required_else_help = true)]
    Decrypt {
        /// Input file to decrypt
        input: String,
        /// Output file (defaults to input without .enc)
        output: Option<String>,
    },

    /// Initialize encryption keys for a new file
    #[command(arg_required_else_help = true)]
    Init {
        /// Source file to initialize encryption for
        input: String,
        /// Prefix for the 1password secret name
        #[arg(long, short = 'p')]
        prefix: Option<String>,
    },
}

pub fn run_with_flags(sh: &Shell, flags: File) -> Result<()> {
    match flags.subcommand {
        FileCmd::Encrypt { input, output } => encrypt_file(sh, &input, output.as_deref()),
        FileCmd::Decrypt { input, output } => decrypt_file(sh, &input, output.as_deref()),
        FileCmd::Init { input, prefix } => init_file(sh, &input, prefix.as_deref()),
    }
}

fn encrypt_file(sh: &Shell, input: &str, output: Option<&str>) -> Result<()> {
    let output = match output {
        Some(o) => o.to_string(),
        None => format!("{input}.enc"),
    };

    // the output's header names the 1Password key, so only `file init` can create it
    if !sh.path_exists(&output) {
        let hint = if output.ends_with(".enc") {
            format!("run `cmd file init {output}` first to create its encryption key")
        } else {
            "`cmd file init` only creates .enc files, so use an output ending in .enc".to_string()
        };

        return Err(eyre::eyre!("output file {output} does not exist, {hint}"));
    }

    encrypt::encrypt(sh, input, &output)
}

fn decrypt_file(sh: &Shell, input: &str, output: Option<&str>) -> Result<()> {
    // relative to the cwd like any other CLI path, the default lands next to the input
    let output = match output {
        Some(o) => o.to_string(),
        None => match input.strip_suffix(".enc") {
            Some(stem) => stem.to_string(),
            None => format!("{input}.dec"),
        },
    };

    encrypt::decrypt(sh, input, &output)
}

fn init_file(sh: &Shell, input: &str, prefix: Option<&str>) -> Result<()> {
    let output = if input.ends_with(".enc") {
        input.to_string()
    } else {
        format!("{input}.enc")
    };

    if sh.path_exists(&output) {
        return Err(eyre::eyre!("{output} already exists"));
    }

    let prefix = prefix.unwrap_or_else(|| {
        Path::new(&output)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("file")
    });

    encrypt::create_secret_and_files(sh, prefix, &output)?;

    println!("Created encryption keys and initialized {output}");
    Ok(())
}
