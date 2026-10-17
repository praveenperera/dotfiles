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
use xshell::Shell;

use crate::util;

/// Public key of an encrypted file's 1Password item
///
/// Fetching it before touching any file means a locked or failing 1Password cannot leave the
/// output half-written, and lets callers fetch it before work they must not lose
pub struct FileRecipient {
    secret_name: String,
    pubkey: Recipient,
}

impl FileRecipient {
    /// Reads the item name from `encrypted_file`'s header and fetches the item's public key
    pub fn fetch(sh: &Shell, encrypted_file: &str) -> Result<Self> {
        let secret_name = secret_name(sh, encrypted_file)?;
        let pubkey = util::pass_read(sh, &secret_name, "public_key")?
            .parse()
            .map_err(|_| eyre::eyre!("could not parse public key from password store"))?;

        Ok(Self {
            secret_name,
            pubkey,
        })
    }

    /// Encrypts `input` into `output`, replacing `output` atomically so a failure keeps the old
    /// ciphertext
    pub fn encrypt(&self, sh: &Shell, input: &str, output: &str) -> Result<()> {
        let original = sh.read_file(input)?;
        if original.is_empty() {
            return Err(eyre::eyre!("{input} is empty"));
        }

        let encrypted = age::encrypt(&self.pubkey, original.as_bytes())?;

        // the temp file must share the output's directory for the rename to be atomic
        let output_path = Path::new(output);
        let output_dir = output_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));

        let mut file = tempfile::NamedTempFile::new_in(output_dir)
            .wrap_err_with(|| format!("could not create temp file next to {output}"))?;

        file.write_all(create_header(&self.secret_name).as_bytes())?;
        file.write_all(b"\n")?;
        file.write_all(&encrypted)?;
        file.as_file().sync_all()?;

        file.persist(output_path)
            .wrap_err_with(|| format!("could not replace {output}"))?;

        Ok(())
    }
}

/// Encrypts `input` with the key named in `output`'s header, which must already exist
pub fn encrypt(sh: &Shell, input: &str, output: &str) -> Result<()> {
    FileRecipient::fetch(sh, output)?.encrypt(sh, input, output)
}

pub fn create_secret_and_files(sh: &Shell, secret_prefix: &str, file_name: &str) -> Result<()> {
    let id = util::random_base32(10);
    let secret_name = format!("{secret_prefix}-{id}");

    let key = age::x25519::Identity::generate();
    let password = key.to_string();
    let pubkey = key.to_public().to_string();

    util::pass_create(
        sh,
        &secret_name,
        &[
            ("password", password.expose_secret()),
            ("public_key", &pubkey),
        ],
    )?;

    sh.write_file(file_name, create_header(&secret_name))?;
    log::debug!("created secret: {secret_name}, at {file_name}");

    Ok(())
}

pub fn decrypt(sh: &Shell, input: &str, output: impl AsRef<Path>) -> Result<()> {
    let output = output.as_ref();

    println!("decrypting {input} to {output:?}");
    if sh.path_exists(output) {
        return Err(eyre::eyre!("{output:?} already exists"));
    }

    let secret_name =
        secret_name(sh, input).wrap_err("Could not get secret name for input file")?;

    let encrypted = read_encrypted_file(input)?;
    if encrypted.is_empty() {
        sh.write_file(output, "")
            .wrap_err("could not write empty file")?;

        return Ok(());
    }

    let key: Identity = util::pass_read(sh, &secret_name, "password")?
        .parse()
        .map_err(|_| eyre::eyre!("could not parse private key from password store"))?;

    let decrypted = {
        let decryptor = age::Decryptor::new(&encrypted[..])?;

        let mut decrypted = vec![];
        let mut reader = decryptor
            .decrypt(iter::once(&key as &dyn age::Identity))
            .wrap_err("could not decrypt")?;

        reader
            .read_to_end(&mut decrypted)
            .wrap_err("could not read decrypted file")?;

        String::from_utf8(decrypted)?
    };

    sh.write_file(output, decrypted)?;
    println!("decrypted {input} to {output:?}");

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

#[cfg(test)]
mod tests {
    use std::{io::Read as _, iter};

    use age::x25519::Identity;
    use xshell::Shell;

    use super::{read_encrypted_file, FileRecipient};

    #[test]
    fn encrypt_replaces_output_atomically_and_keeps_it_on_failure() {
        let sh = Shell::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("state");
        let output = dir.path().join("state.enc");
        let (input, output) = (input.to_str().unwrap(), output.to_str().unwrap());

        let key = Identity::generate();
        let recipient = FileRecipient {
            secret_name: "test-key".to_string(),
            pubkey: key.to_public(),
        };

        std::fs::write(output, "old ciphertext").unwrap();
        std::fs::write(input, "").unwrap();
        assert!(recipient.encrypt(&sh, input, output).is_err());
        assert_eq!(std::fs::read_to_string(output).unwrap(), "old ciphertext");

        std::fs::write(input, "plaintext").unwrap();
        recipient.encrypt(&sh, input, output).unwrap();

        let encrypted = read_encrypted_file(output).unwrap();
        let mut decrypted = String::new();
        age::Decryptor::new(&encrypted[..])
            .unwrap()
            .decrypt(iter::once(&key as &dyn age::Identity))
            .unwrap()
            .read_to_string(&mut decrypted)
            .unwrap();

        assert!(std::fs::read(output)
            .unwrap()
            .starts_with(b"!!CMD!!ID!!test-key\n"));
        assert_eq!(decrypted, "plaintext");

        // no temp file is left next to the output
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    }
}
