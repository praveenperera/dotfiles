use clap::{Parser, Subcommand};
use std::{ffi::OsString, process::Command};

use eyre::{Context as _, ContextCompat as _, Result};
use log::debug;
use sha2::Digest;
use xshell::Shell;

use crate::encrypt::{self, FileRecipient};

const ENCRYPTED_STATE: &str = "terraform.tfstate.enc";

#[derive(Debug, Clone, Parser)]
pub struct Terraform {
    #[command(subcommand)]
    pub subcommand: TerraformCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum TerraformCmd {
    /// Run a tofu command against the decrypted state (default for unknown subcommands)
    #[command(arg_required_else_help = true)]
    Run {
        command: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Create the encrypted state file if needed and run tofu init
    Init,

    /// Encrypt a plaintext state file to <file>.enc
    #[command(visible_alias = "enc")]
    Encrypt { file: Option<String> },

    /// Decrypt an encrypted state file
    #[command(visible_alias = "dec")]
    Decrypt { file: Option<String> },

    /// Any other subcommand, forwarded to tofu like `run`
    #[command(external_subcommand)]
    Tofu(Vec<String>),
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
            let file = file.as_deref().unwrap_or(ENCRYPTED_STATE);
            decrypt(sh, file)?;
        }
        TerraformCmd::Run { command, args } => {
            run_tofu_cmd(sh, &command, &args)?;
        }
        TerraformCmd::Tofu(command_and_args) => {
            let (command, args) = command_and_args
                .split_first()
                .wrap_err("missing tofu command")?;

            run_tofu_cmd(sh, command, args)?;
        }
    }

    Ok(())
}

fn init(sh: &Shell) -> Result<()> {
    ensure_key_file(sh, ENCRYPTED_STATE)?;

    let terraform_state = encrypt::read_encrypted_file(ENCRYPTED_STATE)?;
    if terraform_state.is_empty() {
        eprintln!("{ENCRYPTED_STATE} is empty");
    } else {
        eprintln!("{ENCRYPTED_STATE} is not empty");
    }

    run_tofu_cmd(sh, "init", &[])?;

    Ok(())
}

/// Creates `encrypted_file` with a new 1Password key if it does not exist yet, since the
/// file's header names the key used to encrypt it
fn ensure_key_file(sh: &Shell, encrypted_file: &str) -> Result<()> {
    if sh.path_exists(encrypted_file) {
        eprintln!("{encrypted_file} already exists");
    } else {
        eprintln!("{encrypted_file} does not exist, creating...");
        encrypt::create_secret_and_files(sh, "terraform-state-pw", encrypted_file)?;
    }

    Ok(())
}

fn run_tofu_cmd(sh: &Shell, cmd: &str, args: &[String]) -> Result<()> {
    // fetched before tofu runs so saving changed state cannot fail on a locked 1Password
    let recipient = FileRecipient::fetch(sh, ENCRYPTED_STATE)?;

    // dropping the guard deletes the plaintext state and tofu's backup of it on every return path
    let state_dir = tempfile::tempdir()?;
    let state_path = state_dir.path().join("terraform.tfstate");
    let state_path_str = state_path
        .to_str()
        .wrap_err("could not convert path to string")?;

    encrypt::decrypt(sh, ENCRYPTED_STATE, state_path_str)?;
    let before_hash = sha2::Sha256::digest(sh.read_file(state_path_str)?).to_vec();

    let state_arg = if cmd == "init" {
        format!("-backend-config=path={state_path_str}")
    } else {
        format!("-state={state_path_str}")
    };
    println!("state arg: {state_arg}");

    // use command instead of xshell because to deal with interactive prompts
    let mut child = Command::new("tofu")
        .arg(cmd)
        .arg(state_arg)
        .args(args)
        .spawn()
        .wrap_err_with(|| format!("could not spawn tofu {cmd} with args: {args:?}"))?;

    // Ctrl-C, a closed terminal or a group-wide kill reach tofu too, and tofu shuts down
    // cleanly, so cmd must survive them to save the state and delete the plaintext. Ignored
    // only after spawning so tofu keeps the default dispositions
    // held until the state is saved, so a repeated Ctrl-C cannot interrupt the save either
    let _interrupts = IgnoreInterrupts::new();
    let status = child.wait().wrap_err("could not wait for tofu")?;

    // a partially failed or interrupted apply can still have changed the state, so save it
    // before failing
    if let Err(err) = save_changed_state(sh, &recipient, state_path_str, &before_hash) {
        // the plaintext is the only copy of the new state, so keep it instead of deleting it
        let kept = state_dir.keep();
        return Err(err.wrap_err(format!(
            "changed state was NOT saved to {ENCRYPTED_STATE}; the plaintext state was kept in {}\n\
             check it, copy it here as terraform.tfstate, run `tf enc`, then delete both plaintext copies",
            kept.display()
        )));
    }

    if !status.success() {
        return Err(eyre::eyre!("tofu {cmd} failed"));
    }

    Ok(())
}

