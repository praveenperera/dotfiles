use clap::{Parser, Subcommand};
use std::{ffi::OsString, path::Path};

use eyre::{Context as _, ContextCompat as _, Result};
use log::debug;
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
        /// Output file to create
        output: String,
        /// Prefix for the 1password secret name
        #[arg(long, short = 'p')]
        prefix: Option<String>,
    },
}

pub fn run(sh: &Shell, args: &[OsString]) -> Result<()> {
    debug!("file args: {args:?}");
    let flags = File::parse_from(args);
    run_with_flags(sh, flags)
}

pub fn run_with_flags(sh: &Shell, flags: File) -> Result<()> {
    match flags.subcommand {
        FileCmd::Encrypt { input, output } => encrypt_file(sh, &input, output.as_deref()),
        FileCmd::Decrypt { input, output } => decrypt_file(sh, &input, output.as_deref()),
        FileCmd::Init { output, prefix } => init_file(sh, &output, prefix.as_deref()),
    }
}

fn encrypt_file(sh: &Shell, input: &str, output: Option<&str>) -> Result<()> {
    let output = match output {
        Some(o) => o.to_string(),
        None => format!("{input}.enc"),
    };

    if !sh.path_exists(&output) {
        return Err(eyre::eyre!(
            "Output file {output} does not exist. Run `cmd file init {output}` first to create encryption keys."
        ));
    }

    encrypt::encrypt(sh, input, &output)
}

fn decrypt_file(sh: &Shell, input: &str, output: Option<&str>) -> Result<()> {
    let input_path = std::fs::canonicalize(input).wrap_err("could not canonicalize input path")?;

    let parent = input_path
        .parent()
        .wrap_err("could not get parent of input file")?;

    let output = match output {
        Some(o) => o.to_string(),
        None => {
            if input.ends_with(".enc") {
                input.trim_end_matches(".enc").to_string()
            } else {
                format!("{input}.dec")
            }
        }
    };

    let output_path = if Path::new(&output).is_absolute() {
        Path::new(&output).to_path_buf()
    } else {
        parent.join(&output)
    };

    encrypt::decrypt(sh, input, &output_path)
}

fn init_file(sh: &Shell, output: &str, prefix: Option<&str>) -> Result<()> {
    if sh.path_exists(output) {
        return Err(eyre::eyre!("{output} already exists"));
    }

    let prefix = prefix.unwrap_or_else(|| {
        Path::new(output)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("file")
    });

    encrypt::create_secret_and_files(sh, prefix, output)?;

    println!("Created encryption keys and initialized {output}");
    Ok(())
}
