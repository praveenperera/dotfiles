use std::{
    fs::File,
    io::{BufRead, Read as _, Write},
    iter,
    path::Path,
};

use age::{
    secrecy::ExposeSecret as _,
    x25519::{Identity, Recipient},
};

use eyre::{Context as _, Result};
use xshell::{cmd, Shell};

use crate::util;

pub fn encrypt(sh: &Shell, input: &str, output: &str) -> Result<()> {
    let secret_name = secret_name(sh, output)?;
    let original = sh.read_file(input)?;

    if original.is_empty() {
        return Err(eyre::eyre!("{input} is empty"));
    }

    if sh.path_exists(output) {
        sh.remove_path(output)?;
    }

    let pubkey: Recipient = util::pass_read(sh, &secret_name, "public_key")?
        .parse()
        .map_err(|_| eyre::eyre!("could not parse public key from password store"))?;

    let encrypted = age::encrypt(&pubkey, original.as_bytes())?;

    let header = create_header(&secret_name);

    sh.remove_path(output)?;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(output)?;

    file.write_all(header.as_bytes())?;
    file.write_all(b"\n")?;
    file.write_all(&encrypted)?;

    Ok(())
}

pub fn create_secret_and_files(sh: &Shell, secret_prefix: &str, file_name: &str) -> Result<()> {
    let id = util::random_base32(10);
    let secret_name = format!("{secret_prefix}-{id}");

    let key = age::x25519::Identity::generate();
    let password = key.to_string();
    let pubkey = key.to_public();

    // create item
    cmd!(
        sh,
        "op item create --category='Secure Note' --vault CLI --title {secret_name}"
    )
    .run()?;

    util::pass_edit(sh, &secret_name, "password", password.expose_secret())?;
    util::pass_edit(sh, &secret_name, "public_key", &pubkey.to_string())?;

    sh.write_file(file_name, create_header(&secret_name))?;
    log::debug!("created secret: {secret_name}, at {file_name}");

    Ok(())
}

pub fn decrypt(sh: &Shell, input: &str, output: &str) -> Result<()> {
    println!("decrypting {input} to {output}");
    if sh.path_exists(output) {
        return Err(eyre::eyre!("{output} already exists"));
    }

    let secret_name =
        secret_name(sh, input).wrap_err("Could not get secret name for input file")?;

    let encrypted = read_encrypted_file(input)?;

    let key: Identity = util::pass_read(sh, &secret_name, "password")?
        .parse()
        .map_err(|_| eyre::eyre!("could not parse private key from password store"))?;

    let decrypted = {
        let decryptor = age::Decryptor::new(&encrypted[..])?;

        let mut decrypted = vec![];
        let mut reader = decryptor.decrypt(iter::once(&key as &dyn age::Identity))?;
        reader.read_to_end(&mut decrypted)?;

        String::from_utf8(decrypted)?
    };

    sh.write_file(output, decrypted)?;

    Ok(())
}

pub fn read_encrypted_file(path_str: &str) -> Result<Vec<u8>> {
    let path = Path::new(path_str);
    let file =
        File::open(path).wrap_err_with(|| format!("could not open file: {}", path.display()))?;

    let mut buf_reader = std::io::BufReader::new(file);

    // Skip the first line without allocating
    for byte in buf_reader.by_ref().bytes() {
        if byte? == b'\n' {
            break;
        }
    }

    let mut output = Vec::new();
    buf_reader.read_to_end(&mut output)?;

    Ok(output)
}

fn secret_name(sh: &Shell, path: &str) -> Result<String> {
    if !sh.path_exists(path) {
        return Err(eyre::eyre!("{path} does not exist"));
    }

    let path = Path::new(path);
    let file = File::open(path)?;
    let mut lines = std::io::BufReader::new(file).lines();

    let line_1 = lines
        .next()
        .ok_or_else(|| eyre::eyre!("no lines in file"))??;

    if !line_1.starts_with("!!CMD!!ID!!") {
        return Err(eyre::eyre!("first line does not start with !!CMD!!ID!!"));
    }

    let secret_name = line_1.trim_start_matches("!!CMD!!ID!!");
    Ok(secret_name.to_string())
}

fn create_header(secret_name: &str) -> String {
    format!("!!CMD!!ID!!{secret_name}")
}
