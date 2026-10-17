use clap::{Parser, Subcommand};
use eyre::Result;
use std::ffi::OsString;
use xshell::Shell;

use crate::encrypt;

#[derive(Debug, Clone, Parser)]
pub struct Vault {
    #[command(subcommand)]
    pub subcommand: VaultCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum VaultCmd {
    /// Encrypt file
    #[command(visible_alias = "enc", arg_required_else_help = true)]
    Encrypt { file: String },

    /// Decrypt file
    #[command(visible_alias = "dec", arg_required_else_help = true)]
    Decrypt { file: String },
}

static DEFAULT_SECRET_HEADER: &str = "!!CMD!!ID!!vault-default";

pub fn run(sh: &Shell, args: &[OsString]) -> Result<()> {
    let flags = Vault::parse_from(args);
    run_with_flags(sh, flags)
}

pub fn run_with_flags(sh: &Shell, flags: Vault) -> Result<()> {
    match flags.subcommand {
        VaultCmd::Encrypt { file } => {
            encrypt(sh, &file)?;
        }
        VaultCmd::Decrypt { file } => {
            decrypt(sh, &file)?;
        }
    }

    Ok(())
}

fn encrypt(sh: &Shell, file: &str) -> Result<()> {
    let output = format!("{file}.enc");

    if !sh.path_exists(&output) {
        sh.write_file(&output, DEFAULT_SECRET_HEADER)?;
    }

    encrypt::encrypt(sh, file, &output)?;
    Ok(())
}

fn decrypt(sh: &Shell, file: &str) -> Result<()> {
    // derived from the input path relative to the cwd, so it lands next to the input
    let output_file = match file.strip_suffix(".enc") {
        Some(stem) => stem.to_string(),
        None => format!("{file}.dec"),
    };

    encrypt::decrypt(sh, file, output_file)?;
    Ok(())
}
