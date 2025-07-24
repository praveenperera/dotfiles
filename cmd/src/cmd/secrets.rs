pub mod flags;

use std::ffi::OsString;
use std::path::PathBuf;

use eyre::Result;
use xshell::{cmd, Shell};

use crate::util;
use crate::util::VAULT;

static SECRET_NAME: &str = "cmd_secrets";
static SECRETS: [&str; 2] = ["ln.yaml", "sq.yaml"];

pub fn run(sh: &Shell, args: &[OsString]) -> Result<()> {
    let flags = flags::Secrets::from_args(args)?;

    match flags.subcommand {
        flags::SecretsCmd::Gen(cmd) => {
            gen(sh, cmd.length, cmd.no_symbols)?;
        }
        flags::SecretsCmd::Get(cmd) => {
            get(sh, cmd.secret_name.as_deref(), &cmd.secret)?;
        }
        flags::SecretsCmd::Save(_) => {
            save(sh)?;
        }
        flags::SecretsCmd::Update(cmd) => {
            update(sh, &cmd.secret)?;
        }
    }

    Ok(())
}


pub fn gen(_sh: &Shell, length: Option<usize>, no_symbols: bool) -> Result<()> {
    let length = length.unwrap_or(32);

    let string = if no_symbols {
        util::random_alpha_numeric(length)
    } else {
        util::random_ascii(length)
    };

    println!("{string}");

    Ok(())
}

pub fn save(sh: &Shell) -> Result<()> {
    let secret_dir = crate::dotfiles_dir().join("cmd/secrets");
    for secret in SECRETS {
        eprintln!("getting secret: {secret}");
        let secret_text = get_and_return(sh, SECRET_NAME, secret)?;
        let secret_path = secret_dir.join(secret);

        std::fs::write(secret_path, secret_text.trim())?;
    }

    Ok(())
}

pub fn get(sh: &Shell, secret_name: Option<&str>, secret: &str) -> Result<()> {
    let secret_name = secret_name.unwrap_or(SECRET_NAME);

    eprintln!("getting secret: {secret}");

    let secret_text = get_and_return(sh, secret_name, secret)?;
    println!("{}", secret_text.trim());

    Ok(())
}

pub fn get_and_return(sh: &Shell, secret_name: &str, secret: &str) -> Result<String> {
    let secret_text = cmd!(sh, "op read op://{VAULT}/{secret_name}/{secret}").read()?;
    Ok(secret_text.trim().to_string())
}

pub fn update(sh: &Shell, secret: &str) -> Result<()> {
    let secret_dir = crate::dotfiles_dir().join("cmd/secrets");

    match secret {
        "all" => {
            for secret in SECRETS {
                update_single_secret(sh, secret, secret_dir.join(secret))?;
            }
        }
        secret => update_single_secret(sh, secret, secret_dir.join(secret))?,
    }

    Ok(())
}

fn update_single_secret(sh: &Shell, secret: &str, secret_path: PathBuf) -> Result<()> {
    let secret_text = std::fs::read_to_string(secret_path)?;
    let cleaned_field = secret.replace('.', "\\.");

    cmd!(
        sh,
        "op item edit {SECRET_NAME} {cleaned_field}={secret_text}"
    )
    .run()?;

    Ok(())
}
