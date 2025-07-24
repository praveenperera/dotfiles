pub mod flags;

use eyre::Result;
use std::ffi::OsString;
use xshell::Shell;

use crate::encrypt;

static DEFAULT_SECRET_HEADER: &str = "!!CMD!!ID!!vault-default";

pub fn run(sh: &Shell, args: &[OsString]) -> Result<()> {
    let flags = flags::Vault::from_args(args)?;
    run_with_flags(sh, flags)
}

pub fn run_with_flags(sh: &Shell, flags: flags::Vault) -> Result<()> {

    match flags.subcommand {
        flags::VaultCmd::Encrypt { file } | flags::VaultCmd::Enc { file } => {
            encrypt(sh, &file)?;
        }
        flags::VaultCmd::Decrypt { file } | flags::VaultCmd::Dec { file } => {
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
    let output = file.trim_end_matches(".enc");

    encrypt::decrypt(sh, file, output)?;
    Ok(())
}
