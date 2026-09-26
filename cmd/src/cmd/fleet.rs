use clap::{Parser, Subcommand};
use eyre::{eyre, Result, WrapErr};
use std::process::{Command, Stdio};
use xshell::{cmd, Shell};

use crate::command_exists;

const HOST: &str = "praveen@ai5090";
const UPDATE_COMMAND: &str = "/home/praveen/.local/bin/fleet-update";

/// Terminal type for ssh and tmux when the caller has no usable one
const FALLBACK_TERM: &str = "xterm-256color";

#[derive(Debug, Clone, Parser)]
pub struct Fleet {
    #[command(subcommand)]
    pub subcommand: FleetCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum FleetCmd {
    /// Pull ~/code/dotfiles with git up on ai5090 and code
    #[command(name = "dotfiles-up", visible_alias = "dfu")]
    DotfilesUp,

    /// Pull ~/code/dotfiles, then update Codex, Claude Code, Grok Build, and
    /// installed Homebased on ai5090 and code; update Homebased on the calling
    /// machine too if it is installed and was not updated remotely
    ///
    /// Without --all, this uses no sudo and changes no system packages;
    /// running Homebased daemons restart after their update
    Update {
        /// Also upgrade APT packages and update T3 Code; this can use sudo and
        /// restart T3 Code
        #[arg(long)]
        all: bool,
    },
}

/// Selects how much of the fleet `fleet-update` is allowed to change
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateMode {
    /// User-owned tools only: no sudo, APT, or T3 Code
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
    let local = on_ai5090(sh);
    require_update_command(local)?;

    let (program, args) = update_argv(local, mode);
    run_attached(program, &args)?;

    if !updated_by_remote_script(sh) && command_exists(sh, "homebased") {
        run_attached("homebased", &["update"])?;
    }

    Ok(())
}

fn require_update_command(on_ai5090: bool) -> Result<()> {
    let installed = if on_ai5090 {
        Command::new("test")
            .args(["-x", UPDATE_COMMAND])
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    } else {
        Command::new("ssh")
            .args(["-o", "BatchMode=yes", HOST, "test", "-x", UPDATE_COMMAND])
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    };

    if installed {
        return Ok(());
    }

    Err(eyre!(
        "fleet-update is not installed at {UPDATE_COMMAND}; setup-host.sh installs it"
    ))
}

fn update_argv(on_ai5090: bool, mode: UpdateMode) -> (&'static str, Vec<&'static str>) {
    let mut args = Vec::new();

    if on_ai5090 {
        args.extend([
            "new-session",
            "-A",
            "-s",
            mode.tmux_session(),
            UPDATE_COMMAND,
        ]);

        args.extend(mode.args().iter().copied());

        ("tmux", args)
    } else {
        // force a remote pty even when stdin is not a local tty
        args.extend([
            "-tt",
            HOST,
            "tmux",
            "new-session",
            "-A",
            "-s",
            mode.tmux_session(),
            UPDATE_COMMAND,
        ]);

        args.extend(mode.args().iter().copied());

        ("ssh", args)
    }
}

fn run_attached(program: &str, args: &[&str]) -> Result<()> {
    // inherit stdin so ssh and tmux can use the local tty; xshell run() uses
    // /dev/null and then ssh -t refuses to allocate a pty
    eprintln!("$ {program} {}", args.join(" "));

    let mut command = Command::new(program);
    // ssh passes TERM to the remote pty, and tmux refuses to start when it
    // is unset or dumb, as it is under agents and other non-interactive shells
    if let Some(term) = fallback_term(std::env::var("TERM").ok().as_deref()) {
        command.env("TERM", term);
    }

    let status = command
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .wrap_err_with(|| format!("failed to run `{program}`"))?;

    if status.success() {
        return Ok(());
    }

    let displayed = std::iter::once(program)
        .chain(args.iter().copied())
        .collect::<Vec<_>>()
        .join(" ");
    let code = status.code().unwrap_or(1);

    Err(eyre!(
        "command exited with non-zero code `{displayed}`: {code}"
    ))
}

/// Terminal type to set when `current` cannot drive tmux
fn fallback_term(current: Option<&str>) -> Option<&'static str> {
    match current.map(str::trim) {
        None | Some("" | "dumb") => Some(FALLBACK_TERM),
        Some(_) => None,
    }
}

fn on_ai5090(sh: &Shell) -> bool {
    cmd!(sh, "hostname -s")
        .read()
        .is_ok_and(|name| is_ai5090_host(&name))
}

fn updated_by_remote_script(sh: &Shell) -> bool {
    cmd!(sh, "hostname -s")
        .read()
        .is_ok_and(|name| is_remote_updated_host(&name))
}

fn is_ai5090_host(hostname: &str) -> bool {
    hostname.trim() == "ai5090"
}

fn is_remote_updated_host(hostname: &str) -> bool {
    matches!(hostname.trim(), "ai5090" | "code")
}

#[cfg(test)]
mod tests {
    use super::{
        fallback_term, is_ai5090_host, is_remote_updated_host, update_argv, UpdateMode,
        FALLBACK_TERM,
    };

    #[test]
    fn sets_a_terminal_type_only_when_tmux_cannot_use_the_current_one() {
        assert_eq!(fallback_term(None), Some(FALLBACK_TERM));
        assert_eq!(fallback_term(Some("")), Some(FALLBACK_TERM));
        assert_eq!(fallback_term(Some("dumb")), Some(FALLBACK_TERM));
        assert_eq!(fallback_term(Some("xterm-ghostty")), None);
    }

    #[test]
    fn treats_ai5090_as_local() {
        assert!(is_ai5090_host("ai5090"));
        assert!(is_ai5090_host("ai5090\n"));
    }

    #[test]
    fn treats_other_hosts_as_remote() {
        assert!(!is_ai5090_host("code"));
        assert!(!is_ai5090_host("workstation"));
    }

    #[test]
    fn local_homebased_update_skips_remote_machines() {
        assert!(is_remote_updated_host("ai5090"));
        assert!(is_remote_updated_host("code\n"));
        assert!(!is_remote_updated_host("workstation"));
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

    #[test]
    fn local_update_attaches_tmux() {
        let (program, args) = update_argv(true, UpdateMode::Safe);

        assert_eq!(program, "tmux");
        assert_eq!(
            args,
            [
                "new-session",
                "-A",
                "-s",
                "fleet-update",
                "/home/praveen/.local/bin/fleet-update"
            ]
        );
    }

    #[test]
    fn remote_update_forces_a_pty() {
        let (program, args) = update_argv(false, UpdateMode::Safe);

        assert_eq!(program, "ssh");
        assert_eq!(
            args,
            [
                "-tt",
                "praveen@ai5090",
                "tmux",
                "new-session",
                "-A",
                "-s",
                "fleet-update",
                "/home/praveen/.local/bin/fleet-update"
            ]
        );
    }

    #[test]
    fn remote_full_update_uses_the_all_session() {
        let (program, args) = update_argv(false, UpdateMode::Full);

        assert_eq!(program, "ssh");
        assert_eq!(
            args,
            [
                "-tt",
                "praveen@ai5090",
                "tmux",
                "new-session",
                "-A",
                "-s",
                "fleet-update-all",
                "/home/praveen/.local/bin/fleet-update",
                "--all"
            ]
        );
    }
}
