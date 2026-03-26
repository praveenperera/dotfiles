use clap::{Args, Subcommand};
use eyre::{eyre, Result};
use std::ffi::OsString;
use std::path::PathBuf;
use xshell::Shell;

#[derive(Debug, Clone, Args)]
pub struct Codex {
    #[command(subcommand)]
    pub subcommand: CodexCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum CodexCmd {
    /// Launch codex with a specific profile
    Launch {
        /// Profile name
        profile: String,

        /// Arguments to pass to codex
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },

    /// Login and save a new profile
    Login {
        /// Profile name to save
        profile: String,
    },

    /// List available profiles
    #[command(visible_alias = "ls")]
    List,
}

fn codex_dir() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME must be set");
    PathBuf::from(home).join(".codex")
}

fn profiles_dir() -> PathBuf {
    codex_dir().join("profiles")
}

fn auth_path() -> PathBuf {
    codex_dir().join("auth.json")
}

pub fn run_with_flags(_sh: &Shell, flags: Codex) -> Result<()> {
    match flags.subcommand {
        CodexCmd::Launch { profile, args } => launch(&profile, &args),
        CodexCmd::Login { profile } => login(&profile),
        CodexCmd::List => list(),
    }
}

fn launch(profile: &str, args: &[OsString]) -> Result<()> {
    let profile_auth = profiles_dir().join(profile).join("auth.json");
    if !profile_auth.exists() {
        return Err(eyre!(
            "Profile '{profile}' not found. Run: cmd codex login {profile}"
        ));
    }

    let auth = auth_path();
    std::fs::copy(&profile_auth, &auth)?;

    let status = std::process::Command::new("codex").args(args).status()?;

    // save back refreshed tokens
    if auth.exists() {
        std::fs::copy(&auth, &profile_auth)?;
    }

    std::process::exit(status.code().unwrap_or(1));
}

fn login(profile: &str) -> Result<()> {
    let profile_dir = profiles_dir().join(profile);
    std::fs::create_dir_all(&profile_dir)?;

    std::process::Command::new("codex")
        .arg("logout")
        .status()
        .ok();

    let status = std::process::Command::new("codex").arg("login").status()?;

    if !status.success() {
        return Err(eyre!("codex login failed"));
    }

    let auth = auth_path();
    if !auth.exists() {
        return Err(eyre!("No auth.json found after login"));
    }

    std::fs::copy(&auth, profile_dir.join("auth.json"))?;
    println!("Saved codex profile: {profile}");
    Ok(())
}

fn list() -> Result<()> {
    let dir = profiles_dir();
    if !dir.exists() {
        println!("No profiles. Run: cmd codex login <name>");
        return Ok(());
    }

    let mut entries: Vec<String> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().join("auth.json").exists())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    if entries.is_empty() {
        println!("No profiles. Run: cmd codex login <name>");
        return Ok(());
    }

    entries.sort();
    for name in entries {
        println!("{name}");
    }
    Ok(())
}
