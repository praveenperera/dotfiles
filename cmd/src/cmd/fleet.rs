use clap::{Parser, Subcommand};
use eyre::Result;
use xshell::{cmd, Shell};

const HOST: &str = "praveen@ai5090";
const MAINTAIN_SESSION: &str = "maintenance";

#[derive(Debug, Clone, Parser)]
pub struct Fleet {
    #[command(subcommand)]
    pub subcommand: FleetCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum FleetCmd {
    /// Pull ~/code/dotfiles with git up on ai5090, code, and training
    #[command(name = "dotfiles-up", visible_alias = "dfu")]
    DotfilesUp,

    /// Update packages and agent CLIs on ai5090, code, and training
    Maintain,
}

pub fn run_with_flags(sh: &Shell, flags: Fleet) -> Result<()> {
    match flags.subcommand {
        FleetCmd::DotfilesUp => run_dfu(sh),
        FleetCmd::Maintain => run_maintain(sh),
    }
}

fn run_dfu(sh: &Shell) -> Result<()> {
    if on_ai5090(sh) {
        cmd!(sh, "dfu").run()?;
    } else {
        cmd!(sh, "ssh {HOST} dfu").run()?;
    }
    Ok(())
}

fn run_maintain(sh: &Shell) -> Result<()> {
    if on_ai5090(sh) {
        cmd!(sh, "tmux new-session -A -s {MAINTAIN_SESSION} maintain").run()?;
    } else {
        cmd!(
            sh,
            "ssh -t {HOST} tmux new-session -A -s {MAINTAIN_SESSION} maintain"
        )
        .run()?;
    }
    Ok(())
}

fn on_ai5090(sh: &Shell) -> bool {
    cmd!(sh, "hostname -s")
        .read()
        .is_ok_and(|name| is_ai5090_host(&name))
}

fn is_ai5090_host(hostname: &str) -> bool {
    hostname.trim() == "ai5090"
}

#[cfg(test)]
mod tests {
    use super::is_ai5090_host;

    #[test]
    fn treats_ai5090_as_local() {
        assert!(is_ai5090_host("ai5090"));
        assert!(is_ai5090_host("ai5090\n"));
    }

    #[test]
    fn treats_other_hosts_as_remote() {
        assert!(!is_ai5090_host("code"));
        assert!(!is_ai5090_host("Praveens-Mac-mini"));
    }
}
