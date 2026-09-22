pub(super) use super::model::ResolvedPasteTarget;
use super::model::{
    validate_name, ClientRef, ConnectionId, MachineId, PaneId, PaneRef, Route, ServerIdentity,
    SessionRef, SshDestination, WindowRef,
};
use clap::{Args, Subcommand, ValueEnum};
use eyre::{eyre, Result, WrapErr};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, IsTerminal, Write};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use xshell::Shell;

const WORKSPACE_OPTION: &str = "@fleet_workspace";
const WORKSPACE_NAME_OPTION: &str = "@fleet_workspace_name";
const ROLE_OPTION: &str = "@fleet_role";
const DISPLAY_OPTION: &str = "@fleet_display";
const SERVICE_OPTION: &str = "@fleet_service";
const VIEW_OPTION: &str = "@fleet_view";
const RESERVED_PREFIX: &str = "__fleet_";

#[derive(Debug, Clone, Args)]
pub struct RemoteArgs {
    /// Fleet execution machine
    machine: MachineId,
    /// Exact canonical session name
    session: Option<String>,
    /// Select automatic, normal, or minimal presentation
    #[arg(long, value_enum, default_value_t = ViewMode::Auto)]
    mode: ViewMode,
    /// Attach only and fail if the exact session does not exist
    #[arg(short = 'a', long, conflicts_with = "list", requires = "session")]
    attach: bool,
    /// List canonical sessions
    #[arg(short = 'l', long, conflicts_with = "session")]
    list: bool,
    /// Exact remote window ID for an internal managed-view attach
    #[arg(long, hide = true, requires = "session")]
    window_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
enum ViewMode {
    #[default]
    Auto,
    Normal,
    Minimal,
}

#[derive(Debug, Clone, Args)]
pub struct WorkspaceArgs {
    #[command(subcommand)]
    command: WorkspaceCommand,
}

#[derive(Debug, Clone, Subcommand)]
enum WorkspaceCommand {
    /// Attach a display client to the shared workspace
    Attach {
        /// Stable display label
        #[arg(long, default_value = "main")]
        display: String,
        /// Adopt this exact existing session as the canonical workspace
        #[arg(long)]
        adopt: Option<String>,
        /// Route identity supplied by the managed first-hop launcher
        #[arg(long, hide = true)]
        connection_id: Option<String>,
    },
    /// Connect from another Mac through the registered Mini route
    Connect {
        /// Stable display label
        #[arg(long, default_value = "main")]
        display: String,
    },
    /// Reconcile saved subscriptions and managed views
    Reconnect,
    /// Restore managed displays and the sync worker after canonical restore
    Restore,
    /// Stop the worker and remove only managed workspace sessions
    Shutdown,
}

#[derive(Debug, Clone, Args)]
pub struct SyncArgs {
    #[command(subcommand)]
    command: SyncCommand,
}

#[derive(Debug, Clone, Subcommand)]
enum SyncCommand {
    /// Run the supervised synchronization loop
    Worker {
        /// Canonical workspace session ID
        #[arg(long)]
        workspace: String,
    },
    /// Reconcile the current remote snapshots once
    Reconcile {
        /// Canonical workspace session ID
        #[arg(long)]
        workspace: String,
    },
}

#[derive(Debug, Clone, Args)]
pub struct CloseArgs {
    /// Exact outer tmux pane ID
    #[arg(long)]
    pub(super) target_pane: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct UnsubscribeArgs {
    /// Fleet execution machine
    machine: MachineId,
    /// Stable canonical session ID
    session_id: String,
    /// Canonical workspace session ID
    #[arg(long)]
    workspace: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct WorkspaceState {
    #[serde(default)]
    subscriptions: Vec<Subscription>,
    #[serde(default)]
    views: Vec<ManagedView>,
    #[serde(default)]
    connections: Vec<ConnectionRecord>,
    #[serde(default)]
    stopped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ConnectionRecord {
    pub(super) id: ConnectionId,
    pub(super) owner_pid: u32,
    pub(super) source_tty: PathBuf,
    pub(super) source_client: Option<ClientRef>,
    pub(super) source_pane: Option<PaneRef>,
    pub(super) source_session: Option<SessionRef>,
    pub(super) destination_session: Option<SessionRef>,
    pub(super) visible_window: Option<WindowRef>,
    pub(super) route: Route,
    pub(super) publication_generation: u64,
    pub(super) status: ConnectionStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum ConnectionStatus {
    Live,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Subscription {
    machine: MachineId,
    session_id: String,
    session_name: String,
    server_generation: u32,
    workspace_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ManagedView {
    machine: MachineId,
    server_generation: u32,
    window_id: String,
    outer_window_id: String,
    hidden: bool,
    #[serde(default)]
    subscription_ids: Vec<String>,
    #[serde(default)]
    generated_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RemoteWindow {
    id: String,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReconcileDecision {
    Keep(String),
    Create(String),
    Remove(String),
    Hidden(String),
}

fn reconcile_decisions(
    desired_keys: impl IntoIterator<Item = String>,
    views: &[ManagedView],
) -> Vec<ReconcileDecision> {
    let mut desired = desired_keys.into_iter().collect::<Vec<_>>();
    desired.sort();
    desired.dedup();
    let mut decisions = desired
        .iter()
        .map(|key| {
            match views.iter().find(|view| {
                view_key(view.machine, view.server_generation, &view.window_id) == *key
            }) {
                Some(view) if view.hidden => ReconcileDecision::Hidden(key.clone()),
                Some(_) => ReconcileDecision::Keep(key.clone()),
                None => ReconcileDecision::Create(key.clone()),
            }
        })
        .collect::<Vec<_>>();
    decisions.extend(views.iter().filter_map(|view| {
        let key = view_key(view.machine, view.server_generation, &view.window_id);
        (!desired.contains(&key)).then_some(ReconcileDecision::Remove(key))
    }));
    decisions.sort_by(|left, right| format!("{left:?}").cmp(&format!("{right:?}")));
    decisions
}

pub(super) fn resolve_route_for_tty(tty: &Path) -> Result<ResolvedPasteTarget> {
    let source = match local_source_for_tty(tty) {
        Ok(source) => source,
        Err(local_error) => {
            return resolve_registered_route(tty).wrap_err_with(|| {
                format!(
                    "no local tmux client or managed connection matches the TTY: {local_error:#}"
                )
            });
        }
    };
    let view_key = local_tmux_output([
        "show-option",
        "-wqv",
        "-t",
        &source.pane.to_string(),
        "@fleet_view_key",
    ])?;
    if view_key.trim().is_empty() {
        let (display_session, display_window) = local_display_labels(&source)?;
        let route = Route::Local {
            pane: source.clone(),
        };
        return Ok(ResolvedPasteTarget {
            source_tty: tty.to_path_buf(),
            source_pane: Some(source.clone()),
            route,
            machine: MachineId::Mini,
            server: source.server.clone(),
            pane: source.pane.clone(),
            cwd: local_pane_cwd(&source)?,
            ssh: None,
            connections: Vec::new(),
            display_session,
            display_window,
        });
    }
    resolve_remote_view(tty, source, view_key.trim())
}

fn resolve_registered_route(tty: &Path) -> Result<ResolvedPasteTarget> {
    let state = load_state()?;
    let records = state
        .connections
        .iter()
        .filter(|record| record.status == ConnectionStatus::Live && record.source_tty == tty)
        .collect::<Vec<_>>();
    let [record] = records.as_slice() else {
        return Err(eyre!("managed connection registry is missing or ambiguous"));
    };
    if !process_is_live(record.owner_pid) {
        return Err(eyre!("managed connection registry entry is stale"));
    }
    let Route::Ssh { destination, .. } = &record.route else {
        return Err(eyre!("managed first-hop route is invalid"));
    };
    let session = record
        .destination_session
        .as_ref()
        .ok_or_else(|| eyre!("managed first-hop route has no destination session"))?;
    let output = remote_tmux_output(
        &destination.alias,
        [
            "display-message",
            "-p",
            "-t",
            &session.session.to_string(),
            "#{pane_id}\t#{pane_current_path}\t#{socket_path}\t#{pid}\t#{session_name}\t#{window_name}\t#{@fleet_view_key}",
        ],
    )?;
    let mut fields = output.trim().split('\t');
    let pane = fields
        .next()
        .ok_or_else(|| eyre!("managed destination has no pane"))?
        .parse::<PaneId>()?;
    let cwd = PathBuf::from(
        fields
            .next()
            .ok_or_else(|| eyre!("managed destination has no cwd"))?,
    );
    let socket = PathBuf::from(
        fields
            .next()
            .ok_or_else(|| eyre!("managed destination has no socket"))?,
    );
    let generation = fields
        .next()
        .ok_or_else(|| eyre!("managed destination has no generation"))?
        .parse::<u32>()?;
    let display_session = fields
        .next()
        .ok_or_else(|| eyre!("managed destination has no session label"))?
        .to_owned();
    let display_window = fields
        .next()
        .ok_or_else(|| eyre!("managed destination has no window label"))?
        .to_owned();
    let view_key = fields.next().unwrap_or_default();
    if fields.next().is_some() || generation != session.server.generation {
        return Err(eyre!(
            "managed first-hop route belongs to a stale tmux server"
        ));
    }
    if !view_key.is_empty() {
        return resolve_registered_remote_hop(tty, record, view_key);
    }
    let server = ServerIdentity::new(destination.machine, socket, generation)?;
    let final_pane = PaneRef {
        server: server.clone(),
        pane: pane.clone(),
    };
    Ok(ResolvedPasteTarget {
        source_tty: tty.to_path_buf(),
        source_pane: None,
        route: Route::ManagedHop {
            connection: record.id.clone(),
            next: Box::new(Route::Ssh {
                destination: destination.clone(),
                pane: final_pane,
            }),
        },
        machine: destination.machine,
        server,
        pane,
        cwd,
        ssh: Some(destination.clone()),
        connections: vec![record.id.clone()],
        display_session,
        display_window,
    })
}

fn resolve_registered_remote_hop(
    tty: &Path,
    record: &ConnectionRecord,
    key: &str,
) -> Result<ResolvedPasteTarget> {
    let (machine, expected_generation, window) = parse_view_key(key)?;
    let ssh = SshDestination::approved(machine)?;
    let output = remote_tmux_output(
        &ssh.alias,
        [
            "display-message",
            "-p",
            "-t",
            window,
            "#{pane_id}\t#{pane_current_path}\t#{socket_path}\t#{pid}\t#{session_name}\t#{window_name}",
        ],
    )?;
    let mut fields = output.trim().split('\t');
    let pane = fields
        .next()
        .ok_or_else(|| eyre!("remote route has no pane"))?
        .parse::<PaneId>()?;
    let cwd = PathBuf::from(
        fields
            .next()
            .ok_or_else(|| eyre!("remote route has no cwd"))?,
    );
    let socket = PathBuf::from(
        fields
            .next()
            .ok_or_else(|| eyre!("remote route has no socket"))?,
    );
    let generation = fields
        .next()
        .ok_or_else(|| eyre!("remote route has no generation"))?
        .parse::<u32>()?;
    let display_session = fields
        .next()
        .ok_or_else(|| eyre!("remote route has no session label"))?
        .to_owned();
    let display_window = fields
        .next()
        .ok_or_else(|| eyre!("remote route has no window label"))?
        .to_owned();
    if generation != expected_generation || fields.next().is_some() {
        return Err(eyre!("remote route belongs to a stale tmux server"));
    }
    let server = ServerIdentity::new(machine, socket, generation)?;
    let final_pane = PaneRef {
        server: server.clone(),
        pane: pane.clone(),
    };
    Ok(ResolvedPasteTarget {
        source_tty: tty.to_path_buf(),
        source_pane: None,
        route: Route::ManagedHop {
            connection: record.id.clone(),
            next: Box::new(Route::Ssh {
                destination: ssh.clone(),
                pane: final_pane,
            }),
        },
        machine,
        server,
        pane,
        cwd,
        ssh: Some(ssh),
        connections: vec![record.id.clone()],
        display_session,
        display_window,
    })
}

fn process_is_live(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .is_ok_and(|status| status.success())
}

pub(super) fn list_paste_targets(tty: &Path) -> Result<Vec<ResolvedPasteTarget>> {
    let source = local_source_for_tty(tty)?;
    let mut targets = local_paste_targets(tty, &source)?;
    let state = load_state()?;
    for subscription in state.subscriptions {
        targets.extend(remote_paste_targets(tty, &source, &subscription)?);
    }
    Ok(targets)
}

fn local_paste_targets(tty: &Path, source: &PaneRef) -> Result<Vec<ResolvedPasteTarget>> {
    let output = local_tmux_output([
        "list-panes",
        "-a",
        "-F",
        "#{pane_id}\t#{pane_current_path}\t#{session_name}\t#{window_name}\t#{@fleet_role}",
    ])?;
    Ok(output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let pane = fields.next()?.parse::<PaneId>().ok()?;
            let cwd = PathBuf::from(fields.next()?);
            let display_session = fields.next()?.to_owned();
            let display_window = fields.next()?.to_owned();
            let role = fields.next().unwrap_or_default();
            if matches!(role, "view" | "service" | "display") {
                return None;
            }
            let destination = PaneRef {
                server: source.server.clone(),
                pane: pane.clone(),
            };
            Some(ResolvedPasteTarget {
                source_tty: tty.to_path_buf(),
                source_pane: Some(source.clone()),
                route: Route::Local { pane: destination },
                machine: MachineId::Mini,
                server: source.server.clone(),
                pane,
                cwd,
                ssh: None,
                connections: Vec::new(),
                display_session,
                display_window,
            })
        })
        .collect::<Vec<_>>())
}

fn remote_paste_targets(
    tty: &Path,
    source: &PaneRef,
    subscription: &Subscription,
) -> Result<Vec<ResolvedPasteTarget>> {
    let ssh = SshDestination::approved(subscription.machine)?;
    let identity = remote_session_identity(subscription.machine, &subscription.session_name)?;
    if identity
        != (
            subscription.server_generation,
            subscription.session_id.clone(),
        )
    {
        return Ok(Vec::new());
    }
    let output = remote_tmux_output(
        &ssh.alias,
        [
            "list-panes",
            "-s",
            "-t",
            &format!("={}", subscription.session_name),
            "-F",
            "#{pane_id}\t#{pane_current_path}\t#{socket_path}\t#{pid}\t#{session_name}\t#{window_name}",
        ],
    )?;
    output
        .lines()
        .map(|line| {
            let mut fields = line.split('\t');
            let pane = fields
                .next()
                .ok_or_else(|| eyre!("remote pane has no id"))?
                .parse::<PaneId>()?;
            let cwd = PathBuf::from(
                fields
                    .next()
                    .ok_or_else(|| eyre!("remote pane has no cwd"))?,
            );
            let socket = PathBuf::from(
                fields
                    .next()
                    .ok_or_else(|| eyre!("remote pane has no socket"))?,
            );
            let generation = fields
                .next()
                .ok_or_else(|| eyre!("remote pane has no generation"))?
                .parse::<u32>()?;
            let display_session = fields
                .next()
                .ok_or_else(|| eyre!("remote pane has no session label"))?
                .to_owned();
            let display_window = fields
                .next()
                .ok_or_else(|| eyre!("remote pane has no window label"))?
                .to_owned();
            let server = ServerIdentity::new(subscription.machine, socket, generation)?;
            let destination = PaneRef {
                server: server.clone(),
                pane: pane.clone(),
            };
            Ok(ResolvedPasteTarget {
                source_tty: tty.to_path_buf(),
                source_pane: Some(source.clone()),
                route: Route::Ssh {
                    destination: ssh.clone(),
                    pane: destination,
                },
                machine: subscription.machine,
                server,
                pane,
                cwd,
                ssh: Some(ssh.clone()),
                connections: Vec::new(),
                display_session,
                display_window,
            })
        })
        .collect()
}

pub(super) fn revalidate_paste_target(target: &ResolvedPasteTarget, tty: &Path) -> Result<()> {
    if let Some(expected_source) = target.source_pane.as_ref() {
        let source = local_source_for_tty(tty)?;
        if expected_source != &source || tty != target.source_tty {
            return Err(eyre!("source tmux selection changed during image transfer"));
        }
    } else {
        let current = resolve_registered_route(tty)?;
        if current.connections != target.connections
            || current.server != target.server
            || current.pane != target.pane
            || current.cwd != target.cwd
        {
            return Err(eyre!(
                "managed connection route changed during image transfer"
            ));
        }
    }
    target.route.validate()?;
    let output = match &target.ssh {
        Some(ssh) => remote_tmux_output(
            &ssh.alias,
            [
                "display-message",
                "-p",
                "-t",
                &target.pane.to_string(),
                "#{pane_id}\t#{pane_current_path}\t#{socket_path}\t#{pid}",
            ],
        )?,
        None => local_tmux_output([
            "display-message",
            "-p",
            "-t",
            &target.pane.to_string(),
            "#{pane_id}\t#{pane_current_path}\t#{socket_path}\t#{pid}",
        ])?,
    };
    let mut fields = output.trim().split('\t');
    let pane = fields.next().and_then(|value| value.parse::<PaneId>().ok());
    let cwd = fields.next().map(PathBuf::from);
    let socket = fields.next().map(PathBuf::from);
    let generation = fields.next().and_then(|value| value.parse::<u32>().ok());
    if pane.as_ref() != Some(&target.pane)
        || cwd.as_ref() != Some(&target.cwd)
        || socket.as_ref() != Some(&target.server.socket)
        || generation != Some(target.server.generation)
        || fields.next().is_some()
    {
        return Err(eyre!("destination tmux pane changed during image transfer"));
    }
    Ok(())
}

pub(super) fn remote_pane_picker(sh: &Shell) -> Result<()> {
    let key = local_tmux_output(["display-message", "-p", "#{@fleet_view_key}"])?;
    let (machine, _, window) = parse_view_key(key.trim())?;
    let destination = machine
        .ssh_alias()
        .ok_or_else(|| eyre!("selected window is not a managed remote view"))?;
    let output = remote_tmux_output(
        destination,
        [
            "list-panes",
            "-t",
            window,
            "-F",
            "#{pane_id}\t#{pane_title}\t#{pane_current_path}",
        ],
    )?;
    if output.trim().is_empty() {
        return Err(eyre!("managed remote window has no live panes"));
    }
    let selection = super::run_fzf(sh, "Remote pane > ", &output)?;
    let pane = selection.split('\t').next().unwrap_or_default();
    if !valid_tmux_id(pane, '%') {
        return Err(eyre!("remote pane picker returned an invalid pane id"));
    }
    require_success(
        remote_tmux_status(destination, ["select-pane", "-t", pane])?,
        "select remote pane",
    )
}

fn parse_view_key(key: &str) -> Result<(MachineId, u32, &str)> {
    let mut fields = key.split(':');
    let machine = match fields.next() {
        Some("code") => MachineId::Code,
        Some("training") => MachineId::Training,
        _ => return Err(eyre!("selected window is not a managed remote view")),
    };
    let generation = fields
        .next()
        .ok_or_else(|| eyre!("managed view key has no generation"))?
        .parse::<u32>()?;
    let window = fields
        .next()
        .filter(|window| valid_tmux_id(window, '@'))
        .ok_or_else(|| eyre!("managed view key has no valid window id"))?;
    if fields.next().is_some() {
        return Err(eyre!("managed view key has extra fields"));
    }
    Ok((machine, generation, window))
}

fn local_source_for_tty(tty: &Path) -> Result<PaneRef> {
    let output = local_tmux_output([
        "list-clients",
        "-F",
        "#{client_tty}\t#{pane_id}\t#{socket_path}\t#{pid}",
    ])?;
    let records = output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let record_tty = PathBuf::from(fields.next()?);
            let pane = fields.next()?.parse::<PaneId>().ok()?;
            let socket = PathBuf::from(fields.next()?);
            let generation = fields.next()?.parse::<u32>().ok()?;
            (record_tty == tty).then(|| {
                ServerIdentity::new(MachineId::Mini, socket, generation)
                    .map(|server| PaneRef { server, pane })
            })
        })
        .collect::<Result<Vec<_>>>()?;
    match records.as_slice() {
        [record] => Ok(record.clone()),
        [] => Err(eyre!("no live tmux client matches TTY {}", tty.display())),
        _ => Err(eyre!("multiple tmux clients match TTY {}", tty.display())),
    }
}

fn local_pane_cwd(pane: &PaneRef) -> Result<PathBuf> {
    Ok(PathBuf::from(
        local_tmux_output([
            "display-message",
            "-p",
            "-t",
            &pane.pane.to_string(),
            "#{pane_current_path}",
        ])?
        .trim(),
    ))
}

fn local_display_labels(pane: &PaneRef) -> Result<(String, String)> {
    let output = local_tmux_output([
        "display-message",
        "-p",
        "-t",
        &pane.pane.to_string(),
        "#{session_name}\t#{window_name}",
    ])?;
    let (session, window) = output
        .trim()
        .split_once('\t')
        .ok_or_else(|| eyre!("tmux returned invalid pane display labels"))?;
    Ok((session.to_owned(), window.to_owned()))
}

fn resolve_remote_view(tty: &Path, source: PaneRef, key: &str) -> Result<ResolvedPasteTarget> {
    let mut fields = key.split(':');
    let machine = match fields.next() {
        Some("code") => MachineId::Code,
        Some("training") => MachineId::Training,
        _ => return Err(eyre!("managed view has an invalid machine")),
    };
    let expected_generation = fields
        .next()
        .ok_or_else(|| eyre!("managed view has no server generation"))?
        .parse::<u32>()?;
    let window = fields
        .next()
        .filter(|window| valid_tmux_id(window, '@'))
        .ok_or_else(|| eyre!("managed view has an invalid window id"))?;
    if fields.next().is_some() {
        return Err(eyre!("managed view key has extra fields"));
    }
    let ssh = SshDestination::approved(machine)?;
    let output = remote_tmux_output(
        &ssh.alias,
        [
            "display-message",
            "-p",
            "-t",
            window,
            "#{pane_id}\t#{pane_current_path}\t#{socket_path}\t#{pid}\t#{session_name}\t#{window_name}",
        ],
    )?;
    let mut fields = output.trim().split('\t');
    let pane = fields
        .next()
        .ok_or_else(|| eyre!("remote view has no selected pane"))?
        .parse::<PaneId>()?;
    let cwd = PathBuf::from(
        fields
            .next()
            .ok_or_else(|| eyre!("remote pane has no cwd"))?,
    );
    let socket = PathBuf::from(
        fields
            .next()
            .ok_or_else(|| eyre!("remote pane has no socket"))?,
    );
    let generation = fields
        .next()
        .ok_or_else(|| eyre!("remote pane has no server generation"))?
        .parse::<u32>()?;
    let display_session = fields
        .next()
        .ok_or_else(|| eyre!("remote pane has no session label"))?
        .to_owned();
    let display_window = fields
        .next()
        .ok_or_else(|| eyre!("remote pane has no window label"))?
        .to_owned();
    if generation != expected_generation || fields.next().is_some() {
        return Err(eyre!("managed view belongs to a stale tmux server"));
    }
    let server = ServerIdentity::new(machine, socket, generation)?;
    let destination = PaneRef {
        server: server.clone(),
        pane: pane.clone(),
    };
    let route = Route::Ssh {
        destination: ssh.clone(),
        pane: destination,
    };
    Ok(ResolvedPasteTarget {
        source_tty: tty.to_path_buf(),
        source_pane: Some(source),
        route,
        machine,
        server,
        pane,
        cwd,
        ssh: Some(ssh),
        connections: Vec::new(),
        display_session,
        display_window,
    })
}

fn subscribe(machine: MachineId, session_name: &str) -> Result<()> {
    let workspace = current_workspace_name()?;
    let (generation, session_id) = remote_session_identity(machine, session_name)?;
    let state_lock = StateLock::acquire()?;
    let mut state = load_state()?;
    state.subscriptions.retain(|subscription| {
        subscription.machine != machine || subscription.session_id != session_id
    });
    state.subscriptions.push(Subscription {
        machine,
        session_id: session_id.clone(),
        session_name: session_name.to_owned(),
        server_generation: generation,
        workspace_id: workspace.clone(),
    });
    let subscribed_windows = remote_windows(state.subscriptions.last().unwrap())?
        .into_iter()
        .map(|window| window.id)
        .collect::<Vec<_>>();
    for view in &mut state.views {
        if view.machine == machine
            && view.server_generation == generation
            && subscribed_windows.contains(&view.window_id)
        {
            view.hidden = false;
            if !view.subscription_ids.contains(&session_id) {
                view.subscription_ids.push(session_id.clone());
            }
        }
    }
    state.stopped = false;
    save_state(&state)?;
    drop(state_lock);
    reconcile(&workspace)?;
    restart_sync_worker(&workspace)
}

fn remote_session_identity(machine: MachineId, session: &str) -> Result<(u32, String)> {
    let destination = machine
        .ssh_alias()
        .ok_or_else(|| eyre!("remote subscription requires code or training"))?;
    let output = remote_tmux_output(
        destination,
        [
            "display-message",
            "-p",
            "-t",
            &format!("={session}:"),
            "#{pid}\t#{session_id}",
        ],
    )?;
    let (generation, session_id) = output
        .trim()
        .split_once('\t')
        .ok_or_else(|| eyre!("remote tmux returned an invalid session identity"))?;
    let generation = generation.parse::<u32>()?;
    if generation == 0 || !valid_tmux_id(session_id, '$') {
        return Err(eyre!("remote tmux returned an invalid session identity"));
    }
    Ok((generation, session_id.to_owned()))
}

fn remote_windows(subscription: &Subscription) -> Result<Vec<RemoteWindow>> {
    let destination = subscription
        .machine
        .ssh_alias()
        .ok_or_else(|| eyre!("remote subscription requires code or training"))?;
    let identity = remote_session_identity(subscription.machine, &subscription.session_name)?;
    if identity
        != (
            subscription.server_generation,
            subscription.session_id.clone(),
        )
    {
        return Err(eyre!(
            "saved session {} on {destination} is unavailable or belongs to a newer tmux server",
            subscription.session_name
        ));
    }
    let output = remote_tmux_output(
        destination,
        [
            "list-windows",
            "-t",
            &format!("={}", subscription.session_name),
            "-F",
            "#{window_id}\t#{window_name}",
        ],
    )?;
    output
        .lines()
        .map(|line| {
            let (id, name) = line
                .split_once('\t')
                .ok_or_else(|| eyre!("remote tmux returned an invalid window record"))?;
            if !valid_tmux_id(id, '@') {
                return Err(eyre!("remote tmux returned an invalid window id"));
            }
            Ok(RemoteWindow {
                id: id.to_owned(),
                name: name.to_owned(),
            })
        })
        .collect()
}

pub(super) fn remote(args: RemoteArgs) -> Result<()> {
    if args.machine == MachineId::Mini {
        return Err(eyre!("remote sessions require code or training"));
    }
    let destination = args
        .machine
        .ssh_alias()
        .ok_or_else(|| eyre!("remote sessions require code or training"))?;
    if args.list || args.session.is_none() {
        return list_sessions(destination);
    }

    let session = args.session.as_deref().unwrap_or_default();
    validate_name("session name", session)?;
    let exists =
        remote_tmux_status(destination, ["has-session", "-t", &format!("={session}")])?.success();
    if args.attach && !exists {
        return Err(eyre!(
            "tmux session {session:?} does not exist on {destination}"
        ));
    }
    if !exists {
        require_success(
            remote_tmux_status(destination, ["new-session", "-d", "-s", session])?,
            "create remote tmux session",
        )?;
    }

    let mode = match args.mode {
        ViewMode::Auto if is_managed_workspace()? => ViewMode::Minimal,
        ViewMode::Auto => ViewMode::Normal,
        mode => mode,
    };
    match mode {
        ViewMode::Normal => attach_remote(destination, args.machine, session),
        ViewMode::Minimal if is_managed_workspace()? && args.window_id.is_none() => {
            subscribe(args.machine, session)
        }
        ViewMode::Minimal => attach_minimal(
            destination,
            args.machine,
            session,
            args.window_id.as_deref(),
        ),
        ViewMode::Auto => unreachable!(),
    }
}

fn list_sessions(destination: &str) -> Result<()> {
    let format = "#{session_id}\t#{session_name}\t#{session_windows}\t#{@fleet_role}";
    let remote = remote_command("tmux", &["list-sessions", "-F", format]);
    let output = Command::new("ssh")
        .args([destination, &remote])
        .output()
        .wrap_err("failed to list remote tmux sessions")?;
    if !output.status.success() {
        return Err(eyre!("failed to list tmux sessions on {destination}"));
    }
    let listing = String::from_utf8(output.stdout).wrap_err("tmux session list was not UTF-8")?;
    for line in listing.lines() {
        let Some((visible, role)) = line.rsplit_once('\t') else {
            continue;
        };
        if matches!(role, "view" | "service" | "display") {
            continue;
        }
        println!("{destination}\t{visible}");
    }
    Ok(())
}

fn attach_remote(destination: &str, machine: MachineId, session: &str) -> Result<()> {
    require_terminal()?;
    let _connection = begin_direct_connection(machine, session, None)?;
    let remote = remote_command("tmux", &["attach-session", "-t", &format!("={session}")]);
    let status = Command::new("ssh")
        .arg("-t")
        .args([destination, &remote])
        .status()
        .wrap_err("failed to start SSH")?;
    require_success(status, "attach remote tmux session")
}

fn attach_minimal(
    destination: &str,
    machine: MachineId,
    session: &str,
    requested_window: Option<&str>,
) -> Result<()> {
    require_terminal()?;
    let view = format!(
        "{RESERVED_PREFIX}view_{}_{session}_{}",
        machine_name(machine),
        std::process::id()
    );
    let target = format!("={session}:");
    let selected_window = if let Some(window) = requested_window {
        window.to_owned()
    } else {
        remote_tmux_output(
            destination,
            ["display-message", "-p", "-t", &target, "#{window_id}"],
        )?
        .trim()
        .to_owned()
    };
    let window = selected_window.as_str();
    if !window.starts_with('@')
        || !window[1..]
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return Err(eyre!("remote tmux returned an invalid window id"));
    }
    require_success(
        remote_tmux_status(
            destination,
            ["new-session", "-d", "-s", &view, "sleep", "2147483647"],
        )?,
        "create minimal tmux view",
    )?;
    let placeholder = remote_tmux_output(
        destination,
        [
            "display-message",
            "-p",
            "-t",
            &format!("={view}:"),
            "#{session_id}:#{window_index}",
        ],
    )?;
    let placeholder = placeholder.trim();
    let view_session = placeholder
        .split_once(':')
        .map(|(session, _)| session)
        .ok_or_else(|| eyre!("minimal view placeholder has no session id"))?;
    require_success(
        remote_tmux_status(
            destination,
            [
                "link-window",
                "-s",
                window,
                "-t",
                &format!("{view_session}:"),
            ],
        )?,
        "link canonical window into minimal view",
    )?;
    require_success(
        remote_tmux_status(destination, ["unlink-window", "-k", "-t", placeholder])?,
        "remove minimal view placeholder",
    )?;
    for (option, value) in [
        (VIEW_OPTION, "1"),
        (ROLE_OPTION, "view"),
        ("status", "off"),
        ("prefix", "None"),
        ("prefix2", "None"),
        ("key-table", "fleet-view"),
        ("mouse", "off"),
    ] {
        require_success(
            remote_tmux_status(
                destination,
                ["set-option", "-t", view_session, option, value],
            )?,
            "configure minimal tmux view",
        )?;
    }

    let _connection = begin_direct_connection(machine, session, Some(window))?;

    let remote = remote_command("tmux", &["attach-session", "-t", view_session]);
    let status = Command::new("ssh")
        .arg("-t")
        .args([destination, &remote])
        .status()
        .wrap_err("failed to attach minimal tmux view")?;
    let _ = remote_tmux_status(destination, ["kill-session", "-t", view_session]);
    require_success(status, "attach minimal tmux view")
}

struct PublishedConnection {
    id: ConnectionId,
    publication_generation: u64,
}

impl Drop for PublishedConnection {
    fn drop(&mut self) {
        let _ = remove_connection(&self.id, self.publication_generation);
    }
}

fn begin_direct_connection(
    machine: MachineId,
    session_name: &str,
    window: Option<&str>,
) -> Result<Option<PublishedConnection>> {
    let source_tty = match invoking_tty() {
        Ok(tty) => tty,
        Err(_) => return Ok(None),
    };
    let ssh = SshDestination::approved(machine)?;
    let exact_session = format!("={session_name}:");
    let target = window.unwrap_or(&exact_session);
    let output = remote_tmux_output(
        &ssh.alias,
        [
            "display-message",
            "-p",
            "-t",
            target,
            "#{socket_path}\t#{pid}\t#{session_id}\t#{window_id}\t#{pane_id}",
        ],
    )?;
    let mut fields = output.trim().split('\t');
    let socket = PathBuf::from(
        fields
            .next()
            .ok_or_else(|| eyre!("remote route has no socket"))?,
    );
    let generation = fields
        .next()
        .ok_or_else(|| eyre!("remote route has no generation"))?
        .parse::<u32>()?;
    let session = fields
        .next()
        .ok_or_else(|| eyre!("remote route has no session"))?
        .parse()?;
    let window = fields
        .next()
        .ok_or_else(|| eyre!("remote route has no window"))?
        .parse()?;
    let pane = fields
        .next()
        .ok_or_else(|| eyre!("remote route has no pane"))?
        .parse()?;
    if fields.next().is_some() {
        return Err(eyre!("remote route has extra fields"));
    }
    let server = ServerIdentity::new(machine, socket, generation)?;
    let pane_ref = PaneRef {
        server: server.clone(),
        pane,
    };
    let publication_generation = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;
    let id = ConnectionId::new(format!("{}-{publication_generation}", std::process::id()))?;
    let record = ConnectionRecord {
        id: id.clone(),
        owner_pid: std::process::id(),
        source_tty: source_tty.clone(),
        source_client: None,
        source_pane: local_source_for_tty(&source_tty).ok(),
        source_session: None,
        destination_session: Some(SessionRef {
            server: server.clone(),
            session,
        }),
        visible_window: Some(WindowRef {
            server: server.clone(),
            window,
        }),
        route: Route::Ssh {
            destination: ssh,
            pane: pane_ref,
        },
        publication_generation,
        status: ConnectionStatus::Live,
    };
    publish_connection(record)?;
    Ok(Some(PublishedConnection {
        id,
        publication_generation,
    }))
}

pub(super) fn workspace(args: WorkspaceArgs) -> Result<()> {
    match args.command {
        WorkspaceCommand::Attach {
            display,
            adopt,
            connection_id,
        } => {
            if let Some(connection_id) = connection_id.as_ref() {
                ConnectionId::new(connection_id.clone())?;
            }
            workspace_attach(&display, adopt.as_deref(), connection_id.as_deref())
        }
        WorkspaceCommand::Connect { display } => workspace_connect(&display),
        WorkspaceCommand::Reconnect => reconcile(saved_workspace_id()?.as_str()),
        WorkspaceCommand::Restore => workspace_restore(),
        WorkspaceCommand::Shutdown => workspace_shutdown(),
    }
}

fn workspace_attach(display: &str, adopt: Option<&str>, connection_id: Option<&str>) -> Result<()> {
    require_terminal()?;
    validate_name("display name", display)?;
    let canonical = adopt.unwrap_or("fleet");
    validate_name("workspace session", canonical)?;
    if !local_tmux_status(["has-session", "-t", &format!("={canonical}")])?.success() {
        if adopt.is_some() {
            return Err(eyre!("cannot adopt missing tmux session {canonical:?}"));
        }
        require_success(
            local_tmux_status(["new-session", "-d", "-s", canonical])?,
            "create workspace",
        )?;
    }
    require_success(
        local_tmux_status([
            "set-option",
            "-t",
            &format!("={canonical}"),
            WORKSPACE_OPTION,
            "1",
        ])?,
        "tag workspace",
    )?;
    require_success(
        local_tmux_status([
            "set-option",
            "-t",
            &format!("={canonical}"),
            ROLE_OPTION,
            "workspace",
        ])?,
        "tag workspace role",
    )?;
    require_success(
        local_tmux_status([
            "set-option",
            "-t",
            &format!("={canonical}"),
            WORKSPACE_NAME_OPTION,
            canonical,
        ])?,
        "record canonical workspace name",
    )?;
    let identity = connection_id
        .map(str::to_owned)
        .unwrap_or_else(|| std::process::id().to_string());
    let display_session = format!("{RESERVED_PREFIX}display_{display}_{identity}");
    require_success(
        local_tmux_status([
            "new-session",
            "-d",
            "-s",
            &display_session,
            "-t",
            &format!("={canonical}"),
        ])?,
        "create workspace display",
    )?;
    require_success(
        local_tmux_status([
            "set-option",
            "-t",
            &format!("={display_session}"),
            DISPLAY_OPTION,
            display,
        ])?,
        "tag workspace display",
    )?;
    require_success(
        local_tmux_status([
            "set-option",
            "-t",
            &format!("={display_session}"),
            ROLE_OPTION,
            "display",
        ])?,
        "tag workspace display role",
    )?;
    require_success(
        local_tmux_status([
            "set-option",
            "-t",
            &format!("={display_session}"),
            WORKSPACE_OPTION,
            "1",
        ])?,
        "tag workspace display membership",
    )?;
    require_success(
        local_tmux_status([
            "set-option",
            "-t",
            &format!("={display_session}"),
            WORKSPACE_NAME_OPTION,
            canonical,
        ])?,
        "record display workspace name",
    )?;
    if let Some(connection_id) = connection_id {
        require_success(
            local_tmux_status([
                "set-option",
                "-t",
                &format!("={display_session}"),
                "@fleet_connection_id",
                connection_id,
            ])?,
            "publish workspace connection identity",
        )?;
    }
    let display_session_id = local_tmux_output([
        "display-message",
        "-p",
        "-t",
        &format!("={display_session}:"),
        "#{pid}\t#{session_id}",
    ])?;
    let (display_generation, display_session_id) = display_session_id
        .trim()
        .split_once('\t')
        .ok_or_else(|| eyre!("workspace display has an invalid server identity"))?;
    save_workspace_id(canonical)?;
    let status = Command::new("tmux")
        .args(["attach-session", "-t", display_session_id])
        .status()
        .wrap_err("failed to attach workspace")?;
    let cleanup = local_tmux_output(["display-message", "-p", "#{pid}"])
        .ok()
        .filter(|generation| generation.trim() == display_generation)
        .map(|_| local_tmux_status(["kill-session", "-t", display_session_id]));
    require_success(status, "attach workspace")?;
    if let Some(cleanup) = cleanup {
        require_success(cleanup?, "remove workspace display")?;
    }
    Ok(())
}

fn workspace_connect(display: &str) -> Result<()> {
    require_terminal()?;
    validate_name("display name", display)?;
    let tty = invoking_tty()?;
    let publication_generation = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;
    let raw_id = format!("{}-{publication_generation}", std::process::id());
    let connection_id = ConnectionId::new(raw_id.clone())?;
    let mini = SshDestination::approved(MachineId::Mini)?;
    let remote = remote_command(
        "cmd",
        &[
            "tmux",
            "workspace",
            "attach",
            "--display",
            display,
            "--connection-id",
            &raw_id,
        ],
    );
    let mut child = Command::new("ssh")
        .arg("-t")
        .args([&mini.alias, &remote])
        .spawn()
        .wrap_err("failed to start managed Mini workspace connection")?;
    let display_session = format!("{RESERVED_PREFIX}display_{display}_{raw_id}");
    let record = wait_for_remote_display(
        &mini,
        &display_session,
        tty,
        connection_id.clone(),
        publication_generation,
    )?;
    publish_connection(record)?;
    let status = child.wait().wrap_err("managed Mini connection failed")?;
    remove_connection(&connection_id, publication_generation)?;
    require_success(status, "managed Mini workspace connection")
}

fn invoking_tty() -> Result<PathBuf> {
    let output = Command::new("tty")
        .output()
        .wrap_err("failed to identify invoking TTY")?;
    if !output.status.success() {
        return Err(eyre!("managed workspace connection requires a terminal"));
    }
    let tty = PathBuf::from(String::from_utf8(output.stdout)?.trim());
    if !tty.is_absolute() {
        return Err(eyre!("invoking TTY is not an absolute path"));
    }
    Ok(tty)
}

fn wait_for_remote_display(
    mini: &SshDestination,
    display_session: &str,
    source_tty: PathBuf,
    id: ConnectionId,
    publication_generation: u64,
) -> Result<ConnectionRecord> {
    let format =
        "#{socket_path}\t#{pid}\t#{session_id}\t#{window_id}\t#{pane_id}\t#{pane_current_path}";
    let mut last_error = None;
    for _ in 0..50 {
        match remote_tmux_output(
            &mini.alias,
            [
                "display-message",
                "-p",
                "-t",
                &format!("={display_session}:"),
                format,
            ],
        ) {
            Ok(output) => {
                return connection_from_remote_display(
                    mini,
                    source_tty,
                    id,
                    publication_generation,
                    &output,
                )
            }
            Err(error) => last_error = Some(error),
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err(last_error.unwrap_or_else(|| eyre!("managed Mini display was not published")))
}

fn connection_from_remote_display(
    mini: &SshDestination,
    source_tty: PathBuf,
    id: ConnectionId,
    publication_generation: u64,
    output: &str,
) -> Result<ConnectionRecord> {
    let mut fields = output.trim().split('\t');
    let socket = PathBuf::from(
        fields
            .next()
            .ok_or_else(|| eyre!("Mini route has no socket"))?,
    );
    let generation = fields
        .next()
        .ok_or_else(|| eyre!("Mini route has no generation"))?
        .parse::<u32>()?;
    let session = fields
        .next()
        .ok_or_else(|| eyre!("Mini route has no session"))?
        .parse()?;
    let window = fields
        .next()
        .ok_or_else(|| eyre!("Mini route has no window"))?
        .parse()?;
    let pane = fields
        .next()
        .ok_or_else(|| eyre!("Mini route has no pane"))?
        .parse()?;
    let _cwd = PathBuf::from(
        fields
            .next()
            .ok_or_else(|| eyre!("Mini route has no cwd"))?,
    );
    if fields.next().is_some() {
        return Err(eyre!("Mini route has extra fields"));
    }
    let server = ServerIdentity::new(MachineId::Mini, socket, generation)?;
    let pane_ref = PaneRef {
        server: server.clone(),
        pane,
    };
    Ok(ConnectionRecord {
        id,
        owner_pid: std::process::id(),
        source_tty,
        source_client: None,
        source_pane: None,
        source_session: None,
        destination_session: Some(SessionRef {
            server: server.clone(),
            session,
        }),
        visible_window: Some(WindowRef {
            server: server.clone(),
            window,
        }),
        route: Route::Ssh {
            destination: mini.clone(),
            pane: pane_ref,
        },
        publication_generation,
        status: ConnectionStatus::Live,
    })
}

fn publish_connection(record: ConnectionRecord) -> Result<()> {
    let _state_lock = StateLock::acquire()?;
    let mut state = load_state()?;
    state
        .connections
        .retain(|existing| existing.id != record.id);
    state.connections.push(record);
    save_state(&state)
}

fn remove_connection(id: &ConnectionId, publication_generation: u64) -> Result<()> {
    let _state_lock = StateLock::acquire()?;
    let mut state = load_state()?;
    state.connections.retain(|record| {
        &record.id != id || record.publication_generation != publication_generation
    });
    save_state(&state)
}

fn workspace_shutdown() -> Result<()> {
    let state_lock = StateLock::acquire()?;
    let sessions = tagged_sessions(SERVICE_OPTION)?;
    for session in sessions {
        require_success(
            local_tmux_status(["kill-session", "-t", &session])?,
            "stop workspace service",
        )?;
    }
    let displays = tagged_sessions(DISPLAY_OPTION)?;
    for session in displays {
        require_success(
            local_tmux_status(["kill-session", "-t", &session])?,
            "remove workspace display",
        )?;
    }
    let mut state = load_state()?;
    for view in &state.views {
        remove_owned_outer_view(view)?;
    }
    state.views.clear();
    state.subscriptions.clear();
    state.connections.clear();
    state.stopped = true;
    save_state(&state)?;
    drop(state_lock);
    Ok(())
}

fn workspace_restore() -> Result<()> {
    let workspace = saved_workspace_id()?;
    let state_lock = StateLock::acquire()?;
    let mut state = load_state()?;
    state.stopped = false;
    save_state(&state)?;
    drop(state_lock);
    reconcile(&workspace)?;
    start_sync_worker(&workspace)
}

pub(super) fn sync(args: SyncArgs) -> Result<()> {
    match args.command {
        SyncCommand::Reconcile { workspace } => reconcile(&workspace),
        SyncCommand::Worker { workspace } => sync_worker(&workspace),
    }
}

fn sync_worker(workspace: &str) -> Result<()> {
    validate_name("workspace id", workspace)?;
    reconcile(workspace)?;
    let (sender, receiver) = mpsc::channel();
    let mut watchers = Vec::new();
    for machine in [MachineId::Code, MachineId::Training] {
        let candidates = load_state()?
            .subscriptions
            .into_iter()
            .filter(|subscription| {
                subscription.workspace_id == workspace && subscription.machine == machine
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            continue;
        }
        let subscription = select_available_subscription(candidates, refresh_subscription)?;
        watchers.push(spawn_control_watcher(subscription, sender.clone())?);
    }
    if watchers.is_empty() {
        return Ok(());
    }
    let mut backoff = Duration::from_secs(1);
    loop {
        match receiver.recv() {
            Ok(ControlEvent::Changed) => {
                reconcile(workspace)?;
                backoff = Duration::from_secs(1);
            }
            Ok(ControlEvent::Disconnected(machine)) => {
                eprintln!(
                    "tmux sync connection to {} disconnected",
                    machine_name(machine)
                );
                thread::sleep(backoff);
                backoff = (backoff * 2).min(Duration::from_secs(30));
                let candidates = load_state()?
                    .subscriptions
                    .into_iter()
                    .filter(|subscription| {
                        subscription.workspace_id == workspace && subscription.machine == machine
                    })
                    .collect::<Vec<_>>();
                if candidates.is_empty() {
                    return Err(eyre!("remote tmux subscription was removed"));
                }
                let subscription = select_available_subscription(candidates, refresh_subscription)?;
                watchers.push(spawn_control_watcher(subscription, sender.clone())?);
            }
            Err(_) => return Err(eyre!("all remote tmux control connections ended")),
        }
    }
}

fn select_available_subscription(
    candidates: Vec<Subscription>,
    mut refresh: impl FnMut(&mut Subscription) -> Result<()>,
) -> Result<Subscription> {
    let mut failures = Vec::new();
    for mut candidate in candidates {
        match refresh(&mut candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) => failures.push(format!("{}: {error:#}", candidate.session_id)),
        }
    }
    Err(eyre!(
        "no subscribed canonical session is available: {}",
        failures.join("; ")
    ))
}

enum ControlEvent {
    Changed,
    Disconnected(MachineId),
}

fn spawn_control_watcher(
    subscription: Subscription,
    sender: mpsc::Sender<ControlEvent>,
) -> Result<std::process::Child> {
    let destination = subscription
        .machine
        .ssh_alias()
        .ok_or_else(|| eyre!("sync watcher requires a remote machine"))?;
    let remote = remote_command(
        "tmux",
        &[
            "-C",
            "attach-session",
            "-E",
            "-f",
            "no-output,ignore-size",
            "-t",
            &format!("={}", subscription.session_name),
        ],
    );
    let mut child = Command::new("ssh")
        .args([destination, &remote])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .wrap_err_with(|| format!("failed to start tmux control connection to {destination}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| eyre!("tmux control connection has no output stream"))?;
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            match line {
                Ok(line) if line.starts_with('%') => {
                    if sender.send(ControlEvent::Changed).is_err() {
                        return;
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        let _ = sender.send(ControlEvent::Disconnected(subscription.machine));
    });
    Ok(child)
}

fn reconcile(workspace: &str) -> Result<()> {
    validate_name("workspace id", workspace)?;
    if !local_tmux_status(["has-session", "-t", &format!("={workspace}")])?.success() {
        return Err(eyre!("managed workspace {workspace:?} is unavailable"));
    }
    let _state_lock = StateLock::acquire()?;
    let mut state = load_state()?;
    let subscriptions = state
        .subscriptions
        .iter()
        .filter(|subscription| subscription.workspace_id == workspace)
        .cloned()
        .collect::<Vec<_>>();
    let mut desired = Vec::new();
    let mut desired_owners = HashMap::<String, Vec<String>>::new();
    let mut unavailable_generations = HashSet::new();
    for mut subscription in subscriptions {
        match refresh_subscription(&mut subscription) {
            Ok(()) => {
                if let Some(saved) = state.subscriptions.iter_mut().find(|saved| {
                    saved.machine == subscription.machine
                        && saved.server_generation == subscription.server_generation
                        && saved.session_id == subscription.session_id
                }) {
                    saved.session_name = subscription.session_name.clone();
                }
            }
            Err(error) => {
                eprintln!("{error:#}");
                unavailable_generations
                    .insert((subscription.machine, subscription.server_generation));
                continue;
            }
        }
        let windows = match remote_windows(&subscription) {
            Ok(windows) => windows,
            Err(error) => {
                eprintln!("{error:#}");
                unavailable_generations
                    .insert((subscription.machine, subscription.server_generation));
                continue;
            }
        };
        for window in windows {
            let key = view_key(
                subscription.machine,
                subscription.server_generation,
                &window.id,
            );
            let owners = desired_owners.entry(key).or_default();
            if !owners.contains(&subscription.session_id) {
                owners.push(subscription.session_id.clone());
            }
            if !desired.iter().any(
                |(machine, generation, existing): &(MachineId, u32, RemoteWindow)| {
                    *machine == subscription.machine
                        && *generation == subscription.server_generation
                        && existing.id == window.id
                },
            ) {
                desired.push((subscription.machine, subscription.server_generation, window));
            }
        }
    }
    let decisions = reconcile_decisions(
        desired
            .iter()
            .map(|(machine, generation, window)| view_key(*machine, *generation, &window.id)),
        &state.views,
    );

    for (machine, generation, window) in &desired {
        let key = view_key(*machine, *generation, &window.id);
        let Some(view) = state.views.iter_mut().find(|view| {
            view.machine == *machine
                && view.server_generation == *generation
                && view.window_id == window.id
        }) else {
            continue;
        };
        view.subscription_ids = desired_owners.get(&key).cloned().unwrap_or_default();
        let generated = format!("{} · {}", machine_name(*machine), window.name);
        if !view.generated_label.is_empty() && local_window_exists(&view.outer_window_id)? {
            let current = local_tmux_output([
                "display-message",
                "-p",
                "-t",
                &view.outer_window_id,
                "#{window_name}",
            ])?;
            if current.trim() == view.generated_label && current.trim() != generated {
                require_success(
                    local_tmux_status(["rename-window", "-t", &view.outer_window_id, &generated])?,
                    "update managed view label",
                )?;
            }
        }
        view.generated_label = generated;
    }

    for (machine, generation, window) in &desired {
        let key = view_key(*machine, *generation, &window.id);
        if decisions.iter().any(
            |decision| matches!(decision, ReconcileDecision::Hidden(existing) if existing == &key),
        ) {
            continue;
        }
        if let Some(view) = state.views.iter().find(|view| {
            view.machine == *machine
                && view.server_generation == *generation
                && view.window_id == window.id
        }) {
            if view.hidden || local_window_exists(&view.outer_window_id)? {
                continue;
            }
        }
        let subscription = state
            .subscriptions
            .iter()
            .find(|subscription| {
                subscription.machine == *machine
                    && subscription.server_generation == *generation
                    && subscription.workspace_id == workspace
            })
            .ok_or_else(|| eyre!("managed view has no owning subscription"))?;
        let outer = create_outer_view(workspace, subscription, window)?;
        if let Some(view) = state.views.iter_mut().find(|view| {
            view.machine == *machine
                && view.server_generation == *generation
                && view.window_id == window.id
        }) {
            view.outer_window_id = outer;
            view.hidden = false;
        } else {
            state.views.push(ManagedView {
                machine: *machine,
                server_generation: *generation,
                window_id: window.id.clone(),
                outer_window_id: outer,
                hidden: false,
                subscription_ids: vec![subscription.session_id.clone()],
                generated_label: format!("{} · {}", machine_name(*machine), window.name),
            });
        }
    }

    let obsolete = state
        .views
        .iter()
        .filter(|view| {
            if unavailable_generations.contains(&(view.machine, view.server_generation)) {
                return false;
            }
            decisions.iter().any(|decision| {
                matches!(decision, ReconcileDecision::Remove(key) if key == &view_key(view.machine, view.server_generation, &view.window_id))
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    for view in &obsolete {
        remove_owned_outer_view(view)?;
    }
    state.views.retain(|view| !obsolete.contains(view));
    save_state(&state)?;
    Ok(())
}

fn refresh_subscription(subscription: &mut Subscription) -> Result<()> {
    let destination = subscription
        .machine
        .ssh_alias()
        .ok_or_else(|| eyre!("remote subscription requires code or training"))?;
    let output = remote_tmux_output(
        destination,
        [
            "list-sessions",
            "-F",
            "#{pid}\t#{session_id}\t#{session_name}",
        ],
    )?;
    let record = output.lines().find_map(|line| {
        let mut fields = line.split('\t');
        let generation = fields.next()?.parse::<u32>().ok()?;
        let id = fields.next()?;
        let name = fields.next()?;
        (id == subscription.session_id).then(|| (generation, name.to_owned()))
    });
    let (generation, name) = record.ok_or_else(|| {
        eyre!(
            "subscribed remote session {} is unavailable",
            subscription.session_id
        )
    })?;
    if generation != subscription.server_generation {
        return Err(eyre!(
            "subscribed remote session belongs to a newer tmux server"
        ));
    }
    subscription.session_name = name;
    Ok(())
}

fn create_outer_view(
    workspace: &str,
    subscription: &Subscription,
    window: &RemoteWindow,
) -> Result<String> {
    let executable = env::current_exe()?;
    let machine = machine_name(subscription.machine);
    let command = [
        shell_word(executable.to_string_lossy().as_ref()),
        "tmux remote".to_owned(),
        machine.to_owned(),
        shell_word(&subscription.session_name),
        "--attach --mode minimal --window-id".to_owned(),
        shell_word(&window.id),
    ]
    .join(" ");
    let label = format!("{machine} · {}", window.name);
    let output = Command::new("tmux")
        .args([
            "new-window",
            "-d",
            "-P",
            "-F",
            "#{window_id}",
            "-t",
            &format!("={workspace}:"),
            "-n",
            &label,
            &command,
        ])
        .output()?;
    if !output.status.success() {
        return Err(eyre!("failed to create managed outer view"));
    }
    let outer = String::from_utf8(output.stdout)?.trim().to_owned();
    if !valid_tmux_id(&outer, '@') {
        return Err(eyre!("tmux returned an invalid outer window id"));
    }
    let key = view_key(
        subscription.machine,
        subscription.server_generation,
        &window.id,
    );
    require_success(
        local_tmux_status(["set-option", "-w", "-t", &outer, "@fleet_view_key", &key])?,
        "tag managed outer view",
    )?;
    Ok(outer)
}

fn remove_owned_outer_view(view: &ManagedView) -> Result<()> {
    if !local_window_exists(&view.outer_window_id)? {
        return Ok(());
    }
    let expected = view_key(view.machine, view.server_generation, &view.window_id);
    let actual = local_tmux_output([
        "show-option",
        "-wqv",
        "-t",
        &view.outer_window_id,
        "@fleet_view_key",
    ])?;
    if actual.trim() != expected {
        return Err(eyre!(
            "refusing to remove a tmux window that is not an owned view"
        ));
    }
    require_success(
        local_tmux_status(["kill-window", "-t", &view.outer_window_id])?,
        "remove managed outer view",
    )
}

fn local_window_exists(window_id: &str) -> Result<bool> {
    Ok(local_tmux_status(["display-message", "-p", "-t", window_id, "#{window_id}"])?.success())
}

fn view_key(machine: MachineId, generation: u32, window_id: &str) -> String {
    format!("{}:{generation}:{window_id}", machine_name(machine))
}

fn shell_word(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub(super) fn close(args: CloseArgs) -> Result<()> {
    let target = args.target_pane.or_else(|| env::var("TMUX_PANE").ok());
    let target = target.ok_or_else(|| eyre!("tmux pane target is not set"))?;
    if !target.starts_with('%')
        || !target[1..]
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return Err(eyre!("invalid tmux pane id: {target:?}"));
    }
    let selected = local_tmux_output([
        "display-message",
        "-p",
        "-t",
        &target,
        "#{window_id}\t#{@fleet_view_key}",
    ])?;
    let (outer_window, key) = selected.trim().split_once('\t').unwrap_or_default();
    if key.is_empty() {
        return require_success(
            local_tmux_status(["kill-pane", "-t", &target])?,
            "close tmux pane",
        );
    }
    let _state_lock = StateLock::acquire()?;
    let mut state = load_state()?;
    let view = state
        .views
        .iter_mut()
        .find(|view| {
            view.outer_window_id == outer_window
                && view_key(view.machine, view.server_generation, &view.window_id) == key
        })
        .ok_or_else(|| eyre!("managed pane metadata is stale"))?;
    view.hidden = true;
    save_state(&state)?;
    require_success(
        local_tmux_status(["kill-window", "-t", outer_window])?,
        "hide managed outer view",
    )
}

pub(super) fn unsubscribe(args: UnsubscribeArgs) -> Result<()> {
    validate_name("session id", &args.session_id)?;
    let workspace = args.workspace.unwrap_or(saved_workspace_id()?);
    let state_lock = StateLock::acquire()?;
    let mut state = load_state()?;
    state.subscriptions.retain(|subscription| {
        subscription.machine != args.machine
            || subscription.session_id != args.session_id
            || subscription.workspace_id != workspace
    });
    save_state(&state)?;
    drop(state_lock);
    reconcile(&workspace)
}

fn is_managed_workspace() -> Result<bool> {
    if env::var_os("TMUX").is_none() {
        return Ok(false);
    }
    let output = Command::new("tmux")
        .args(["show-option", "-qv", WORKSPACE_OPTION])
        .output()
        .wrap_err("failed to inspect current tmux session")?;
    Ok(output.status.success() && output.stdout == b"1\n")
}

fn tagged_sessions(option: &str) -> Result<Vec<String>> {
    let format = format!("#{{session_id}}\t#{{{option}}}");
    let output = Command::new("tmux")
        .args(["list-sessions", "-F", &format])
        .output()?;
    if !output.status.success() {
        return Ok(Vec::new());
    }
    let text = String::from_utf8(output.stdout)?;
    Ok(text
        .lines()
        .filter_map(|line| line.strip_suffix("\t1").map(str::to_owned))
        .collect())
}

fn remote_tmux_status<const N: usize>(destination: &str, args: [&str; N]) -> Result<ExitStatus> {
    let remote = remote_command("tmux", &args);
    Command::new("ssh")
        .arg(destination)
        .arg(remote)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .wrap_err_with(|| format!("failed to run tmux on {destination}"))
}

fn remote_tmux_output<const N: usize>(destination: &str, args: [&str; N]) -> Result<String> {
    let remote = remote_command("tmux", &args);
    let output = Command::new("ssh")
        .arg(destination)
        .arg(remote)
        .output()
        .wrap_err_with(|| format!("failed to query tmux on {destination}"))?;
    if !output.status.success() {
        return Err(eyre!("tmux query failed on {destination}"));
    }
    String::from_utf8(output.stdout).wrap_err("tmux query was not UTF-8")
}

fn remote_command(program: &str, args: &[&str]) -> String {
    std::iter::once(program)
        .chain(args.iter().copied())
        .map(shell_word)
        .collect::<Vec<_>>()
        .join(" ")
}

fn local_tmux_status<const N: usize>(args: [&str; N]) -> Result<ExitStatus> {
    Command::new("tmux")
        .args(args)
        .status()
        .wrap_err("failed to run tmux")
}

fn local_tmux_output<const N: usize>(args: [&str; N]) -> Result<String> {
    let output = Command::new("tmux").args(args).output()?;
    if !output.status.success() {
        return Err(eyre!("tmux query failed"));
    }
    String::from_utf8(output.stdout).wrap_err("tmux query was not UTF-8")
}

fn current_workspace_name() -> Result<String> {
    let output = local_tmux_output([
        "display-message",
        "-p",
        "#{@fleet_workspace_name}\t#{@fleet_workspace}",
    ])?;
    let (name, marker) = output
        .trim()
        .split_once('\t')
        .ok_or_else(|| eyre!("current tmux session is not a managed workspace"))?;
    if marker != "1" {
        return Err(eyre!("current tmux session is not a managed workspace"));
    }
    validate_name("workspace name", name)?;
    Ok(name.to_owned())
}

fn start_sync_worker(workspace: &str) -> Result<()> {
    if !tagged_sessions(SERVICE_OPTION)?.is_empty() {
        return Ok(());
    }
    let service = format!("{RESERVED_PREFIX}service_{workspace}");
    let executable = env::current_exe()?;
    let command = format!(
        "{} tmux sync worker --workspace {}",
        shell_word(executable.to_string_lossy().as_ref()),
        shell_word(workspace)
    );
    require_success(
        local_tmux_status(["new-session", "-d", "-s", &service, &command])?,
        "start workspace sync worker",
    )?;
    for (option, value) in [(SERVICE_OPTION, "1"), (ROLE_OPTION, "service")] {
        require_success(
            local_tmux_status(["set-option", "-t", &format!("={service}"), option, value])?,
            "tag workspace sync worker",
        )?;
    }
    require_success(
        local_tmux_status([
            "set-option",
            "-w",
            "-t",
            &format!("={service}"),
            "remain-on-exit",
            "on",
        ])?,
        "configure sync worker supervision",
    )?;
    let pane = local_tmux_output([
        "display-message",
        "-p",
        "-t",
        &format!("={service}:"),
        "#{pane_id}",
    ])?;
    let respawn = format!(
        "respawn-pane -k -t {} {}",
        pane.trim(),
        shell_word(&command)
    );
    require_success(
        local_tmux_status([
            "set-hook",
            "-t",
            &format!("={service}"),
            "pane-died",
            &respawn,
        ])?,
        "install sync worker restart hook",
    )?;
    Ok(())
}

fn restart_sync_worker(workspace: &str) -> Result<()> {
    for session in tagged_sessions(SERVICE_OPTION)? {
        require_success(
            local_tmux_status(["kill-session", "-t", &session])?,
            "restart workspace sync worker",
        )?;
    }
    start_sync_worker(workspace)
}

fn valid_tmux_id(value: &str, prefix: char) -> bool {
    value.strip_prefix(prefix).is_some_and(|suffix| {
        !suffix.is_empty() && suffix.chars().all(|character| character.is_ascii_digit())
    })
}

fn require_terminal() -> Result<()> {
    if !io::stdin().is_terminal() {
        return Err(eyre!("tmux attachment requires a terminal"));
    }
    Ok(())
}

fn require_success(status: ExitStatus, operation: &str) -> Result<()> {
    if status.success() {
        return Ok(());
    }
    Err(eyre!("{operation} failed with {status}"))
}

fn machine_name(machine: MachineId) -> &'static str {
    match machine {
        MachineId::Mini => "mini",
        MachineId::Code => "code",
        MachineId::Training => "training",
    }
}

fn state_dir() -> Result<PathBuf> {
    let base = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .ok_or_else(|| eyre!("HOME and XDG_STATE_HOME are not set"))?;
    Ok(base.join("cmd/fleet-tmux"))
}

fn state_path() -> Result<PathBuf> {
    Ok(state_dir()?.join("workspace.json"))
}

struct StateLock(PathBuf);

impl StateLock {
    fn acquire() -> Result<Self> {
        let directory = state_dir()?;
        fs::create_dir_all(&directory)?;
        let path = directory.join("workspace.lock");
        for _ in 0..200 {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut file) => {
                    writeln!(file, "{}", std::process::id())?;
                    return Ok(Self(path));
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    let stale = fs::read_to_string(&path)
                        .ok()
                        .and_then(|value| value.trim().parse::<u32>().ok())
                        .is_none_or(|pid| !process_is_live(pid));
                    if stale {
                        let _ = fs::remove_file(&path);
                        continue;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => return Err(error).wrap_err("failed to lock fleet tmux state"),
            }
        }
        Err(eyre!("timed out waiting for fleet tmux state lock"))
    }
}

impl Drop for StateLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn load_state() -> Result<WorkspaceState> {
    match fs::read(state_path()?) {
        Ok(raw) => serde_json::from_slice(&raw).wrap_err("invalid fleet tmux state"),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(WorkspaceState::default()),
        Err(error) => Err(error).wrap_err("failed to read fleet tmux state"),
    }
}

fn save_state(state: &WorkspaceState) -> Result<()> {
    let directory = state_dir()?;
    fs::create_dir_all(&directory)?;
    let mut temporary = tempfile::NamedTempFile::new_in(&directory)?;
    serde_json::to_writer_pretty(&mut temporary, state)?;
    temporary.persist(state_path()?)?;
    Ok(())
}

fn save_workspace_id(workspace: &str) -> Result<()> {
    fs::create_dir_all(state_dir()?)?;
    fs::write(state_dir()?.join("workspace-id"), workspace)?;
    Ok(())
}

fn saved_workspace_id() -> Result<String> {
    let workspace = fs::read_to_string(state_dir()?.join("workspace-id"))?;
    validate_name("saved workspace id", &workspace)?;
    Ok(workspace)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn state_round_trip_preserves_typed_machine() {
        let state = WorkspaceState {
            subscriptions: vec![Subscription {
                machine: MachineId::Code,
                session_id: "$1".to_owned(),
                session_name: "work".to_owned(),
                server_generation: 42,
                workspace_id: "$2".to_owned(),
            }],
            views: Vec::new(),
            connections: Vec::new(),
            stopped: false,
        };
        let encoded = serde_json::to_vec(&state).unwrap();
        assert_eq!(
            serde_json::from_slice::<WorkspaceState>(&encoded)
                .unwrap()
                .subscriptions,
            state.subscriptions
        );
    }

    #[test]
    fn session_names_reject_tmux_target_syntax() {
        assert!(validate_name("session", "work").is_ok());
        assert!(validate_name("session", "work:1").is_err());
        assert!(validate_name("session", "-Lother").is_err());
    }

    fn view(machine: MachineId, generation: u32, window: &str, hidden: bool) -> ManagedView {
        ManagedView {
            machine,
            server_generation: generation,
            window_id: window.to_owned(),
            outer_window_id: "@99".to_owned(),
            hidden,
            subscription_ids: vec!["$1".to_owned()],
            generated_label: "code · work".to_owned(),
        }
    }

    #[test]
    fn reconciliation_deduplicates_shared_and_out_of_order_windows() {
        let desired = vec![
            "code:42:@2".to_owned(),
            "code:42:@1".to_owned(),
            "code:42:@2".to_owned(),
        ];
        assert_eq!(
            reconcile_decisions(desired, &[]),
            vec![
                ReconcileDecision::Create("code:42:@1".to_owned()),
                ReconcileDecision::Create("code:42:@2".to_owned()),
            ]
        );
    }

    #[test]
    fn reconciliation_preserves_hidden_windows_until_explicit_restore() {
        let views = vec![view(MachineId::Code, 42, "@1", true)];
        assert_eq!(
            reconcile_decisions(["code:42:@1".to_owned()], &views),
            vec![ReconcileDecision::Hidden("code:42:@1".to_owned())]
        );
        let restored = vec![view(MachineId::Code, 42, "@1", false)];
        assert_eq!(
            reconcile_decisions(["code:42:@1".to_owned()], &restored),
            vec![ReconcileDecision::Keep("code:42:@1".to_owned())]
        );
    }

    #[test]
    fn late_generation_cannot_keep_or_remove_new_generation_view() {
        let views = vec![view(MachineId::Code, 43, "@1", false)];
        assert_eq!(
            reconcile_decisions(["code:42:@1".to_owned()], &views),
            vec![
                ReconcileDecision::Create("code:42:@1".to_owned()),
                ReconcileDecision::Remove("code:43:@1".to_owned()),
            ]
        );
    }

    #[test]
    fn reconciliation_removes_disappeared_remote_windows() {
        let views = vec![view(MachineId::Training, 7, "@8", false)];
        assert_eq!(
            reconcile_decisions([], &views),
            vec![ReconcileDecision::Remove("training:7:@8".to_owned())]
        );
    }

    #[test]
    fn tmux_ids_use_exact_stable_target_syntax() {
        assert!(valid_tmux_id("$12", '$'));
        assert!(valid_tmux_id("@8", '@'));
        assert!(!valid_tmux_id("session-prefix", '$'));
        assert!(!valid_tmux_id("@8.1", '@'));
    }

    #[test]
    fn watcher_fails_over_to_a_surviving_stable_subscription() {
        let removed = Subscription {
            machine: MachineId::Code,
            session_id: "$1".to_owned(),
            session_name: "removed".to_owned(),
            server_generation: 42,
            workspace_id: "fleet".to_owned(),
        };
        let survivor = Subscription {
            session_id: "$2".to_owned(),
            session_name: "old-name".to_owned(),
            ..removed.clone()
        };
        let selected = select_available_subscription(vec![removed, survivor], |candidate| {
            if candidate.session_id == "$1" {
                return Err(eyre!("session was removed"));
            }
            candidate.session_name = "renamed-live-session".to_owned();
            Ok(())
        })
        .unwrap();
        assert_eq!(selected.session_id, "$2");
        assert_eq!(selected.session_name, "renamed-live-session");
    }

    #[test]
    fn remote_command_survives_openssh_shell_joining() {
        let marker = tempfile::NamedTempFile::new().unwrap();
        let marker_path = marker.path().to_path_buf();
        drop(marker);
        let malicious = format!("$(touch {})", marker_path.display());
        let command = remote_command(
            "printf",
            &[
                "<%s>\\n",
                "#{session_id}\t#{session_name}",
                "name with spaces and ' quote",
                "=exact:target",
                &malicious,
            ],
        );
        let output = Command::new("sh").args(["-c", &command]).output().unwrap();
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!(
                "<#{{session_id}}\t#{{session_name}}>\n<name with spaces and ' quote>\n<=exact:target>\n<{malicious}>\n"
            )
        );
        assert!(!marker_path.exists());
    }

    #[test]
    fn disposable_tmux_proves_view_ownership_and_option_scope() {
        if Command::new("tmux").arg("-V").output().is_err() {
            return;
        }
        let socket = format!("cmd-test-{}-{}", std::process::id(), rand::random::<u32>());
        let tmux = |args: &[&str]| {
            Command::new("tmux")
                .args(["-L", &socket, "-f", "/dev/null"])
                .args(args)
                .output()
                .unwrap()
        };
        let cleanup = TestServer {
            socket: socket.clone(),
        };
        assert!(tmux(&["new-session", "-d", "-s", "normal", "sleep", "60"])
            .status
            .success());
        let window = String::from_utf8(
            tmux(&["display-message", "-p", "-t", "=normal:", "#{window_id}"]).stdout,
        )
        .unwrap()
        .trim()
        .to_owned();
        let pane_pid = String::from_utf8(
            tmux(&["display-message", "-p", "-t", &window, "#{pane_pid}"]).stdout,
        )
        .unwrap()
        .trim()
        .to_owned();
        let initial_size = String::from_utf8(
            tmux(&[
                "display-message",
                "-p",
                "-t",
                &window,
                "#{window_width}x#{window_height}",
            ])
            .stdout,
        )
        .unwrap();
        let mut observer = Command::new("tmux")
            .args([
                "-L",
                &socket,
                "-C",
                "attach-session",
                "-f",
                "no-output,ignore-size",
                "-t",
                "normal",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        thread::sleep(Duration::from_millis(50));
        let observed_size = String::from_utf8(
            tmux(&[
                "display-message",
                "-p",
                "-t",
                &window,
                "#{window_width}x#{window_height}",
            ])
            .stdout,
        )
        .unwrap();
        assert_eq!(observed_size, initial_size);
        let _ = observer.kill();
        let _ = observer.wait();

        assert!(tmux(&["new-session", "-d", "-s", "view", "sleep", "60"])
            .status
            .success());
        let placeholder = String::from_utf8(
            tmux(&[
                "display-message",
                "-p",
                "-t",
                "=view:",
                "#{session_id}:#{window_index}",
            ])
            .stdout,
        )
        .unwrap()
        .trim()
        .to_owned();
        let view_session = placeholder.split_once(':').unwrap().0;
        assert!(tmux(&[
            "link-window",
            "-s",
            &window,
            "-t",
            &format!("{view_session}:")
        ])
        .status
        .success());
        assert!(tmux(&["unlink-window", "-k", "-t", &placeholder])
            .status
            .success());
        for (option, value) in [
            (ROLE_OPTION, "view"),
            ("status", "off"),
            ("prefix", "None"),
            ("key-table", "fleet-view"),
            ("mouse", "off"),
        ] {
            let output = tmux(&["set-option", "-t", view_session, option, value]);
            assert!(
                output.status.success(),
                "{option}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        assert_eq!(
            String::from_utf8(
                tmux(&["show-option", "-qv", "-t", view_session, ROLE_OPTION]).stdout
            )
            .unwrap()
            .trim(),
            "view"
        );
        assert_eq!(
            String::from_utf8(tmux(&["show-option", "-qv", "-t", view_session, "prefix"]).stdout)
                .unwrap()
                .trim(),
            "None"
        );
        assert_ne!(
            String::from_utf8(tmux(&["show-option", "-qv", "-t", "=normal", "prefix"]).stdout)
                .unwrap()
                .trim(),
            "None"
        );
        let linked_pid = String::from_utf8(
            tmux(&["display-message", "-p", "-t", "=view:", "#{pane_pid}"]).stdout,
        )
        .unwrap();
        assert_eq!(linked_pid.trim(), pane_pid);

        assert!(
            tmux(&["new-session", "-d", "-s", "normal-two", "sleep", "60"])
                .status
                .success()
        );
        let second_placeholder = String::from_utf8(
            tmux(&[
                "display-message",
                "-p",
                "-t",
                "=normal-two:",
                "#{session_id}:#{window_index}",
            ])
            .stdout,
        )
        .unwrap()
        .trim()
        .to_owned();
        let second_session = second_placeholder.split_once(':').unwrap().0;
        assert!(tmux(&[
            "link-window",
            "-s",
            &window,
            "-t",
            &format!("{second_session}:")
        ])
        .status
        .success());
        assert!(tmux(&["unlink-window", "-k", "-t", &second_placeholder])
            .status
            .success());
        assert!(tmux(&["kill-session", "-t", "=normal"]).status.success());
        assert!(
            tmux(&["display-message", "-p", "-t", &window, "#{window_id}"])
                .status
                .success()
        );
        assert!(tmux(&["kill-session", "-t", "=normal-two"])
            .status
            .success());
        assert!(
            tmux(&["display-message", "-p", "-t", &window, "#{window_id}"])
                .status
                .success()
        );
        assert!(tmux(&["kill-session", "-t", view_session]).status.success());
        assert!(
            !tmux(&["display-message", "-p", "-t", &window, "#{window_id}"])
                .status
                .success()
        );
        drop(cleanup);
    }

    struct TestServer {
        socket: String,
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            let _ = Command::new("tmux")
                .args(["-L", &self.socket, "kill-server"])
                .output();
        }
    }
}
