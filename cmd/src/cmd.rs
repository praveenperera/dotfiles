use eyre::Result;
use xshell::{cmd, Shell};

pub fn run(sh: &Shell) -> Result<()> {
    cmd!(sh, "echo hi").run()?;
    Ok(())
}
