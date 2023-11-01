use eyre::Result;
use xshell::{cmd, Shell};

static SECRETS: [&str; 2] = ["ln.yaml", "sq.yaml"];

pub fn run(sh: &Shell, _args: &[&str]) -> Result<()> {
    let secret_dir = crate::dotfiles_dir().join("cmd/secrets");
    for secret in SECRETS {
        let secret_text = cmd!(sh, "op read op://Personal/cmd_secrets/{secret}").read()?;
        let secret_path = secret_dir.join(secret);

        std::fs::write(secret_path, secret_text.trim())?;
    }

    Ok(())
}
