use clap::{Parser, Subcommand};
use eyre::{eyre, Result, WrapErr};
use serde_json::{json, Value};
use std::io::Write as _;
use std::process::{Command, Stdio};
use xshell::{cmd, Shell};

use crate::{cmd::update, dotfiles_dir, util::has_tool};

const HOST: &str = "praveen@ai5090";
/// Checks out the Homebased skill at the installed Homebased release
const HOMEBASED_SKILL_SYNC: &str = "bin/homebased-skill-sync";
const UPDATE_COMMAND: &str = "/home/praveen/.local/bin/fleet-update";
/// Pulls dotfiles on ai5090 and its code container
const DFU_COMMAND: &str = "/home/praveen/.local/bin/dfu";

/// Terminal type for ssh and tmux when the caller has no usable one
const FALLBACK_TERM: &str = "xterm-256color";

/// Fleet machines that are often asleep; `fleet update` queues their update
/// in Homebased, which sends it once the machine answers
const SLEEPING_MACHINES: &[&str] = &["mba"];

/// Homebased coalesce key, so the updates a sleeping machine missed run once
const UPDATE_COALESCE_KEY: &str = "fleet-update";

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
    /// installed cmd, cln, Homebased, and codemode-mcp on ai5090 and code;
    /// update cmd and any installed cln, Homebased, and codemode-mcp on the
    /// calling machine too if it was not updated remotely. Each Homebased
    /// update also checks out the Homebased skill at the matching release tag,
    /// and codemode-mcp installs its latest release binary
    ///
    /// The mba is often asleep, so its update is queued in Homebased instead:
    /// it runs `fleet update --local` there once someone has used the mba for
    /// two minutes, and queued updates it missed collapse to one
    ///
    /// Without --all, this uses no sudo and changes no system packages;
    /// running Homebased daemons restart after their update
    Update {
        /// Also upgrade APT packages and update T3 Code; this can use sudo and
        /// restart T3 Code
        #[arg(long)]
        all: bool,

        /// Update only this machine: pull ~/code/dotfiles, then update cmd and
        /// any installed cln, Homebased and its skill, and codemode-mcp; a
        /// failed step does not stop the rest
        #[arg(long, conflicts_with = "all")]
        local: bool,
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
        FleetCmd::Update { local: true, .. } => run_local_update(sh),
        FleetCmd::Update { all, .. } => run_update(sh, UpdateMode::from_all_flag(all)),
    }
}

// called by path because a non-interactive ssh shell has no ~/.local/bin on PATH
fn run_dfu(sh: &Shell) -> Result<()> {
    if on_ai5090(sh) {
        sh.cmd(DFU_COMMAND).run()?;
    } else {
        cmd!(sh, "ssh -o BatchMode=yes {HOST} {DFU_COMMAND}").run()?;
    }

    Ok(())
}

fn run_update(sh: &Shell, mode: UpdateMode) -> Result<()> {
    // queued first, so a remote failure below cannot skip a sleeping machine
    queue_sleeping_updates(sh);

    let local = on_ai5090(sh);
    require_update_command(local)?;

    clear_stale_session(local, mode.tmux_session())?;
    let (program, args) = update_argv(local, mode);
    run_attached(program, &args)?;

    if updated_by_remote_script(sh) {
        return Ok(());
    }

    let mut failures = Vec::new();
    update_tools(sh, &mut failures);
    finish(failures)
}

/// Update only this machine, as a queued update does on a machine that was asleep
fn run_local_update(sh: &Shell) -> Result<()> {
    let mut failures = Vec::new();
    let dotfiles = dotfiles_dir()?;
    let dotfiles = dotfiles
        .to_str()
        .ok_or_else(|| eyre!("non-UTF-8 path {}", dotfiles.display()))?;
    step(&mut failures, "dotfiles", || {
        run_attached("git", &["-C", dotfiles, "up"])
    });
    update_tools(sh, &mut failures);
    finish(failures)
}

