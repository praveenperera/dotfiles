use clap::{Parser, Subcommand};
use eyre::Result;
use xshell::{cmd, Shell};

const HOST: &str = "praveen@ai5090";
const UPDATE_COMMAND: &str = "fleet-update";

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

    /// Update Codex, Claude Code, and Grok Build on ai5090, code, and training
    ///
    /// Without --all, this uses no sudo, changes no system packages, and
    /// restarts no services
    Update {
        /// Also upgrade APT packages and update T3 Code, which uses sudo and
        /// restarts services
        #[arg(long)]
        all: bool,
    },
}

/// Selects how much of the fleet `fleet-update` is allowed to change
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateMode {
    /// User-owned agent CLIs only: no sudo, APT, T3 Code, or service restart
    Safe,
    /// Safe mode plus APT packages, T3 Code, sudo, and service restarts
    Full,
}

impl UpdateMode {
    fn from_all_flag(all: bool) -> Self {
        if all {
            Self::Full
        } else {
            Self::Safe
        }
    }

    /// Arguments for the installed `fleet-update` command on ai5090
    fn args(self) -> &'static [&'static str] {
        match self {
            Self::Safe => &[],
            Self::Full => &["--all"],
        }
    }

    // separate sessions keep `--all` from silently reattaching to a safe run;
    // fleet-update holds a lock so the two modes cannot run at the same time
    fn tmux_session(self) -> &'static str {
        match self {
            Self::Safe => "fleet-update",
            Self::Full => "fleet-update-all",
        }
    }
}

pub fn run_with_flags(sh: &Shell, flags: Fleet) -> Result<()> {
    match flags.subcommand {
        FleetCmd::DotfilesUp => run_dfu(sh),
        FleetCmd::Update { all } => run_update(sh, UpdateMode::from_all_flag(all)),
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

fn run_update(sh: &Shell, mode: UpdateMode) -> Result<()> {
    let session = mode.tmux_session();
    let args = mode.args();

    if on_ai5090(sh) {
        cmd!(
            sh,
            "tmux new-session -A -s {session} {UPDATE_COMMAND} {args...}"
        )
        .run()?;
    } else {
        cmd!(
            sh,
            "ssh -t {HOST} tmux new-session -A -s {session} {UPDATE_COMMAND} {args...}"
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
    use super::{is_ai5090_host, UpdateMode};

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

    #[test]
    fn safe_mode_passes_no_flags() {
        let mode = UpdateMode::from_all_flag(false);

        assert_eq!(mode, UpdateMode::Safe);
        assert!(mode.args().is_empty());
    }

    #[test]
    fn full_mode_passes_only_all_flag() {
        let mode = UpdateMode::from_all_flag(true);

        assert_eq!(mode, UpdateMode::Full);
        assert_eq!(mode.args(), ["--all"]);
    }
}
