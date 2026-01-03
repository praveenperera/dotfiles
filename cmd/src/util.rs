use eyre::Result;
use rand::{
    distr::{Alphanumeric, SampleString as _, Uniform},
    Rng,
};
use xshell::{cmd, Shell};

pub const VAULT: &str = "CLI";

fn random_with_charset(length: usize, charset: &[u8]) -> String {
    let mut rng = rand::rng();
    let dist = Uniform::new(0, charset.len()).expect("invalid charset");
    (0..length)
        .map(|_| charset[rng.sample(dist)] as char)
        .collect()
}

pub fn random_ascii(length: usize) -> String {
    random_with_charset(
        length,
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcdefghijklmnopqrstuvwxyz!@#%^&|-_=+*",
    )
}

pub fn random_alpha(length: usize) -> String {
    random_with_charset(
        length,
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
    )
}

pub fn random_base32(length: usize) -> String {
    random_with_charset(length, b"23456789abcdefghjkmnopqrstuvwxyz")
}

pub fn random_pin(length: usize) -> String {
    random_with_charset(length, b"0123456789")
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
