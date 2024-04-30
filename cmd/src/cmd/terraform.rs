use std::{path::Path, process::Command};

use eyre::{Context as _, ContextCompat as _, Result};
use sha2::Digest;
use xshell::Shell;

use crate::encrypt;

pub fn run(sh: &Shell, args: &[&str]) -> Result<()> {
    match args {
        [] => eprintln!("need args"),

        ["init", args @ ..] => {
            init(sh, args)?;
        }

        ["encrypt" | "enc"] => {
            encrypt(sh)?;
        }

        ["decrypt" | "dec"] => {
            decrypt(sh)?;
        }

        [cmd, args @ ..] => {
            run_terraform_cmd(sh, cmd, args)?;
        }
    }

    Ok(())
}

fn init(sh: &Shell, _args: &[&str]) -> Result<()> {
    if sh.path_exists("terraform.tfstate.enc") {
        eprintln!("terraform.tfstate.enc already exists");
    } else {
        eprintln!("terraform.tfstate.enc does not exist, creating...");
        encrypt::create_secret_and_files(sh, "terraform-state-pw", "terraform.tfstate.enc")?;
    }

    let terraform_state = encrypt::read_encrypted_file("")?;

    if terraform_state.is_empty() {
        eprintln!("terraform.tfstate.enc is empty");
    } else {
        eprintln!("terraform.tfstate.enc is not empty");
    }

    run_terraform_cmd(sh, "init", &[])?;

    Ok(())
}

fn run_terraform_cmd(sh: &Shell, cmd: &str, args: &[&str]) -> Result<()> {
    let tmpdir = tempfile::tempdir()?;
    let tfstate = tmpdir.path().join("terraform.tfstate");

    let tfstate = tfstate
        .to_str()
        .wrap_err("could not convert path to string")?;

    encrypt::encrypt(sh, "terraform.tfstate.enc", tfstate)?;
    let before_hash = sha2::Sha256::digest(sh.read_file(tfstate)?);

    // use command instead of xshell because to deal with interactive prompts
    let result = Command::new("terraform")
        .arg(cmd)
        .arg("-state")
        .arg(tfstate)
        .args(args)
        .spawn()
        .wrap_err("could not spawn terraform")?
        .wait()
        .wrap_err("could not wait for terraform")?;

    if !result.success() {
        sh.remove_path(tfstate)?;
        return Err(eyre::eyre!("terraform {cmd} failed"));
    };

    let after_hash = sha2::Sha256::digest(sh.read_file(tfstate)?);
    if before_hash != after_hash {
        encrypt::encrypt(sh, tfstate, "terraform.tfstate.enc")?;

        let tfstate_parent = Path::new(tfstate)
            .parent()
            .wrap_err("could not get parent of input file")?;

        sh.remove_path(tfstate_parent)?;

        let tfstate = tfstate_parent.join("terraform.tfstate");
        let tfstate_backup = tfstate.join("terraform.tfstate.backup");

        sh.remove_path(tfstate_backup)?;
        sh.remove_path(tfstate)?;
    }

    sh.remove_path(tfstate)?;

    Ok(())
}

fn encrypt(sh: &Shell) -> Result<()> {
    init(sh, &[])?;
    encrypt::encrypt(sh, "terraform.tfstate", "terraform.tfstate.enc")
}

fn decrypt(sh: &Shell) -> Result<()> {
    encrypt::decrypt(sh, "terraform.tfstate.enc", "terraform.tfstate")
}
