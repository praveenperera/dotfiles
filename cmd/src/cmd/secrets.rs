use clap::{Parser, Subcommand};
use std::ffi::OsString;
use std::path::PathBuf;

use eyre::Result;
use xshell::{cmd, Shell};

use crate::util;
use crate::util::VAULT;

#[derive(Debug, Clone, Parser)]
pub struct Secrets {
    #[command(subcommand)]
    pub subcommand: SecretsCmd,
}

#[derive(Debug, Clone, Subcommand)]
#[command(arg_required_else_help = true)]
pub enum SecretsCmd {
    /// Generate a random secret
    Gen {
        /// Length of the secret to generate
        #[arg(long, short)]
        length: Option<usize>,

        /// Generate without symbols (alphanumeric only)
        #[arg(long, short = 'n')]
        no_symbols: bool,

        /// Generate lowercase only
        #[arg(long, short = 'L')]
        lowercase: bool,

        /// Generate letters only (no numbers)
        #[arg(long, short = 'a')]
        letters_only: bool,
    },

    /// Get a secret from vault
    #[command(arg_required_else_help = true)]
    Get {
        /// Secret key to retrieve
        secret: String,

        /// Name of the secret store
        secret_name: Option<String>,
    },

    /// Save secrets to local files
    Save,

    /// Update secrets in vault
    #[command(arg_required_else_help = true)]
    Update {
        /// Secret to update (or "all" for all secrets)
        secret: String,
    },
}

static SECRET_NAME: &str = "cmd_secrets";
static SECRETS: [&str; 2] = ["ln.yaml", "sq.yaml"];

pub fn run(sh: &Shell, args: &[OsString]) -> Result<()> {
    use clap::Parser;
    let flags = Secrets::parse_from(args);
    run_with_flags(sh, flags)
}

pub fn run_with_flags(sh: &Shell, flags: Secrets) -> Result<()> {
    match flags.subcommand {
        SecretsCmd::Gen {
            length,
            no_symbols,
            lowercase,
            letters_only,
        } => {
            gen(sh, length, no_symbols, lowercase, letters_only)?;
        }
        SecretsCmd::Get {
            secret,
            secret_name,
        } => {
            get(sh, secret_name.as_deref(), &secret)?;
        }
        SecretsCmd::Save => {
            save(sh)?;
        }
        SecretsCmd::Update { secret } => {
            update(sh, &secret)?;
        }
    }

    Ok(())
}

pub fn gen(
    _sh: &Shell,
    length: Option<usize>,
    no_symbols: bool,
    lowercase: bool,
    letters_only: bool,
) -> Result<()> {
    let length = length.unwrap_or(32);

    let string = match (letters_only, no_symbols, lowercase) {
        (true, _, true) => util::random_alpha_lower(length),
        (true, _, false) => util::random_alpha(length),
        (false, true, true) => util::random_alpha_numeric_lower(length),
        (false, true, false) => util::random_alpha_numeric(length),
        (false, false, true) => util::random_ascii(length).to_lowercase(),
        (false, false, false) => util::random_ascii(length),
    };

    print!("{string}");

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
