use colored::Colorize as _;
use eyre::{eyre, Result};
use rand::{
    distr::{Alphanumeric, SampleString as _, Uniform},
    Rng,
};
use xshell::{cmd, Shell};

use crate::cmd::flags::{Cmd, CmdCmd};

pub const VAULT: &str = "CLI";

pub fn random_ascii(length: usize) -> String {
    const CHARSET: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcdefghijklmnopqrstuvwxyz!@#%^&|-_=+*";
    let mut rng = rand::rng();
    let char_num = Uniform::new(0, CHARSET.len()).expect("invalid char set");

    (0..length)
        .map(|_| CHARSET[rng.sample(char_num)] as char)
        .collect()
}

pub fn random_alpha(length: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mut rng = rand::rng();
    let char_num = Uniform::new(0, CHARSET.len()).expect("invalid char set");

    (0..length)
        .map(|_| CHARSET[rng.sample(char_num)] as char)
        .collect()
}

pub fn random_base32(length: usize) -> String {
    const CHARSET: &[u8] = b"23456789abcdefghjkmnopqrstuvwxyz";
    let mut rng = rand::rng();
    let char_num = Uniform::new(0, CHARSET.len()).expect("invalid char set");

    (0..length)
        .map(|_| CHARSET[rng.sample(char_num)] as char)
        .collect()
}

pub fn random_pin(length: usize) -> String {
    const CHARSET: &[u8] = b"01234567890";
    let mut rng = rand::rng();
    let char_num = Uniform::new(0, CHARSET.len()).expect("invalid char set");

    (0..length)
        .map(|_| CHARSET[rng.sample(char_num)] as char)
        .collect()
}

pub fn random_alpha_numeric(length: usize) -> String {
    Alphanumeric.sample_string(&mut rand::rng(), length)
}

pub fn pass_edit(
    sh: &xshell::Shell,
    secret_name: &str,
    key: &str,
    password: &str,
) -> eyre::Result<()> {
    // add password to item
    Ok(cmd!(
        sh,
        "op item edit {secret_name} {key}={password} --vault {VAULT}"
    )
    .run()?)
}

pub fn pass_read(sh: &xshell::Shell, secret_name: &str, key: &str) -> eyre::Result<String> {
    Ok(cmd!(sh, "op read op://{VAULT}/{secret_name}/{key}").read()?)
}

pub fn hex_to_rgb(hex: &str) -> Result<(f32, f32, f32), std::num::ParseIntError> {
    let hex = hex.trim_start_matches('#');
    let num = u32::from_str_radix(hex, 16)?;

    let r = (num >> 16) as u8;
    let g = (num >> 8) as u8;
    let b = num as u8;

    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    Ok((r, g, b))
}

pub fn has_tool(sh: &Shell, tool: &str) -> bool {
    cmd!(sh, "command -v {tool}").quiet().output().is_ok()
}

// xflags utility functions

pub fn extract_unknown_command_from_args(args: &[std::ffi::OsString]) -> Option<String> {
    // get the first argument which should be the subcommand
    args.last().and_then(|s| s.to_str()).map(|s| s.to_string())
}

pub fn handle_xflags_error<T>(
    result: xflags::Result<T>,
    args: &[std::ffi::OsString],
    help_txt: &str,
) -> Result<T> {
    match result {
        Ok(flags) => Ok(flags),
        Err(err) => {
            let unknown_cmd = extract_unknown_command_from_args(args);
            match unknown_cmd.as_deref() {
                Some("help" | "-h" | "--help") => {
                    // default help
                    if args.len() < 2 {
                        println!("{help_txt}");
                        std::process::exit(0);
                    }

                    let mut args = args.to_vec();
                    args.pop();

                    let cmd = Cmd::from_vec(args.to_vec())?;
                    let help = match cmd.subcommand {
                        CmdCmd::Bootstrap(cmd) => cmd.help(),
                        CmdCmd::Release(release) => release.help(),
                        CmdCmd::Config(config) => config.help(),
                        CmdCmd::Gcloud(gcloud) => gcloud.help(),
                        CmdCmd::Secret(secret) => secret.help(),
                        CmdCmd::Terraform(terraform) => terraform.help(),
                        CmdCmd::Vault(vault) => vault.help(),
                        CmdCmd::Generate(generate) => generate.help(),
                    };

                    println!("{}\n\n", help);
                    std::process::exit(0);
                }
                Some(unknown_cmd) => {
                    let suggestions = did_you_mean(unknown_cmd, help_txt);
                    println!("unknown command: {}", unknown_cmd.red());
                    if !suggestions.is_empty() {
                        println!("\ndid you mean: {}\n", suggestions.join(", ").yellow());
                    }
                    println!("{help_txt}\n");
                    Err(eyre!("failed to parse arguments: {err}"))
                }
                None => {
                    println!("need args\n");
                    println!("{help_txt}\n");
                    Err(eyre!("no args provided"))
                }
            }
        }
    }
}

pub fn extract_commands_from_help(help_text: &str) -> Vec<String> {
    let mut commands = Vec::new();

    // parse the help text to extract subcommands
    // look for lines that start with spaces followed by command names
    for line in help_text.lines() {
        let trimmed = line.trim_start();
        if line.starts_with("  ") && !line.starts_with("   ") && !trimmed.starts_with('-') {
            // this looks like a command line (starts with 2 spaces, not 3+, not a flag)
            if let Some(command) = trimmed.split_whitespace().next() {
                // extract the main command name and any aliases
                if command.contains(',') {
                    // handle commands with aliases like "config, cfg"
                    for cmd in command.split(',') {
                        let cmd = cmd.trim();
                        if !cmd.is_empty() {
                            commands.push(cmd.to_string());
                        }
                    }
                } else {
                    commands.push(command.to_string());
                }
            }
        }
    }

    commands.sort();
    commands.dedup();
    commands
}

pub fn did_you_mean(user_text: &str, help_text: &str) -> Vec<String> {
    use textdistance::nstr::damerau_levenshtein;

    let available_commands = extract_commands_from_help(help_text);

    let mut suggestions = available_commands
        .iter()
        .filter(|name| !name.starts_with(user_text))
        .map(|name| (name.as_str(), damerau_levenshtein(user_text, name)))
        .map(|(name, distance)| (name, distance * 100.0))
        .map(|(name, distance)| (name, distance as usize))
        .filter(|(_, distance)| *distance <= 90)
        .collect::<Vec<_>>();

    suggestions.sort_unstable_by(|a, b| a.1.cmp(&b.1));

    let starts_with: Vec<String> = available_commands
        .iter()
        .filter(|name| name.starts_with(user_text))
        .map(Into::into)
        .collect();

    let suggestions: Vec<String> = suggestions
        .into_iter()
        .map(|(name, _)| name.to_string())
        .take(3)
        .collect();

    starts_with.into_iter().chain(suggestions).collect()
}
