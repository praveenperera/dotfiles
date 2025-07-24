mod flags;

use eyre::Result;
use xshell::Shell;

use crate::{encrypt, util::handle_xflags_error};

static DEFAULT_SECRET_HEADER: &str = "!!CMD!!ID!!vault-default";

pub fn run(sh: &Shell, args: &[&str]) -> Result<()> {
    let os_args = args
        .iter()
        .map(|s| std::ffi::OsString::from(*s))
        .collect::<Vec<_>>();

    let flags = handle_xflags_error(flags::Vault::from_vec(os_args), args, flags::Vault::help())?;

    match flags.subcommand {
        flags::VaultCmd::Encrypt(cmd) => {
            encrypt(sh, &cmd.file)?;
        }
        flags::VaultCmd::Decrypt(cmd) => {
            decrypt(sh, &cmd.file)?;
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