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
use eyre::Result;
use xshell::{cmd, Shell};

use crate::util;

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
        // ["plan", args @ ..] => {
        //     cmd!(sh, "terraform plan").run()?;
        // }
        // ["apply", args @ ..] => {
        //     cmd!(sh, "terraform apply").run()?;
        // }
        // ["destroy", args @ ..] => {
        //     cmd!(sh, "terraform destroy").run()?;
        // }
        [unknown, ..] => {
            eprintln!("unknown terraform command ({unknown})");
        }
    }

    Ok(())
}

fn init(sh: &Shell, _args: &[&str]) -> Result<()> {
    if sh.path_exists("terraform.tfstate.enc") {
        eprintln!("terraform.tfstate.enc already exists");
    } else {
        eprintln!("terraform.tfstate.enc does not exist, creating...");
        let id = util::random_base32(10);
        let secret_name = format!("terraform-state-pw-{id}");

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

        // create terraform.tfstate.enc file
        sh.write_file("terraform.tfstate.enc", create_header(&secret_name))?;
    }

    let terraform_state = read_encrypted_file("terraform.tfstate.enc")?;

    if terraform_state.is_empty() {
        eprintln!("terraform.tfstate.enc is empty");
        cmd!(sh, "terraform init").run()?;
    } else {
        eprintln!("terraform.tfstate.enc is not empty");
    }

    Ok(())
}

pub fn encrypt(sh: &Shell) -> Result<()> {
    encrypt_internal(sh, "terraform.tfstate", "terraform.tfstate.enc")
}

fn encrypt_internal(sh: &Shell, input: &str, output: &str) -> Result<()> {
    let tf_state = sh.read_file(input)?;

    if tf_state.is_empty() {
        return Err(eyre::eyre!("{input} is empty"));
    }

    if sh.path_exists(output) {
        return Err(eyre::eyre!("{output} already exists"));
    }

    init(sh, &[])?;
    let secret_name = secret_name(sh, output)?;
    let pubkey: Recipient = util::pass_read(sh, &secret_name, "public_key")?
        .parse()
        .map_err(|_| eyre::eyre!("could not parse public key from password store"))?;

    let encrypted = {
        let encryptor = age::Encryptor::with_recipients(vec![Box::new(pubkey.clone())])
            .expect("we provided a recipient");

        let mut encrypted = vec![];
        let mut writer = encryptor.wrap_output(&mut encrypted)?;
        writer.write_all(tf_state.as_bytes())?;
        writer.finish()?;

        encrypted
    };

    let header = create_header(&secret_name);

    sh.remove_path("terraform.tfstate.enc")?;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open("terraform.tfstate.enc")?;

    file.write_all(header.as_bytes())?;
    file.write_all(b"\n")?;
    file.write_all(&encrypted)?;

    sh.remove_path("terraform.tfstate")?;

    Ok(())
}

fn decrypt(sh: &Shell) -> Result<()> {
    if sh.path_exists("terraform.tfstate") {
        return Err(eyre::eyre!("terraform.tfstate already exists"));
    }

    let secret_name = secret_name(sh, "terraform.tfstate.enc")?;
    let encrypted = read_encrypted_file("terraform.tfstate.enc")?;

    let key: Identity = util::pass_read(sh, &secret_name, "password")?
        .parse()
        .map_err(|_| eyre::eyre!("could not parse private key from password store"))?;

    let decrypted = {
        let decryptor = match age::Decryptor::new(&encrypted[..])? {
            age::Decryptor::Recipients(d) => d,
            _ => unreachable!(),
        };

        let mut decrypted = vec![];
        let mut reader = decryptor.decrypt(iter::once(&key as &dyn age::Identity))?;
        reader.read_to_end(&mut decrypted)?;

        String::from_utf8(decrypted)?
    };

    sh.write_file("terraform.tfstate", decrypted)?;

    Ok(())
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

fn read_encrypted_file(path: &str) -> Result<Vec<u8>> {
    let path = Path::new(path);
    let file = File::open(path)?;
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

fn create_header(secret_name: &str) -> String {
    format!("!!CMD!!ID!!{secret_name}")
}