/// Re-encrypts the state if tofu changed it, refusing to replace the last good ciphertext
/// with a state file that does not parse, e.g. one truncated by a crash mid-write
fn save_changed_state(
    sh: &Shell,
    recipient: &FileRecipient,
    state_path: &str,
    before_hash: &[u8],
) -> Result<()> {
    let after_state = sh
        .read_file(state_path)
        .wrap_err("could not read state after tofu")?;

    if before_hash == sha2::Sha256::digest(&after_state).as_slice() {
        return Ok(());
    }

    serde_json::from_str::<serde_json::Value>(&after_state)
        .wrap_err("state written by tofu is not valid JSON")?;

    recipient
        .encrypt(sh, state_path, ENCRYPTED_STATE)
        .wrap_err_with(|| format!("could not save changed state to {ENCRYPTED_STATE}"))
}

/// Signals that would kill cmd while tofu still holds the plaintext state
const INTERRUPT_SIGNALS: [libc::c_int; 3] = [libc::SIGINT, libc::SIGHUP, libc::SIGTERM];

/// Ignores [`INTERRUPT_SIGNALS`] in this process until dropped, restoring the previous handlers
struct IgnoreInterrupts([libc::sighandler_t; INTERRUPT_SIGNALS.len()]);

impl IgnoreInterrupts {
    fn new() -> Self {
        // SAFETY: SIG_IGN installs no Rust handler, and the previous dispositions are restored
        // on drop
        Self(INTERRUPT_SIGNALS.map(|signal| unsafe { libc::signal(signal, libc::SIG_IGN) }))
    }
}

impl Drop for IgnoreInterrupts {
    fn drop(&mut self) {
        for (signal, previous) in INTERRUPT_SIGNALS.into_iter().zip(self.0) {
            // SAFETY: restores the disposition returned by `signal` in `new`
            unsafe {
                libc::signal(signal, previous);
            }
        }
    }
}

fn encrypt(sh: &Shell, input_file: &str) -> Result<()> {
    // appended instead of `with_extension` so terraform.tfstate maps to terraform.tfstate.enc
    let output_file = format!("{input_file}.enc");

    // checked first so a mistyped input never creates a 1Password key and an orphan .enc
    if !sh.path_exists(input_file) {
        return Err(eyre::eyre!("{input_file} does not exist"));
    }

    ensure_key_file(sh, &output_file)?;

    encrypt::encrypt(sh, input_file, &output_file)
}

fn decrypt(sh: &Shell, input_file: &str) -> Result<()> {
    // derived from the input path, so it lands next to the input
    let output_file = match input_file.strip_suffix(".enc") {
        Some(stem) => stem.to_string(),
        None => format!("{input_file}.dec"),
    };

    encrypt::decrypt(sh, input_file, output_file)
}

#[cfg(test)]
mod tests {
    use clap::Parser as _;

    use super::{Terraform, TerraformCmd};

    fn parse(args: &[&str]) -> TerraformCmd {
        Terraform::try_parse_from(args).unwrap().subcommand
    }

    #[test]
    fn unknown_subcommands_forward_to_tofu_like_run() {
        let TerraformCmd::Tofu(forwarded) = parse(&["tf", "plan", "-out", "x"]) else {
            panic!("plan should forward to tofu");
        };

        let TerraformCmd::Run { command, args } = parse(&["tf", "run", "plan", "-out", "x"]) else {
            panic!("run should parse as run");
        };

        assert_eq!(forwarded, ["plan", "-out", "x"]);
        assert_eq!(command, "plan");
        assert_eq!(args, ["-out", "x"]);
    }

    #[test]
    fn named_subcommands_and_aliases_still_parse() {
        assert!(matches!(parse(&["tf", "init"]), TerraformCmd::Init));
        assert!(matches!(
            parse(&["tf", "enc"]),
            TerraformCmd::Encrypt { file: None }
        ));
        assert!(matches!(
            parse(&["tf", "dec", "x.enc"]),
            TerraformCmd::Decrypt { file: Some(_) }
        ));
    }
}
