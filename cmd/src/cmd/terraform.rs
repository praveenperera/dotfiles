use clap::{Parser, Subcommand};
use std::{ffi::OsString, path::Path, process::Command};

use eyre::{Context as _, ContextCompat as _, Result};
use log::debug;
use sha2::Digest;
use xshell::Shell;

use crate::encrypt;

#[derive(Debug, Clone, Parser)]
pub struct Terraform {
    #[command(subcommand)]
    pub subcommand: TerraformCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum TerraformCmd {
    /// Run terraform command (default)
    #[command(arg_required_else_help = true)]
    Run {
        command: String,
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Initialize terraform state
    Init,

    /// Encrypt terraform state file
    #[command(visible_alias = "enc")]
    Encrypt { file: Option<String> },

    /// Decrypt terraform state file
    #[command(visible_alias = "dec")]
    Decrypt { file: Option<String> },
}

pub fn run(sh: &Shell, args: &[OsString]) -> Result<()> {
    debug!("terraform args 1: {args:?}");
    let flags = Terraform::parse_from(args);
    run_with_flags(sh, flags)
}

pub fn run_with_flags(sh: &Shell, flags: Terraform) -> Result<()> {
    match flags.subcommand {
        TerraformCmd::Init => {
            init(sh)?;
        }
        TerraformCmd::Encrypt { file } => {
            let file = file.as_deref().unwrap_or("terraform.tfstate");
            encrypt(sh, file)?;
        }
        TerraformCmd::Decrypt { file } => {
            let file = file.as_deref().unwrap_or("terraform.tfstate.enc");
            decrypt(sh, file)?;
        }
        TerraformCmd::Run { command, args } => {
            let args: Vec<OsString> = args.iter().map(OsString::from).collect();
            run_terraform_cmd(sh, &command, &args)?;
        }
    }

    Ok(())
}

fn init(sh: &Shell) -> Result<()> {
    if sh.path_exists("terraform.tfstate.enc") {
        eprintln!("terraform.tfstate.enc already exists");
    } else {
        eprintln!("terraform.tfstate.enc does not exist, creating...");
        encrypt::create_secret_and_files(sh, "terraform-state-pw", "terraform.tfstate.enc")?;
    }

    let terraform_state = encrypt::read_encrypted_file("terraform.tfstate.enc")?;

    if terraform_state.is_empty() {
        eprintln!("terraform.tfstate.enc is empty");
    } else {
        eprintln!("terraform.tfstate.enc is not empty");
    }

    run_terraform_cmd(sh, "init", &[])?;

    Ok(())
}

fn run_terraform_cmd(sh: &Shell, cmd: &str, args: &[OsString]) -> Result<()> {
    let tmpdir = tempfile::tempdir()?;
    let decrypted_tf_state_path = tmpdir.path().join("terraform.tfstate");
    let decrypted_tf_state_path_str = decrypted_tf_state_path
        .to_str()
        .wrap_err("could not convert path to string")?;

    encrypt::decrypt(sh, "terraform.tfstate.enc", decrypted_tf_state_path_str)?;
    let before_hash = sha2::Sha256::digest(sh.read_file(decrypted_tf_state_path_str)?);

    if !decrypted_tf_state_path.exists() {
        return Err(eyre::eyre!("could not find decrypted tf state file"));
    }

    let state_config = format!("-backend-config=\"path={}\"", decrypted_tf_state_path_str);
    println!("state config: {state_config}");

    // use command instead of xshell because to deal with interactive prompts
    let result = Command::new("tofu")
        .arg(cmd)
        .arg(state_config)
        .args(args)
        .spawn()
        .wrap_err_with(|| {
            format!(
                "could not spawn terraform with args: {args:?} in: {decrypted_tf_state_path_str:?}"
            )
        })?
        .wait()
        .wrap_err("could not wait for terraform")?;

    if !result.success() {
        sh.remove_path(decrypted_tf_state_path_str)?;
        return Err(eyre::eyre!("terraform {cmd} failed"));
    };

    let after_hash = sha2::Sha256::digest(sh.read_file(decrypted_tf_state_path_str)?);
    if before_hash != after_hash {
        encrypt::encrypt(sh, decrypted_tf_state_path_str, "terraform.tfstate.enc")?;

        let tfstate_parent = Path::new(decrypted_tf_state_path_str)
            .parent()
            .wrap_err("could not get parent of input file")?;

        sh.remove_path(tfstate_parent)?;

        let tfstate = tfstate_parent.join("terraform.tfstate");
        let tfstate_backup = tfstate.join("terraform.tfstate.backup");

        sh.remove_path(tfstate_backup)?;
        sh.remove_path(tfstate)?;
    }

    sh.remove_path(decrypted_tf_state_path)?;

    Ok(())
}

fn encrypt(sh: &Shell, input_file: &str) -> Result<()> {
    init(sh)?;
    let output_file = Path::new(input_file).with_extension("enc").to_path_buf();
    encrypt::encrypt(sh, input_file, output_file.to_string_lossy().as_ref())
}

fn decrypt(sh: &Shell, input_file: &str) -> Result<()> {
    let input_file_path =
        std::fs::canonicalize(input_file).wrap_err("could not canonicalize input path")?;

    let parent = input_file_path
        .parent()
        .wrap_err("could not get parent of input file")?;

    let output_file = if input_file.ends_with(".enc") {
        input_file.trim_end_matches(".enc").to_string()
    } else {
        input_file.to_string() + ".dec"
    };

    let output_path = parent.join(output_file);

    encrypt::decrypt(sh, input_file, &output_path)
}