/// Update cmd and any installed cln, Homebased, and codemode-mcp on this machine
///
/// Each step runs even when an earlier one failed, because they are
/// independent and an unattended run has nobody to rerun the rest
fn update_tools(sh: &Shell, failures: &mut Vec<String>) {
    if has_tool(sh, "cln") {
        step(failures, "cln", || run_attached("cln", &["update"]));
    }

    if has_tool(sh, "homebased") {
        step(failures, "homebased", || {
            run_attached("homebased", &["update"])?;

            let sync = dotfiles_dir()?.join(HOMEBASED_SKILL_SYNC);
            let sync = sync
                .to_str()
                .ok_or_else(|| eyre!("non-UTF-8 path {}", sync.display()))?;
            run_attached(sync, &[])
        });
    }

    // on a Mac this installs the signed release and skips the restart when
    // already current; ai5090 and code build theirs in fleet-update instead
    if has_tool(sh, "codemode-mcp") {
        step(failures, "codemode-mcp", || {
            run_attached("codemode-mcp", &["update"])
        });
    }

    // last, because a just-tagged release has no assets until its build
    // finishes; this also refreshes the btx and prc links to the new binary
    step(failures, "cmd", || {
        update::run_with_flags(
            sh,
            update::Update {
                force: false,
                wait: false,
            },
        )
    });
}

/// Run one update step, recording its name when it fails
fn step(failures: &mut Vec<String>, name: &str, run: impl FnOnce() -> Result<()>) {
    if let Err(error) = run() {
        eprintln!("{name} update failed: {error:#}");
        failures.push(name.to_owned());
    }
}

fn finish(failures: Vec<String>) -> Result<()> {
    if failures.is_empty() {
        return Ok(());
    }

    Err(eyre!("update failed for {}", failures.join(", ")))
}

/// Queue `fleet update --local` on each sleeping machine other than this one
///
/// A failure here only warns: the rest of the fleet still updates
fn queue_sleeping_updates(sh: &Shell) {
    if !has_tool(sh, "homebased") {
        eprintln!("homebased is not installed, so no update is queued for sleeping machines");
        return;
    }

    let this_machine = local_fleet_name();
    for machine in SLEEPING_MACHINES {
        if this_machine.as_deref() == Some(*machine) {
            continue;
        }

        match queue_update(machine) {
            Ok(queued) if queued.status == "held" => println!(
                "{machine} is asleep; its update waits as Homebased task {} and runs when it wakes",
                queued.id
            ),
            Ok(queued) => println!("{machine}: update started as Homebased task {}", queued.id),
            Err(error) => eprintln!("warning: could not queue the update for {machine}: {error:#}"),
        }
    }
}

/// Homebased answer to a queued update
struct QueuedUpdate {
    id: String,
    status: String,
}

