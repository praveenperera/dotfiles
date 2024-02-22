use rand::{
    distributions::{Alphanumeric, DistString, Uniform},
    Rng,
};
use xshell::cmd;

pub const VAULT: &str = "CLI";

pub fn random_ascii(length: usize) -> String {
    const CHARSET: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcdefghijklmnopqrstuvwxyz!@#%^&|-_=+*";
    let mut rng = rand::thread_rng();
    let char_num = Uniform::from(0..CHARSET.len());

    (0..length)
        .map(|_| CHARSET[rng.sample(char_num)] as char)
        .collect()
}

pub fn random_alpha(length: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mut rng = rand::thread_rng();
    let char_num = Uniform::from(0..CHARSET.len());

    (0..length)
        .map(|_| CHARSET[rng.sample(char_num)] as char)
        .collect()
}

pub fn random_base32(length: usize) -> String {
    const CHARSET: &[u8] = b"23456789abcdefghjkmnopqrstuvwxyz";
    let mut rng = rand::thread_rng();
    let char_num = Uniform::from(0..CHARSET.len());

    (0..length)
        .map(|_| CHARSET[rng.sample(char_num)] as char)
        .collect()
}

pub fn random_pin(length: usize) -> String {
    const CHARSET: &[u8] = b"01234567890";
    let mut rng = rand::thread_rng();
    let char_num = Uniform::from(0..CHARSET.len());

    (0..length)
        .map(|_| CHARSET[rng.sample(char_num)] as char)
        .collect()
}

pub fn random_alpha_numeric(length: usize) -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), length)
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
