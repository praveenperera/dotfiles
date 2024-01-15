use std::path::PathBuf;

use eyre::Result;
use xshell::{cmd, Shell};

use crate::util;

static SECRET_NAME: &str = "cmd_secrets";
static SECRETS: [&str; 2] = ["ln.yaml", "sq.yaml"];

pub fn gen(_sh: &Shell, args: &[&str]) -> Result<()> {
    let length = args
        .first()
        .map(|s| s.parse::<usize>())
        .transpose()?
        .unwrap_or(32);

    let pass = util::random_ascii(length);
    println!("{}", pass);

    Ok(())
}

pub fn get(sh: &Shell, _args: &[&str]) -> Result<()> {
    let secret_dir = crate::dotfiles_dir().join("cmd/secrets");
    for secret in SECRETS {
        println!("getting secret: {secret}");
        let secret_text = cmd!(sh, "op read op://Personal/{SECRET_NAME}/{secret}").read()?;
        let secret_path = secret_dir.join(secret);

        std::fs::write(secret_path, secret_text.trim())?;
    }

    Ok(())
}

pub fn update(sh: &Shell, args: &[&str]) -> Result<()> {
    let secret_dir = crate::dotfiles_dir().join("cmd/secrets");

    match *args.first().expect("need arg for secret") {
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
    let cleaned_field = secret.replace(".", "\\.");

    cmd!(
        sh,
        "op item edit {SECRET_NAME} {cleaned_field}={secret_text}"
    )
    .run()?;

    Ok(())
}