/// Submit the update spec for `machine` through the local Homebased daemon
///
/// `--allow-other-thread` lets a Homebased worker, such as an agent running
/// this update, queue it too: the update belongs to no thread, not the worker's
fn queue_update(machine: &str) -> Result<QueuedUpdate> {
    let spec = update_spec(machine);
    let mut child = Command::new("homebased")
        .args([
            "--json",
            "task",
            "submit",
            "--spec",
            "-",
            "--allow-other-thread",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .wrap_err("failed to run `homebased task submit`")?;
    child
        .stdin
        .take()
        .ok_or_else(|| eyre!("homebased task submit has no stdin"))?
        .write_all(spec.to_string().as_bytes())
        .wrap_err("failed to send the update spec to homebased")?;
    let output = child
        .wait_with_output()
        .wrap_err("failed to wait for `homebased task submit`")?;

    if !output.status.success() {
        return Err(eyre!("{}", String::from_utf8_lossy(&output.stderr).trim()));
    }

    let body: Value = serde_json::from_slice(&output.stdout)
        .wrap_err("homebased task submit printed invalid JSON")?;
    let field = |name: &str| {
        body.get(name)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| eyre!("homebased task submit printed no {name}"))
    };

    Ok(QueuedUpdate {
        id: field("id")?,
        status: field("status")?,
    })
}

/// Homebased spec that runs `cmd fleet update --local` on `machine`
///
/// It has no thread, so nothing wakes an agent when it ends. cmd updates
/// itself first, so a machine asleep through a cmd release still runs the
/// newest update steps. The executor's daemon PATH has ~/.local/bin, where
/// cmd lives
fn update_spec(machine: &str) -> Value {
    json!({
        "api_version": 1,
        // no session waits on it, so its ending wakes no agent hours later
        "thread": null,
        "name": "fleet update",
        "cwd": "~/",
        "machine": machine,
        "wait_for_machine": true,
        "coalesce": UPDATE_COALESCE_KEY,
        "timeout": "1h",
        "workload": {
            "type": "task",
            "command": ["/bin/sh", "-c", "cmd update; exec cmd fleet update --local"]
        }
    })
}

/// This machine's Homebased Fleet name, when Homebased answers
fn local_fleet_name() -> Option<String> {
    let output = Command::new("homebased")
        .args(["--json", "fleet", "machines"])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let body: Value = serde_json::from_slice(&output.stdout).ok()?;
    body.get("local")?.get("name")?.as_str().map(str::to_owned)
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

/// Remove an update session whose panes are not running `fleet-update`
///
/// `new-session -A` attaches to any session with the name and ignores the
/// command, which is how an interrupted run is resumed. tmux-continuum
/// restores saved sessions after a reboot as plain shells, so a session saved
/// mid-update would otherwise swallow every later update without running it
fn clear_stale_session(on_ai5090: bool, session: &str) -> Result<()> {
    let target = format!("={session}");
    let panes = tmux_command(
        on_ai5090,
        &[
            "list-panes",
            "-s",
            "-t",
            &target,
            "-F",
            "#{pane_start_command}",
        ],
    )
    .stderr(Stdio::null())
    .output()
    .wrap_err("failed to run `tmux list-panes`")?;

    // no such session, so `new-session` creates a fresh one
    if !panes.status.success() {
        return Ok(());
    }

    if runs_update(&String::from_utf8_lossy(&panes.stdout)) {
        return Ok(());
    }

    eprintln!("tmux session {session} is not running {UPDATE_COMMAND}; replacing it");
    let status = tmux_command(on_ai5090, &["kill-session", "-t", &target])
        .status()
        .wrap_err("failed to run `tmux kill-session`")?;

    if status.success() {
        return Ok(());
    }

    Err(eyre!("could not remove stale tmux session {session}"))
}

/// Whether any pane in `start_commands`, one per line, was started as `fleet-update`
fn runs_update(start_commands: &str) -> bool {
    start_commands
        .lines()
        .any(|command| command.trim_start_matches('"').starts_with(UPDATE_COMMAND))
}

/// A non-interactive tmux invocation on ai5090, through ssh when called elsewhere
fn tmux_command(on_ai5090: bool, args: &[&str]) -> Command {
    if on_ai5090 {
        let mut command = Command::new("tmux");
        command.args(args);
        return command;
    }

    // ssh joins its arguments into one line for the remote zsh, which would
    // expand `=name` into a command path and `#` into a comment
    let mut command = Command::new("ssh");
    command
        .args(["-o", "BatchMode=yes", HOST, "tmux"])
        .args(args.iter().map(|arg| shell_quote(arg)));
    command
}

fn shell_quote(arg: &str) -> String {
    format!("'{}'", arg.replace('\'', "'\\''"))
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
        fallback_term, is_ai5090_host, is_remote_updated_host, runs_update, shell_quote,
        update_argv, UpdateMode, FALLBACK_TERM,
    };

    #[test]
    fn reuses_only_a_session_that_runs_fleet_update() {
        assert!(runs_update("/home/praveen/.local/bin/fleet-update\n"));
        assert!(runs_update(
            "\"/home/praveen/.local/bin/fleet-update\" --all\n"
        ));
        // tmux-continuum restores a saved session as a shell with no start command
        assert!(!runs_update("\n"));
        assert!(!runs_update("zsh\n"));
    }

    #[test]
    fn quotes_tmux_arguments_for_the_remote_shell() {
        assert_eq!(shell_quote("=fleet-update"), "'=fleet-update'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }

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
