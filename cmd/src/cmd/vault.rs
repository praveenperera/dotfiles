use eyre::Result;
use xshell::Shell;

use crate::encrypt;

static DEFAULT_SECRET_HEADER: &str = "!!CMD!!ID!!vault-default";

pub fn run(sh: &Shell, args: &[&str]) -> Result<()> {
    match args {
        [] => eprintln!("need args"),

        ["enc" | "encrypt", file] => {
            encrypt(sh, file)?;
        }

        ["dec" | "decrypt", file] => {
            decrypt(sh, file)?;
        }

        cmd => {
            eprintln!("vault command not implemented: {cmd:?}");
        }
    }

    Ok(())
}

fn encrypt(sh: &Shell, file: &str) -> Result<()> {
    let output = format!("{}.enc", file);

    if !sh.path_exists(&output) {
        sh.write_file(&output, DEFAULT_SECRET_HEADER)?;
    }

    encrypt::encrypt(sh, file, &output)?;
    Ok(())
}

fn decrypt(sh: &Shell, file: &str) -> Result<()> {
    let output = file.trim_end_matches(".enc");

    if file == output {
        return Err(eyre::eyre!("file does not end with .enc"));
    }

    encrypt::decrypt(sh, file, output)?;

    Ok(())
}
