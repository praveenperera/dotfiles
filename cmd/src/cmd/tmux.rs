use super::codex::app_server::{
    current_loaded_thread, set_thread_name, SessionControl, SessionMarker,
};
use clap::{Args, Parser, Subcommand, ValueEnum};
use eyre::{eyre, Result, WrapErr};
use serde::Deserialize;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::{Builder as TempFileBuilder, NamedTempFile};
use xshell::{cmd, Shell};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum NotifyKind {
    /// Send BEL character (marks tmux window)
    Bell,
    /// Send macOS desktop notification via osascript
    Macos,
}

#[derive(Debug, Clone, Args)]
pub struct Tmux {
    #[command(subcommand)]
    pub subcommand: TmuxCmd,
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "notf",
    about = "Send terminal notification (bell, macos, or both)",
    arg_required_else_help = true
)]
pub struct NotifyArgs {
    /// Notification message (used with macos)
    message: Option<String>,
    /// Notification types (comma-separated: bell, macos)
    #[arg(short = 'T', long = "type", value_delimiter = ',')]
    kind: Vec<NotifyKind>,
    /// Notification title (default: "{window_name} Notification")
    #[arg(short, long)]
    title: Option<String>,
    /// Send even if window is active
    #[arg(short, long)]
    force: bool,
}

#[derive(Debug, Clone, Subcommand)]
pub enum TmuxCmd {
    /// Move current window after specified position (0 = move to first)
    MoveAfter {
        /// Window position to move after (0 moves to first position)
        position: u32,
    },
    /// Clear 🔔 prefix from current window name
    ClearBell,
    /// Print "unset SSH_CONNECTION SSH_CLIENT" if the tmux client is local (no sshd in parent chain)
    SyncSsh {
        /// Unset SSH vars in tmux session env and all idle panes at once
        #[arg(short, long)]
        all: bool,
    },
    /// Send terminal notification (bell, macos, or both)
    Notify {
        /// Notification message (used with macos)
        message: Option<String>,
        /// Notification types (comma-separated: bell, macos)
        #[arg(short = 'T', long = "type", value_delimiter = ',')]
        kind: Vec<NotifyKind>,
        /// Notification title (default: "{window_name} Notification")
        #[arg(short, long)]
        title: Option<String>,
        /// Send even if window is active
        #[arg(short, long)]
        force: bool,
    },
    /// Execute a quick action by name (used by fzf quick actions menu)
    Action {
        /// Action name (e.g., "New Tab", "Close Pane")
        #[arg(trailing_var_arg = true)]
        name: Vec<String>,
    },
    /// Name a running Codex pane from its current thread
    NameCodexPane {
        /// Tmux pane target, defaults to the active pane
        #[arg(long)]
        target_pane: Option<String>,
    },
    /// Set a pane name from a Codex turn-complete notification on stdin
    SyncCodexPaneName {
        /// Tmux pane target, defaults to TMUX_PANE
        #[arg(long)]
        target_pane: Option<String>,
    },
    /// Rename a pane and the matching Codex session when possible
    RenamePane {
        /// Tmux pane target, defaults to the active pane
        #[arg(long)]
        target_pane: Option<String>,
        /// New pane name
        #[arg(trailing_var_arg = true, required = true)]
        name: Vec<String>,
    },
    /// Fzf picker menus (window, session, action, pane)
    Picker {
        #[command(subcommand)]
        kind: PickerKind,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum PickerKind {
    /// Fzf window switcher
    #[command(alias = "w")]
    Window,
    /// Fzf session switcher
    #[command(alias = "s")]
    Session,
    /// Fzf quick actions menu
    #[command(alias = "a")]
    Action,
    /// Fzf pane switcher
    #[command(alias = "p")]
    Pane,
}

pub fn run_with_flags(sh: &Shell, flags: Tmux) -> Result<()> {
    match flags.subcommand {
        TmuxCmd::MoveAfter { position } => move_after(sh, position),
        TmuxCmd::ClearBell => clear_bell(sh),
        TmuxCmd::SyncSsh { all } => sync_ssh(sh, all),
        TmuxCmd::Notify {
            kind,
            message,
            title,
            force,
        } => notify(sh, &kind, message.as_deref(), title.as_deref(), force),
        TmuxCmd::Action { name } => action(sh, &name.join(" ")),
        TmuxCmd::NameCodexPane { target_pane } => name_codex_pane(sh, target_pane.as_deref()),
        TmuxCmd::SyncCodexPaneName { target_pane } => {
            sync_codex_pane_name(sh, target_pane.as_deref())
        }
        TmuxCmd::RenamePane { target_pane, name } => {
            rename_pane(sh, target_pane.as_deref(), &name.join(" "))
        }
        TmuxCmd::Picker { kind } => match kind {
            PickerKind::Window => window_picker(sh),
            PickerKind::Session => session_picker(sh),
            PickerKind::Action => action_picker(sh),
            PickerKind::Pane => pane_picker(sh),
        },
    }
}

pub fn notify_run(sh: &Shell, args: &[OsString]) -> Result<()> {
    let flags = NotifyArgs::parse_from(args);
    notify(
        sh,
        &flags.kind,
        flags.message.as_deref(),
        flags.title.as_deref(),
        flags.force,
    )
}

fn move_after(sh: &Shell, position: u32) -> Result<()> {
    if position == 0 {
        cmd!(sh, "tmux move-window -b -t 1").quiet().run()?;
    } else {
        let target = position.to_string();
        cmd!(sh, "tmux move-window -a -t {target}").quiet().run()?;
    }
    Ok(())
}

fn session_picker(sh: &Shell) -> Result<()> {
    let client_fmt = format!("#{{client_session}}{FIELD_SEP}#{{client_last_session}}");
    let client_context = cmd!(sh, "tmux display-message -p {client_fmt}")
        .quiet()
        .read()
        .unwrap_or_default();
    let (current_session, last_session) = parse_client_session_context(&client_context);

    let fmt = format!("#S{FIELD_SEP}#{{session_last_attached}}{FIELD_SEP}#{{session_activity}}");
    let sessions = cmd!(sh, "tmux list-sessions -F {fmt}").quiet().read()?;
    let mut sessions = sessions
        .lines()
        .filter_map(SessionEntry::parse)
        .collect::<Vec<_>>();
    order_sessions(
        &mut sessions,
        current_session.as_deref(),
        last_session.as_deref(),
    );
    let selection = run_fzf(sh, "Session > ", &render_lines(&sessions))?;
    let session = selection.trim();
    if !session.is_empty() {
        cmd!(sh, "tmux switch-client -t {session}").quiet().run()?;
    }
    Ok(())
}

fn window_picker(sh: &Shell) -> Result<()> {
    let stack_fmt = "#{session_stack}";
    let stack = cmd!(sh, "tmux display-message -p {stack_fmt}")
        .quiet()
        .read()
        .unwrap_or_default();
    let fmt =
        format!("#I{FIELD_SEP}#W{FIELD_SEP}#{{window_active}}{FIELD_SEP}#{{window_last_flag}}");
    let windows = cmd!(sh, "tmux list-windows -F {fmt}").quiet().read()?;
    let mut windows = windows
        .lines()
        .filter_map(WindowEntry::parse)
        .collect::<Vec<_>>();
    order_windows(&mut windows, &parse_index_list(&stack));
    let selection = run_fzf(sh, "Window > ", &render_lines(&windows))?;
    if let Some(index) = selection.split(':').next() {
        let index = index.trim();
        cmd!(sh, "tmux select-window -t {index}").quiet().run()?;
    }
    Ok(())
}

fn pane_picker(sh: &Shell) -> Result<()> {
    let fmt = format!(
        "#P{FIELD_SEP}#{{?#{{@pane_name}},#{{@pane_name}} - ,}}#{{pane_current_command}} (#{{pane_current_path}}){FIELD_SEP}#{{pane_active}}{FIELD_SEP}#{{pane_last}}"
    );
    let panes = cmd!(sh, "tmux list-panes -F {fmt}").quiet().read()?;
    let mut panes = panes
        .lines()
        .filter_map(PaneEntry::parse)
        .collect::<Vec<_>>();
    order_panes(&mut panes);
    let selection = run_fzf(sh, "Pane > ", &render_lines(&panes))?;
    if let Some(index) = selection.split(':').next() {
        let index = index.trim();
        cmd!(sh, "tmux select-pane -t {index}").quiet().run()?;
    }
    Ok(())
}

const FIELD_SEP: char = '\u{1f}';

trait PickerEntry {
    fn display_line(&self) -> String;
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SessionEntry {
    name: String,
    last_attached: u64,
    activity: u64,
}

impl SessionEntry {
    fn parse(line: &str) -> Option<Self> {
        let mut parts = line.split(FIELD_SEP);
        let name = parts.next()?.to_string();
        let last_attached = parse_num(parts.next()?);
        let activity = parse_num(parts.next()?);
        Some(Self {
            name,
            last_attached,
            activity,
        })
    }

    fn recency_key(&self) -> (bool, u64, u64) {
        (self.last_attached != 0, self.last_attached, self.activity)
    }
}

impl PickerEntry for SessionEntry {
    fn display_line(&self) -> String {
        self.name.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowEntry {
    index: u32,
    name: String,
    active: bool,
    last: bool,
}

impl WindowEntry {
    fn parse(line: &str) -> Option<Self> {
        let mut parts = line.split(FIELD_SEP);
        let index = parse_num(parts.next()?);
        let name = parts.next()?.to_string();
        let active = parse_flag(parts.next()?);
        let last = parse_flag(parts.next()?);
        Some(Self {
            index,
            name,
            active,
            last,
        })
    }
}

impl PickerEntry for WindowEntry {
    fn display_line(&self) -> String {
        format!("{}: {}", self.index, self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PaneEntry {
    index: u32,
    label: String,
    active: bool,
    last: bool,
}

impl PaneEntry {
    fn parse(line: &str) -> Option<Self> {
        let mut parts = line.split(FIELD_SEP);
        let index = parse_num(parts.next()?);
        let label = parts.next()?.to_string();
        let active = parse_flag(parts.next()?);
        let last = parse_flag(parts.next()?);
        Some(Self {
            index,
            label,
            active,
            last,
        })
    }
}

impl PickerEntry for PaneEntry {
    fn display_line(&self) -> String {
        format!("{}: {}", self.index, self.label)
    }
}

fn run_fzf(sh: &Shell, prompt: &str, input: &str) -> Result<String> {
    Ok(cmd!(sh, "fzf --prompt {prompt} --height=100% --no-sort")
        .quiet()
        .stdin(input.as_bytes())
        .read()?)
}

fn render_lines<T: PickerEntry>(entries: &[T]) -> String {
    entries
        .iter()
        .map(PickerEntry::display_line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn order_sessions(
    entries: &mut [SessionEntry],
    current_session: Option<&str>,
    last_session: Option<&str>,
) {
    entries.sort_by(|a, b| {
        demote_active(
            is_named_session(a, current_session),
            is_named_session(b, current_session),
        )
        .then_with(|| compare_session_rank(a, b, last_session))
        .then_with(|| b.recency_key().cmp(&a.recency_key()))
        .then_with(|| a.name.cmp(&b.name))
    });
}

fn is_named_session(entry: &SessionEntry, session: Option<&str>) -> bool {
    session.is_some_and(|session| entry.name == session)
}

fn compare_session_rank(
    left: &SessionEntry,
    right: &SessionEntry,
    last_session: Option<&str>,
) -> Ordering {
    match (
        is_named_session(left, last_session),
        is_named_session(right, last_session),
    ) {
        (true, true) | (false, false) => Ordering::Equal,
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
    }
}

fn order_windows(entries: &mut [WindowEntry], session_stack: &[u32]) {
    let stack_rank = session_stack
        .iter()
        .enumerate()
        .map(|(rank, index)| (*index, rank))
        .collect::<HashMap<_, _>>();

    entries.sort_by(|a, b| {
        demote_active(a.active, b.active)
            .then_with(|| compare_window_rank(a, b, &stack_rank))
            .then_with(|| b.last.cmp(&a.last))
            .then_with(|| a.index.cmp(&b.index))
    });
}

fn order_panes(entries: &mut [PaneEntry]) {
    entries.sort_by(|a, b| {
        demote_active(a.active, b.active)
            .then_with(|| b.last.cmp(&a.last))
            .then_with(|| a.index.cmp(&b.index))
    });
}

fn compare_window_rank(
    left: &WindowEntry,
    right: &WindowEntry,
    stack_rank: &HashMap<u32, usize>,
) -> Ordering {
    match (stack_rank.get(&left.index), stack_rank.get(&right.index)) {
        (Some(left_rank), Some(right_rank)) => left_rank.cmp(right_rank),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn demote_active(left: bool, right: bool) -> Ordering {
    left.cmp(&right)
}

fn parse_index_list(value: &str) -> Vec<u32> {
    value
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse().ok())
        .collect()
}

fn parse_client_session_context(value: &str) -> (Option<String>, Option<String>) {
    let mut parts = value.trim_end().split(FIELD_SEP);
    let current = non_empty_string(parts.next());
    let last = non_empty_string(parts.next());
    (current, last)
}

fn non_empty_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn parse_num<T: std::str::FromStr + Default>(value: &str) -> T {
    value.trim().parse().unwrap_or_default()
}

fn parse_flag(value: &str) -> bool {
    value.trim() == "1"
}

const ACTIONS: &[&str] = &[
    "New Tab",
    "Close Pane",
    "Zoom Pane",
    "Split Right",
    "Split Down",
    "Next Tab",
    "Prev Tab",
    "Swap Down",
    "Swap Up",
    "Rename Tab",
    "Rename Session",
    "Rename Pane",
    "Name Pane with Codex",
    "Toggle Pane Names",
    "Scroll Back",
    "Move Tab to Session",
];

fn action_picker(sh: &Shell) -> Result<()> {
    let menu = ACTIONS.join("\n");
    let selection = run_fzf(sh, "Action > ", &menu)?;
    action(sh, &selection)
}

fn action(sh: &Shell, name: &str) -> Result<()> {
    let pane_path = || -> String {
        let fmt = "#{pane_current_path}";
        cmd!(sh, "tmux display-message -p {fmt}")
            .quiet()
            .read()
            .unwrap_or_default()
    };

    match name.trim() {
        "New Tab" => {
            let path = pane_path();
            cmd!(sh, "tmux new-window -c {path}").quiet().run()?;
        }
        "Close Pane" => {
            cmd!(sh, "tmux kill-pane").quiet().run()?;
        }
        "Zoom Pane" => {
            cmd!(sh, "tmux resize-pane -Z").quiet().run()?;
        }
        "Split Right" => {
            let path = pane_path();
            cmd!(sh, "tmux split-window -h -c {path}").quiet().run()?;
        }
        "Split Down" => {
            let path = pane_path();
            cmd!(sh, "tmux split-window -c {path}").quiet().run()?;
        }
        "Next Tab" => {
            cmd!(sh, "tmux next-window").quiet().run()?;
        }
        "Prev Tab" => {
            cmd!(sh, "tmux previous-window").quiet().run()?;
        }
        "Swap Down" => {
            cmd!(sh, "tmux swap-pane -D").quiet().run()?;
        }
        "Swap Up" => {
            cmd!(sh, "tmux swap-pane -U").quiet().run()?;
        }
        "Rename Tab" => {
            prompt_and_run(sh, "Window name > ", "rename-window")?;
        }
        "Rename Session" => {
            prompt_and_run(sh, "Session name > ", "rename-session")?;
        }
        "Rename Pane" => {
            if let Some(name) = prompt_text("Pane name > ")? {
                rename_pane(sh, None, &name)?;
            }
        }
        "Name Pane with Codex" => {
            let pane_id_format = "#{pane_id}";
            let pane_id = cmd!(sh, "tmux display-message -p {pane_id_format}")
                .quiet()
                .read()?
                .trim()
                .to_owned();
            let command = format!("cmd tmux name-codex-pane --target-pane {pane_id}");
            cmd!(sh, "tmux run-shell -b {command}").quiet().run()?;
        }
        "Toggle Pane Names" => {
            let status = cmd!(sh, "tmux show-option -gv pane-border-status")
                .quiet()
                .read()
                .unwrap_or_default();
            let value = if status.trim() == "off" { "top" } else { "off" };
            cmd!(sh, "tmux set -g pane-border-status {value}")
                .quiet()
                .run()?;
        }
        "Scroll Back" => {
            cmd!(sh, "tmux copy-mode").quiet().run()?;
        }
        "Move Tab to Session" => {
            let session_fmt = "#S";
            let current = cmd!(sh, "tmux display-message -p {session_fmt}")
                .quiet()
                .read()
                .unwrap_or_default();
            let current = current.trim().to_string();

            let src_fmt = "#{session_name}:#{window_index}";
            let source = cmd!(sh, "tmux display-message -p {src_fmt}")
                .quiet()
                .read()
                .unwrap_or_default();
            let source = source.trim().to_string();

            let all_sessions = cmd!(sh, "tmux list-sessions -F {session_fmt}")
                .quiet()
                .read()?;
            let other_sessions: String = all_sessions
                .lines()
                .filter(|s| s.trim() != current)
                .collect::<Vec<_>>()
                .join("\n");

            if other_sessions.is_empty() {
                eprintln!("No other sessions to move to");
                return Ok(());
            }

            let selection = run_fzf(sh, "Move to > ", &other_sessions)?;

            let target = selection.trim();
            if !target.is_empty() {
                let dst = format!("{target}:");
                cmd!(sh, "tmux move-window -s {source} -t {dst}").run()?;
            }
        }
        other => {
            eprintln!("Unknown action: {other}");
        }
    }
    Ok(())
}

const NAME_MODEL: &str = "gpt-5.3-codex-spark";
const THREAD_TITLE_MAX_CHARS: usize = 36;
const THREAD_TITLE_PROMPT_MAX_BYTES: usize = 960;
const THREAD_TITLE_RECENT_MESSAGES: usize = 8;
const SESSION_THREAD_WAIT_TIMEOUT: Duration = Duration::from_secs(3);
const SESSION_THREAD_POLL_INTERVAL: Duration = Duration::from_millis(50);
const TURN_COMPLETE_NOTIFICATION: &str = "agent-turn-complete";

#[derive(Debug, Deserialize)]
struct CodexNotificationPayload {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "thread-id")]
    thread_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CodexThreadId(String);

impl CodexThreadId {
    fn parse(value: String) -> Result<Self> {
        if value.is_empty() || value.chars().any(char::is_whitespace) {
            return Err(eyre!("Codex notification has an invalid thread id"));
        }

        Ok(Self(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CodexThreadName(String);

impl CodexThreadName {
    fn parse(value: &str) -> Option<Self> {
        let without_controls = value
            .chars()
            .map(|character| {
                if character.is_control() {
                    ' '
                } else {
                    character
                }
            })
            .collect::<String>();
        let normalized = without_controls
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        (!normalized.is_empty()).then_some(Self(normalized))
    }
}

#[derive(Debug, Deserialize)]
struct SessionIndexEntry {
    id: String,
    thread_name: Option<String>,
}

#[derive(Debug, Clone)]
struct PaneTarget {
    id: String,
    tty: String,
    cwd: PathBuf,
    current_command: String,
    current_name: String,
    visible_text: String,
}

#[derive(Debug, Clone)]
struct PaneProcess {
    pid: u32,
    ppid: u32,
    command: String,
    args: String,
}

#[derive(Debug, Clone)]
struct ActiveCodexSession {
    launch_home: PathBuf,
    socket_path: PathBuf,
    thread_id: String,
    rollout_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TitleMessageRole {
    User,
    Assistant,
}

impl TitleMessageRole {
    fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TitleMessage {
    role: TitleMessageRole,
    text: String,
}

#[derive(Debug, Default)]
struct NamingContext {
    messages: Vec<TitleMessage>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GeneratedPaneTitle {
    title: String,
}

fn name_codex_pane(sh: &Shell, target_pane: Option<&str>) -> Result<()> {
    let pane = read_pane_target(sh, target_pane)?;
    let previous_name = pane.current_name.clone();
    set_tmux_pane_name(sh, &pane.id, "renaming...")?;

    if let Err(err) = name_codex_pane_after_progress(sh, &pane) {
        set_tmux_pane_name(sh, &pane.id, &previous_name).ok();
        return Err(err);
    }

    Ok(())
}

fn sync_codex_pane_name(sh: &Shell, target_pane: Option<&str>) -> Result<()> {
    let mut payload = String::new();
    std::io::stdin()
        .read_to_string(&mut payload)
        .wrap_err("Failed to read the Codex notification")?;
    let Some(thread_id) = thread_id_from_notification(&payload)? else {
        return Ok(());
    };
    let pane_id = target_pane
        .map(str::to_owned)
        .or_else(|| std::env::var("TMUX_PANE").ok())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| eyre!("Tmux pane target is not set"))?;
    let session_index = codex_session_index_path()?;
    let Some(name) = latest_thread_name(&session_index, &thread_id)? else {
        return Ok(());
    };

    // use the notification thread id because cwd can match several live panes
    set_tmux_pane_name(sh, &pane_id, &name.0)
}

fn thread_id_from_notification(payload: &str) -> Result<Option<CodexThreadId>> {
    let notification = serde_json::from_str::<CodexNotificationPayload>(payload)
        .wrap_err("Failed to parse the Codex notification")?;
    if notification.kind != TURN_COMPLETE_NOTIFICATION {
        return Ok(None);
    }

    let thread_id = notification
        .thread_id
        .ok_or_else(|| eyre!("Codex turn-complete notification has no thread id"))?;

    Ok(Some(CodexThreadId::parse(thread_id)?))
}

fn codex_session_index_path() -> Result<PathBuf> {
    let home = home_dir()?;
    let codex_home = std::env::var_os("CODEX_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);

    Ok(resolve_session_index_path(codex_home.as_deref(), &home))
}

fn resolve_session_index_path(codex_home: Option<&Path>, home: &Path) -> PathBuf {
    let fallback = home.join(".codex").join("session_index.jsonl");
    let Some(codex_home) = codex_home else {
        return fallback;
    };
    let configured = codex_home.join("session_index.jsonl");

    if configured.is_file() {
        configured
    } else {
        fallback
    }
}

fn latest_thread_name(path: &Path, thread_id: &CodexThreadId) -> Result<Option<CodexThreadName>> {
    let file = File::open(path)
        .wrap_err_with(|| format!("Failed to open Codex session index {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    let mut line_number = 0;
    let mut latest = None;

    loop {
        line.clear();
        let bytes_read = reader
            .read_line(&mut line)
            .wrap_err_with(|| format!("Failed to read Codex session index {}", path.display()))?;
        if bytes_read == 0 {
            break;
        }
        line_number += 1;
        let is_complete_line = line.ends_with('\n');
        let serialized = line.trim_end_matches(['\r', '\n']);
        let entry = match serde_json::from_str::<SessionIndexEntry>(serialized) {
            Ok(entry) => entry,
            Err(_) if !is_complete_line => break,
            Err(error) => {
                return Err(error).wrap_err_with(|| {
                    format!(
                        "Failed to parse Codex session index {} at line {line_number}",
                        path.display()
                    )
                });
            }
        };
        if entry.id != thread_id.0 {
            continue;
        }
        if let Some(name) = entry
            .thread_name
            .as_deref()
            .and_then(CodexThreadName::parse)
        {
            latest = Some(name);
        }
    }

    Ok(latest)
}

fn name_codex_pane_after_progress(sh: &Shell, pane: &PaneTarget) -> Result<()> {
    let processes = processes_on_tty(&pane.tty)?;
    if !pane_runs_codex(pane, &processes) {
        return Err(eyre!("Target pane is not running Codex"));
    }

    let session = resolve_active_codex_session(pane, &processes)?;
    let context = build_naming_context(pane, Some(&session));
    let prompt = build_naming_prompt(&context);
    let raw_name = run_codex_name_model(&pane.cwd, Some(&session.launch_home), &prompt)
        .wrap_err("Failed to generate pane name")?;
    let name = parse_generated_title(&raw_name)
        .or_else(|| fallback_pane_name(pane))
        .ok_or_else(|| eyre!("Generated pane name was empty"))?;

    // Pane label is the primary result. Codex session rename is best-effort so a
    // missing rollout / app-server rejection still exits 0 and keeps the name.
    // Do not surface session failures: Alt+r is fire-and-forget via run-shell -b,
    // and a non-zero exit makes tmux flash "'cmd ...' returned 1".
    let _ = apply_synced_codex_name(
        &name,
        |pane_name| set_tmux_pane_name(sh, &pane.id, pane_name),
        || set_codex_thread_name_for_session(&session, &name),
    );

    Ok(())
}

fn rename_pane(sh: &Shell, target_pane: Option<&str>, name: &str) -> Result<()> {
    let name = name.trim();
    let pane = read_pane_target(sh, target_pane)?;
    let processes = processes_on_tty(&pane.tty)?;
    if !pane_runs_codex(&pane, &processes) {
        if name.is_empty() {
            clear_tmux_pane_name(sh, &pane.id)?;
        } else {
            set_tmux_pane_name(sh, &pane.id, name)?;
        }
        return Ok(());
    }
    if name.is_empty() {
        return Err(eyre!("Codex session names cannot be empty"));
    }

    let session = resolve_active_codex_session(&pane, &processes)?;
    if let Err(err) = apply_synced_codex_name(
        name,
        |pane_name| set_tmux_pane_name(sh, &pane.id, pane_name),
        || set_codex_thread_name_for_session(&session, name),
    ) {
        report_name_result(
            sh,
            &format!("Pane renamed; Codex session rename failed: {err}"),
        );
    }

    Ok(())
}

fn set_codex_thread_name_for_session(session: &ActiveCodexSession, name: &str) -> Result<()> {
    set_thread_name(&session.socket_path, &session.thread_id, name)
}

/// Apply a pane name and optionally keep the Codex session name in sync.
///
/// Session rename failures are returned after the pane name is applied so
/// callers can keep the pane label and report softly instead of rolling back.
fn apply_synced_codex_name(
    name: &str,
    mut set_pane_name: impl FnMut(&str) -> Result<()>,
    set_session_name: impl FnOnce() -> Result<()>,
) -> Result<()> {
    set_pane_name(name)?;
    set_session_name()
}

fn report_name_result(sh: &Shell, message: &str) {
    if std::env::var_os("TMUX").is_some() {
        cmd!(sh, "tmux display-message {message}")
            .quiet()
            .run()
            .ok();
        return;
    }

    println!("{message}");
}

fn read_pane_target(sh: &Shell, target_pane: Option<&str>) -> Result<PaneTarget> {
    let format = format!(
        "#{{pane_id}}{FIELD_SEP}#{{pane_tty}}{FIELD_SEP}#{{pane_current_path}}{FIELD_SEP}#{{pane_current_command}}{FIELD_SEP}#{{@pane_name}}"
    );
    let output = if let Some(target) = target_pane {
        cmd!(sh, "tmux display-message -p -t {target} {format}")
            .quiet()
            .read()?
    } else {
        cmd!(sh, "tmux display-message -p {format}")
            .quiet()
            .read()?
    };
    let mut parts = output.trim_end().split(FIELD_SEP);
    let id = parts
        .next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| eyre!("Failed to read tmux pane id"))?
        .to_owned();
    let tty = parts
        .next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| eyre!("Failed to read tmux pane tty"))?
        .to_owned();
    let cwd = parts
        .next()
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| eyre!("Failed to read tmux pane cwd"))?;
    let current_command = parts.next().unwrap_or_default().to_owned();
    let current_name = parts.next().unwrap_or_default().to_owned();
    let visible_text = capture_visible_pane_text(sh, &id);

    Ok(PaneTarget {
        id,
        tty,
        cwd,
        current_command,
        current_name,
        visible_text,
    })
}

fn capture_visible_pane_text(sh: &Shell, pane_id: &str) -> String {
    cmd!(sh, "tmux capture-pane -p -t {pane_id}")
        .quiet()
        .read()
        .unwrap_or_default()
}

fn processes_on_tty(tty: &str) -> Result<Vec<PaneProcess>> {
    let tty = tty.strip_prefix("/dev/").unwrap_or(tty);
    let output = Command::new("ps")
        .args([
            "-o", "pid=", "-o", "ppid=", "-o", "comm=", "-o", "args=", "-t", tty,
        ])
        .output()?;
    if !output.status.success() {
        return Ok(Vec::new());
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_pane_process)
        .collect())
}

fn parse_pane_process(line: &str) -> Option<PaneProcess> {
    let mut parts = line.split_whitespace();
    let pid = parts.next()?.parse().ok()?;
    let ppid = parts.next()?.parse().ok()?;
    let command = parts.next()?.to_owned();
    let args = parts.collect::<Vec<_>>().join(" ");

    Some(PaneProcess {
        pid,
        ppid,
        command,
        args,
    })
}

fn pane_runs_codex(pane: &PaneTarget, processes: &[PaneProcess]) -> bool {
    command_looks_like_codex(&pane.current_command)
        || processes.iter().any(|process| {
            command_looks_like_codex(&process.command)
                || process
                    .args
                    .split_whitespace()
                    .any(command_looks_like_codex)
        })
}

fn command_looks_like_codex(value: &str) -> bool {
    Path::new(value)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "codex")
}

fn resolve_active_codex_session(
    pane: &PaneTarget,
    processes: &[PaneProcess],
) -> Result<ActiveCodexSession> {
    let codex_pids = codex_process_family(processes);
    let deadline = Instant::now() + SESSION_THREAD_WAIT_TIMEOUT;
    loop {
        if let Some(session) = resolve_active_codex_session_once(pane, &codex_pids)? {
            return Ok(session);
        }
        if Instant::now() >= deadline {
            return Err(eyre!(
                "Codex has not reported its current session yet; send a prompt and try again"
            ));
        }
        std::thread::sleep(SESSION_THREAD_POLL_INTERVAL);
    }
}

fn resolve_active_codex_session_once(
    pane: &PaneTarget,
    codex_pids: &HashSet<u32>,
) -> Result<Option<ActiveCodexSession>> {
    let markers = active_codex_session_markers()?;
    let marker = markers
        .iter()
        .position(|marker| marker.pane_id.as_deref() == Some(pane.id.as_str()))
        .map(|index| markers[index].clone())
        .or_else(|| {
            markers.into_iter().find(|marker| {
                marker
                    .session_pid
                    .is_some_and(|pid| codex_pids.contains(&pid))
            })
        })
        .ok_or_else(|| eyre!("Codex session marker not found; restart Codex from cmd codex"))?;
    let socket_path = match marker.control {
        SessionControl::Local { socket_path } => socket_path,
        SessionControl::External => {
            return Err(eyre!("External Codex sessions cannot be renamed locally"));
        }
        SessionControl::Embedded | SessionControl::Legacy => {
            return Err(eyre!(
                "This Codex session has no live control connection; restart Codex"
            ));
        }
    };
    // Prefer marker thread when it already has a rollout. ThreadStarted can land
    // with a null path; refresh from the app server so name/set and context work.
    let thread = match marker.current_thread {
        Some(thread) if thread.rollout_path.is_some() => Some(thread),
        Some(thread) => match current_loaded_thread(&socket_path)? {
            Some(loaded) => Some(loaded),
            None => Some(thread),
        },
        None => current_loaded_thread(&socket_path)?,
    };
    let Some(thread) = thread else {
        return Ok(None);
    };

    Ok(Some(ActiveCodexSession {
        launch_home: marker.launch_home,
        socket_path,
        thread_id: thread.id,
        rollout_path: thread.rollout_path,
    }))
}

fn codex_process_family(processes: &[PaneProcess]) -> HashSet<u32> {
    let mut pids = processes
        .iter()
        .filter(|process| {
            command_looks_like_codex(&process.command)
                || process
                    .args
                    .split_whitespace()
                    .any(command_looks_like_codex)
        })
        .map(|process| process.pid)
        .collect::<HashSet<_>>();

    let mut changed = true;
    while changed {
        changed = false;
        for process in processes {
            if pids.contains(&process.ppid) && pids.insert(process.pid) {
                changed = true;
            }
        }
    }

    pids
}

fn active_codex_session_markers() -> Result<Vec<SessionMarker>> {
    let profiles = home_dir()?.join(".codex").join("profiles");
    if !profiles.exists() {
        return Ok(Vec::new());
    }

    let mut markers = Vec::new();
    for profile in fs::read_dir(&profiles)? {
        let profile = profile?;
        let markers_dir = profile.path().join(".session-markers");
        if !markers_dir.exists() {
            continue;
        }

        for entry in fs::read_dir(markers_dir)? {
            let entry = entry?;
            let marker = match fs::read(entry.path())
                .ok()
                .and_then(|bytes| serde_json::from_slice::<SessionMarker>(&bytes).ok())
            {
                Some(marker) if process_exists(marker.owner_pid) => marker,
                _ => continue,
            };
            markers.push(marker);
        }
    }

    Ok(markers)
}

fn process_exists(pid: u32) -> bool {
    Command::new("ps")
        .args(["-p", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn build_naming_context(pane: &PaneTarget, session: Option<&ActiveCodexSession>) -> NamingContext {
    let mut messages = Vec::new();

    if let Some(rollout_path) = session.and_then(|session| session.rollout_path.as_deref()) {
        messages = title_messages_from_rollout(rollout_path);
    }

    if messages.is_empty() && !pane.visible_text.trim().is_empty() {
        messages.push(TitleMessage {
            role: TitleMessageRole::User,
            text: pane.visible_text.trim().to_owned(),
        });
    }

    NamingContext { messages }
}

fn title_messages_from_rollout(path: &Path) -> Vec<TitleMessage> {
    let mut messages = read_rollout_lines(path, |value| {
        value
            .get("type")
            .and_then(serde_json::Value::as_str)
            .filter(|kind| *kind == "response_item")?;
        let payload = value.get("payload")?;
        payload
            .get("type")
            .and_then(serde_json::Value::as_str)
            .filter(|kind| *kind == "message")?;

        let role = match payload.get("role").and_then(serde_json::Value::as_str)? {
            "user" => TitleMessageRole::User,
            "assistant"
                if payload.get("phase").and_then(serde_json::Value::as_str)
                    != Some("commentary") =>
            {
                TitleMessageRole::Assistant
            }
            _ => return None,
        };
        let text = match role {
            TitleMessageRole::User => human_user_request_text(payload),
            TitleMessageRole::Assistant => assistant_message_text(payload),
        }?;

        Some(TitleMessage { role, text })
    });
    let keep_from = messages.len().saturating_sub(THREAD_TITLE_RECENT_MESSAGES);
    messages.drain(..keep_from);
    messages
}

fn human_user_request_text(payload: &serde_json::Value) -> Option<String> {
    message_content_text(payload, /*filter_injected_context*/ true)
}

fn assistant_message_text(payload: &serde_json::Value) -> Option<String> {
    message_content_text(payload, /*filter_injected_context*/ false)
}

fn message_content_text(
    payload: &serde_json::Value,
    filter_injected_context: bool,
) -> Option<String> {
    let text = payload
        .get("content")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            item.get("text")
                .or_else(|| item.get("input_text"))
                .or_else(|| item.get("output_text"))
        })
        .filter_map(serde_json::Value::as_str)
        .filter(|text| !filter_injected_context || !is_injected_context_text(text))
        .collect::<Vec<_>>()
        .join("\n");

    (!text.trim().is_empty()).then(|| text.trim().to_owned())
}

fn is_injected_context_text(text: &str) -> bool {
    let text = text.trim_start();
    if text.starts_with("# AGENTS.md instructions for ") && text.contains("\n<INSTRUCTIONS>") {
        return true;
    }

    [
        "<environment_context>",
        "</environment_context>",
        "<skill>",
        "</skill>",
        "## Memory",
        "<permissions instructions>",
        "</permissions instructions>",
        "<collaboration_mode>",
        "</collaboration_mode>",
        "<skills_instructions>",
        "</skills_instructions>",
        "<plugins_instructions>",
        "</plugins_instructions>",
        "</INSTRUCTIONS>",
    ]
    .iter()
    .any(|prefix| text.starts_with(prefix))
}

fn read_rollout_lines<T>(
    path: &Path,
    mut parse: impl FnMut(&serde_json::Value) -> Option<T>,
) -> Vec<T> {
    let Ok(contents) = fs::read_to_string(path) else {
        return Vec::new();
    };

    contents
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|value| parse(&value))
        .collect()
}

fn build_naming_prompt(context: &NamingContext) -> String {
    let conversation = bounded_conversation_markup(&context.messages);
    let instructions = thread_title_instructions();
    let prefix = format!(
        "{instructions}\n\
Prioritize the current task and latest substantive user request.\n\n\
Recent conversation messages:\n"
    );
    let remaining_bytes = THREAD_TITLE_PROMPT_MAX_BYTES.saturating_sub(prefix.len());
    let conversation = trailing_complete_chars(&conversation, remaining_bytes);

    format!("{prefix}{conversation}")
}

fn thread_title_instructions() -> String {
    format!(
        "Generate a concise, single-line task title of at most \
{THREAD_TITLE_MAX_CHARS} characters and under five words where possible. \
Start with an imperative verb. Capitalize only the first word unless the \
user's language, proper nouns, acronyms, or code terms require otherwise. \
Preserve ticket references exactly. Write in the user's language. \
Do not use quotes, markdown, or trailing punctuation. \
Do not answer the request."
    )
}

fn bounded_conversation_markup(messages: &[TitleMessage]) -> String {
    let escaped = messages
        .iter()
        .map(|message| {
            let text = message
                .text
                .trim()
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");

            (message.role.as_str(), text)
        })
        .collect::<Vec<_>>();
    if escaped.is_empty() {
        return "<conversation></conversation>".to_owned();
    }

    let empty_conversation = "<conversation>\n\n</conversation>";
    let available_bytes = THREAD_TITLE_PROMPT_MAX_BYTES
        .saturating_sub(thread_title_instructions().len())
        .saturating_sub("\nPrioritize the current task and latest substantive user request.\n\nRecent conversation messages:\n".len());
    let markup_bytes = empty_conversation.len()
        + escaped.len().saturating_sub(1)
        + escaped
            .iter()
            .map(|(role, _)| "<message role=\"\"></message>".len() + role.len())
            .sum::<usize>();
    let message_bytes = available_bytes.saturating_sub(markup_bytes) / escaped.len();
    let should_truncate = escaped.iter().map(|(_, text)| text.len()).sum::<usize>()
        > available_bytes.saturating_sub(markup_bytes);
    let messages = escaped
        .into_iter()
        .map(|(role, text)| {
            let text = if should_truncate {
                truncate_complete_chars_and_entities(&text, message_bytes)
            } else {
                text
            };

            format!("<message role=\"{role}\">{text}</message>")
        })
        .collect::<Vec<_>>();

    format!("<conversation>\n{}\n</conversation>", messages.join("\n"))
}

fn truncate_complete_chars_and_entities(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }

    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    if let Some(entity_start) = value[..end].rfind('&') {
        if !value[entity_start..end].contains(';') {
            end = entity_start;
        }
    }

    value[..end].to_owned()
}

fn trailing_complete_chars(value: &str, max_bytes: usize) -> &str {
    let mut start = value.len().saturating_sub(max_bytes);
    while !value.is_char_boundary(start) {
        start += 1;
    }

    &value[start..]
}

fn run_codex_name_model(cwd: &Path, launch_home: Option<&Path>, prompt: &str) -> Result<String> {
    let output_file = TempFileBuilder::new()
        .prefix("codex-pane-name")
        .tempfile()?;
    let output_path = output_file.path().to_path_buf();
    let mut schema_file = TempFileBuilder::new()
        .prefix("codex-pane-name-schema")
        .suffix(".json")
        .tempfile()?;
    serde_json::to_writer(schema_file.as_file_mut(), &thread_title_output_schema())?;
    schema_file.as_file_mut().flush()?;
    let codex_home = launch_home
        .map(Path::to_path_buf)
        .unwrap_or(home_dir()?.join(".codex"));

    let mut child = Command::new("codex")
        .args([
            "exec",
            "--ephemeral",
            "--skip-git-repo-check",
            "--ignore-user-config",
            "--ignore-rules",
            "--sandbox",
            "read-only",
            "--model",
            NAME_MODEL,
            "--config",
            "model_reasoning_effort=\"xhigh\"",
            "--cd",
        ])
        .arg(cwd)
        .arg("--output-schema")
        .arg(schema_file.path())
        .args(["--output-last-message"])
        .arg(&output_path)
        .arg("-")
        .env("CODEX_HOME", codex_home)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| eyre!("Failed to open codex stdin"))?;
    stdin.write_all(prompt.as_bytes())?;
    drop(stdin);
    let status = child.wait()?;
    if !status.success() {
        return Err(eyre!("codex exec exited with status {status}"));
    }

    Ok(fs::read_to_string(output_path).unwrap_or_default())
}

fn thread_title_output_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "title": {
                "type": "string",
                "minLength": 1,
                "maxLength": THREAD_TITLE_MAX_CHARS,
            },
        },
        "required": ["title"],
        "additionalProperties": false,
    })
}

fn parse_generated_title(response: &str) -> Option<String> {
    if !response.trim_start().starts_with('{') {
        return None;
    }

    let title = serde_json::from_str::<GeneratedPaneTitle>(response)
        .ok()?
        .title;
    let normalized = title
        .trim()
        .trim_matches(|character| matches!(character, '"' | '\'' | '`' | '“' | '”' | '‘' | '’'))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_end_matches(['.', '?', '!'])
        .trim_end()
        .to_owned();
    if normalized.is_empty() {
        return None;
    }

    Some(normalized.chars().take(THREAD_TITLE_MAX_CHARS).collect())
}

fn fallback_pane_name(pane: &PaneTarget) -> Option<String> {
    let cwd = pane.cwd.file_name()?.to_str()?.trim();
    if cwd.is_empty() {
        return None;
    }
    Some(format!("Codex {cwd}"))
}

fn set_tmux_pane_name(sh: &Shell, pane_id: &str, name: &str) -> Result<()> {
    cmd!(sh, "tmux set -p -t {pane_id} @pane_name {name}")
        .quiet()
        .run()?;
    Ok(())
}

fn clear_tmux_pane_name(sh: &Shell, pane_id: &str) -> Result<()> {
    cmd!(sh, "tmux set -pu -t {pane_id} @pane_name")
        .quiet()
        .run()?;
    Ok(())
}

fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| eyre!("HOME is not set"))
}

fn is_client_local(sh: &Shell) -> Result<bool> {
    let fmt = "#{client_pid}";
    let client_pid: u32 = cmd!(sh, "tmux display-message -p {fmt}")
        .quiet()
        .read()?
        .trim()
        .parse()?;

    let mut pid = client_pid;
    while pid > 1 {
        let pid_str = pid.to_string();
        let output = match cmd!(sh, "ps -o comm= -o ppid= -p {pid_str}").quiet().read() {
            Ok(o) => o,
            Err(_) => break,
        };

        let parts: Vec<&str> = output.split_whitespace().collect();
        if parts.len() < 2 {
            break;
        }

        if parts[0].contains("sshd") {
            return Ok(false);
        }

        pid = parts[1].parse().unwrap_or(0);
    }

    Ok(true)
}

fn sync_ssh(sh: &Shell, all: bool) -> Result<()> {
    if !is_client_local(sh)? {
        if all {
            // silent non-zero exit so the shell function skips the unset
            std::process::exit(1);
        }
        return Ok(());
    }

    if !all {
        println!("unset SSH_CONNECTION SSH_CLIENT");
        return Ok(());
    }

    // clear from tmux session env so new panes start clean
    cmd!(sh, "tmux set-environment -u SSH_CONNECTION")
        .quiet()
        .run()
        .ok();
    cmd!(sh, "tmux set-environment -u SSH_CLIENT")
        .quiet()
        .run()
        .ok();

    // send unset to all idle shell panes (skip current — handled by the shell function)
    let self_pane = std::env::var("TMUX_PANE").ok();
    let fmt = "#{pane_id} #{pane_current_command}";
    let panes = cmd!(sh, "tmux list-panes -a -F {fmt}").quiet().read()?;

    for line in panes.lines() {
        let Some((pane_id, current_cmd)) = line.split_once(' ') else {
            continue;
        };

        if self_pane.as_deref() == Some(pane_id) {
            continue;
        }

        if matches!(current_cmd, "zsh" | "bash") {
            cmd!(
                sh,
                "tmux send-keys -t {pane_id} 'unset SSH_CONNECTION SSH_CLIENT' Enter"
            )
            .quiet()
            .run()
            .ok();
        }
    }

    Ok(())
}

fn clear_bell(sh: &Shell) -> Result<()> {
    let format = "#{window_name}";
    let name = cmd!(sh, "tmux display-message -p {format}")
        .quiet()
        .read()
        .unwrap_or_default();
    let name = name.trim();
    if let Some(stripped) = name.strip_prefix("🔔") {
        let new_name = stripped.to_string();
        cmd!(sh, "tmux rename-window {new_name}").quiet().run().ok();
    }
    Ok(())
}

fn notify(
    sh: &Shell,
    kinds: &[NotifyKind],
    message: Option<&str>,
    title: Option<&str>,
    force: bool,
) -> Result<()> {
    // Get the pane ID where this command is running (not the active pane)
    let pane = std::env::var("TMUX_PANE").unwrap_or_default();
    if pane.is_empty() {
        return Ok(()); // Not in tmux
    }

    // Get this pane's window index and name for the notification
    let format = "#{window_index}:#{window_name}";
    let window_info = cmd!(sh, "tmux display-message -t {pane} -p {format}")
        .quiet()
        .read()
        .unwrap_or_default();

    let window_info = window_info.trim().trim_start_matches("🔔");
    let (window_index, window_name) = window_info
        .split_once(':')
        .map(|(i, n)| (i, n.trim_start_matches("🔔")))
        .unwrap_or(("", window_info));

    let title = title.map(|t| t.to_string()).unwrap_or_else(|| {
        let capitalized = window_name
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string() + &window_name[c.len_utf8()..])
            .unwrap_or_default();
        format!("{capitalized} Notification")
    });

    if !force {
        // Check if this pane's window is currently active
        let format = "#{window_active}";
        let active = cmd!(sh, "tmux display-message -t {pane} -p {format}")
            .quiet()
            .read()
            .unwrap_or_default();
        if active.trim() == "1" {
            return Ok(());
        }
    }

    // Default to both if no kind specified
    let use_macos = kinds.is_empty() || kinds.contains(&NotifyKind::Macos);
    let use_bell = kinds.is_empty() || kinds.contains(&NotifyKind::Bell);

    if use_macos {
        // Use window index and name in message if no message provided
        let msg = message
            .map(|m| m.to_string())
            .unwrap_or_else(|| format!("{window_name} ({window_index}) is ready"));
        std::process::Command::new("osascript")
            .args([
                "-e",
                &format!("display notification \"{msg}\" with title \"{title}\""),
            ])
            .output()
            .ok();
    }
    if use_bell {
        // Add 🔔 prefix to this pane's window name (not the active window)
        let format = "#{window_name}";
        let name = cmd!(sh, "tmux display-message -t {pane} -p {format}")
            .quiet()
            .read()
            .unwrap_or_default();
        if !name.starts_with("🔔") {
            let new_name = format!("🔔{}", name.trim());
            cmd!(sh, "tmux rename-window -t {pane} {new_name}")
                .quiet()
                .run()
                .ok();
        }
    }
    Ok(())
}

fn prompt_text(prompt: &str) -> Result<Option<String>> {
    let output_file = NamedTempFile::new()?;
    let output_path = output_file.path().to_path_buf();
    let stdout = File::create(&output_path)?;

    let status = Command::new("sh")
        .arg("-c")
        .arg("printf '' | fzf --print-query --prompt \"$1\" --phony --bind 'enter:accept'")
        .arg("sh")
        .arg(prompt)
        .stdin(Stdio::inherit())
        .stdout(stdout)
        .stderr(Stdio::inherit())
        .status()?;

    if status.code() == Some(130) {
        return Ok(None);
    }

    let mut output = String::new();
    File::open(output_path)?.read_to_string(&mut output)?;
    let value = output
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .map(str::trim)
        .map(str::to_string);
    Ok(value.filter(|value| !value.is_empty()))
}

fn prompt_and_run(sh: &Shell, prompt: &str, command: &str) -> Result<()> {
    if let Some(value) = prompt_text(prompt)? {
        cmd!(sh, "tmux {command} {value}").quiet().run()?;
    }
    Ok(())
}

impl std::fmt::Display for NotifyKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotifyKind::Bell => write!(f, "bell"),
            NotifyKind::Macos => write!(f, "macos"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_synced_codex_name, build_naming_context, build_naming_prompt, latest_thread_name,
        order_panes, order_sessions, order_windows, parse_client_session_context,
        parse_generated_title, parse_index_list, parse_pane_process, resolve_session_index_path,
        thread_id_from_notification, thread_title_output_schema, title_messages_from_rollout,
        ActiveCodexSession, CodexThreadId, CodexThreadName, PaneEntry, PaneTarget, SessionEntry,
        TitleMessage, TitleMessageRole, WindowEntry, FIELD_SEP, THREAD_TITLE_MAX_CHARS,
        THREAD_TITLE_PROMPT_MAX_BYTES,
    };
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;

    fn rollout_file(lines: &[serde_json::Value]) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        for line in lines {
            writeln!(file, "{line}").unwrap();
        }
        file
    }

    fn user_message(texts: &[&str]) -> serde_json::Value {
        serde_json::json!({
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "user",
                "content": texts
                    .iter()
                    .map(|text| serde_json::json!({
                        "type": "input_text",
                        "text": text,
                    }))
                    .collect::<Vec<_>>(),
            },
        })
    }

    fn assistant_message(phase: &str, text: &str) -> serde_json::Value {
        serde_json::json!({
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "assistant",
                "phase": phase,
                "content": [{
                    "type": "output_text",
                    "text": text,
                }],
            },
        })
    }

    #[test]
    fn parses_pane_process_with_ps_spacing() {
        let process =
            parse_pane_process(" 71979 71954 node             node /opt/bin/codex resume").unwrap();

        assert_eq!(process.pid, 71979);
        assert_eq!(process.ppid, 71954);
        assert_eq!(process.command, "node");
        assert_eq!(process.args, "node /opt/bin/codex resume");
    }

    #[test]
    fn parses_thread_id_from_turn_complete_notification() {
        let thread_id =
            thread_id_from_notification(r#"{"type":"agent-turn-complete","thread-id":"thread-1"}"#)
                .unwrap();

        assert_eq!(thread_id, Some(CodexThreadId("thread-1".to_owned())));
    }

    #[test]
    fn ignores_notifications_that_do_not_complete_a_turn() {
        let thread_id = thread_id_from_notification(r#"{"type":"approval-requested"}"#).unwrap();

        assert_eq!(thread_id, None);
    }

    #[test]
    fn rejects_turn_complete_notification_without_thread_id() {
        let error = thread_id_from_notification(r#"{"type":"agent-turn-complete"}"#).unwrap_err();

        assert!(error.to_string().contains("has no thread id"));
    }

    #[test]
    fn latest_session_index_name_wins_and_is_normalized() {
        let mut index = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            index,
            r#"{{"id":"thread-1","thread_name":"Initial title"}}"#
        )
        .unwrap();
        writeln!(index, r#"{{"id":"thread-2","thread_name":"Other title"}}"#).unwrap();
        writeln!(
            index,
            r#"{{"id":"thread-1","thread_name":"  Final\n  title  "}}"#
        )
        .unwrap();

        let name = latest_thread_name(index.path(), &CodexThreadId("thread-1".to_owned())).unwrap();

        assert_eq!(name, Some(CodexThreadName("Final title".to_owned())));
    }

    #[test]
    fn session_index_reader_ignores_partial_trailing_record() {
        let mut index = tempfile::NamedTempFile::new().unwrap();
        writeln!(index, r#"{{"id":"thread-1","thread_name":"Stable title"}}"#).unwrap();
        write!(index, r#"{{"id":"thread-1""#).unwrap();

        let name = latest_thread_name(index.path(), &CodexThreadId("thread-1".to_owned())).unwrap();

        assert_eq!(name, Some(CodexThreadName("Stable title".to_owned())));
    }

    #[test]
    fn session_index_path_prefers_the_active_codex_home() {
        let temp = tempfile::tempdir().unwrap();
        let configured_home = temp.path().join("launch");
        let fallback_home = temp.path().join("home");
        fs::create_dir_all(&configured_home).unwrap();
        fs::write(configured_home.join("session_index.jsonl"), "").unwrap();

        assert_eq!(
            resolve_session_index_path(Some(&configured_home), &fallback_home),
            configured_home.join("session_index.jsonl")
        );
    }

    #[test]
    fn synced_codex_name_keeps_pane_after_api_failure() {
        let mut pane_names = Vec::new();

        let error = apply_synced_codex_name(
            "new name",
            |name| {
                pane_names.push(name.to_owned());
                Ok(())
            },
            || Err(eyre::eyre!("rename rejected")),
        )
        .unwrap_err();

        assert_eq!(pane_names, ["new name"]);
        assert!(error.to_string().contains("rename rejected"));
    }

    #[test]
    fn synced_codex_name_applies_both_when_session_rename_succeeds() {
        let mut pane_names = Vec::new();
        let mut session_names = Vec::new();

        apply_synced_codex_name(
            "new name",
            |name| {
                pane_names.push(name.to_owned());
                Ok(())
            },
            || {
                session_names.push("new name".to_owned());
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(pane_names, ["new name"]);
        assert_eq!(session_names, ["new name"]);
    }

    #[test]
    fn parses_and_normalizes_structured_generated_title() {
        let name = parse_generated_title(r#"{"title":"  `Fix login timeout!`  "}"#).unwrap();

        assert_eq!(name, "Fix login timeout");
        assert!(parse_generated_title("Fix login timeout").is_none());
    }

    #[test]
    fn rollout_user_requests_skip_injected_context() {
        let file = rollout_file(&[
            user_message(&[
                "# AGENTS.md instructions for /Users/praveen/code/dotfiles\n\n<INSTRUCTIONS>\nread first\n</INSTRUCTIONS>",
                "<environment_context>\n  <cwd>/Users/praveen/code/dotfiles</cwd>\n</environment_context>",
            ]),
            user_message(&[
                "i have $cloudflare and $cloudflare-deploy what is the differences between the 2 skills i need to conslidate",
            ]),
            user_message(&["<skill>\n<name>cloudflare</name>\n---\n# Cloudflare\n</skill>"]),
            user_message(&["how are both installed?"]),
        ]);

        assert_eq!(
            title_messages_from_rollout(file.path()),
            vec![
                TitleMessage {
                    role: TitleMessageRole::User,
                    text: "i have $cloudflare and $cloudflare-deploy what is the differences between the 2 skills i need to conslidate".to_owned(),
                },
                TitleMessage {
                    role: TitleMessageRole::User,
                    text: "how are both installed?".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn rollout_user_requests_preserve_real_text_next_to_context_segments() {
        let file = rollout_file(&[user_message(&[
            "<skill>\n<name>cloudflare</name>\n</skill>",
            "does it have everything we need?",
            "<plugins_instructions>\nplugin setup\n</plugins_instructions>",
        ])]);

        assert_eq!(
            title_messages_from_rollout(file.path()),
            vec![TitleMessage {
                role: TitleMessageRole::User,
                text: "does it have everything we need?".to_owned(),
            }]
        );
    }

    #[test]
    fn rollout_user_requests_skip_injected_context_closing_tags() {
        let file = rollout_file(&[user_message(&[
            "</environment_context>",
            "what happened after this?",
            "</skills_instructions>",
            "</plugins_instructions>",
        ])]);

        assert_eq!(
            title_messages_from_rollout(file.path()),
            vec![TitleMessage {
                role: TitleMessageRole::User,
                text: "what happened after this?".to_owned(),
            }]
        );
    }

    #[test]
    fn rollout_title_messages_include_final_answers_and_skip_commentary() {
        let file = rollout_file(&[
            user_message(&["Investigate flaky tests"]),
            assistant_message("commentary", "Checking the test logs"),
            assistant_message("final_answer", "The timeout caused the failures"),
            user_message(&["Fix the timeout"]),
        ]);

        assert_eq!(
            title_messages_from_rollout(file.path()),
            vec![
                TitleMessage {
                    role: TitleMessageRole::User,
                    text: "Investigate flaky tests".to_owned(),
                },
                TitleMessage {
                    role: TitleMessageRole::Assistant,
                    text: "The timeout caused the failures".to_owned(),
                },
                TitleMessage {
                    role: TitleMessageRole::User,
                    text: "Fix the timeout".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn rollout_title_messages_keep_only_the_latest_eight() {
        let messages = (0..10)
            .map(|index| user_message(&[&format!("Request {index}")]))
            .collect::<Vec<_>>();
        let file = rollout_file(&messages);

        let messages = title_messages_from_rollout(file.path());

        assert_eq!(messages.len(), 8);
        assert_eq!(messages.first().unwrap().text, "Request 2");
        assert_eq!(messages.last().unwrap().text, "Request 9");
    }

    #[test]
    fn title_prompt_is_bounded_and_escapes_conversation_markup() {
        let context = super::NamingContext {
            messages: vec![TitleMessage {
                role: TitleMessageRole::User,
                text: format!("Fix <login> & retries {}", "🚀".repeat(1000)),
            }],
        };

        let prompt = build_naming_prompt(&context);

        assert!(prompt.len() <= THREAD_TITLE_PROMPT_MAX_BYTES);
        assert!(prompt.contains("&lt;login&gt; &amp; retries"));
        assert!(std::str::from_utf8(prompt.as_bytes()).is_ok());
    }

    #[test]
    fn title_schema_enforces_codex_display_limit() {
        let schema = thread_title_output_schema();

        assert_eq!(
            schema["properties"]["title"]["maxLength"],
            THREAD_TITLE_MAX_CHARS
        );
        assert_eq!(schema["additionalProperties"], false);
    }

    #[test]
    fn naming_context_uses_visible_text_when_rollout_has_only_context() {
        let file = rollout_file(&[
            user_message(&[
                "# AGENTS.md instructions for /Users/praveen/code/dotfiles\n\n<INSTRUCTIONS>\nread first\n</INSTRUCTIONS>",
                "<environment_context>\n  <cwd>/Users/praveen/code/dotfiles</cwd>\n</environment_context>",
            ]),
            user_message(&["<skill>\n<name>cloudflare</name>\n</skill>"]),
        ]);
        let pane = PaneTarget {
            id: "%1".to_owned(),
            tty: "/dev/ttys001".to_owned(),
            cwd: PathBuf::from("/tmp"),
            current_command: "codex".to_owned(),
            current_name: String::new(),
            visible_text: "visible task text".to_owned(),
        };
        let session = ActiveCodexSession {
            launch_home: PathBuf::from("/tmp/codex-home"),
            socket_path: PathBuf::from("/tmp/control.sock"),
            thread_id: "thread-1".to_owned(),
            rollout_path: Some(file.path().to_path_buf()),
        };

        let context = build_naming_context(&pane, Some(&session));

        assert_eq!(
            context.messages,
            vec![TitleMessage {
                role: TitleMessageRole::User,
                text: "visible task text".to_owned(),
            }]
        );
    }

    #[test]
    fn parses_session_stack_indexes_from_mixed_separators() {
        let indexes = parse_index_list("@5, 3 1:9");
        assert_eq!(indexes, vec![5, 3, 1, 9]);
    }

    #[test]
    fn parses_client_session_context() {
        let (current, last) =
            parse_client_session_context(&format!("current{FIELD_SEP}previous\n"));

        assert_eq!(current.as_deref(), Some("current"));
        assert_eq!(last.as_deref(), Some("previous"));
    }

    #[test]
    fn orders_sessions_by_last_session_and_moves_current_to_end() {
        let mut entries = vec![
            SessionEntry {
                name: "current".into(),
                last_attached: 300,
                activity: 300,
            },
            SessionEntry {
                name: "recent".into(),
                last_attached: 200,
                activity: 200,
            },
            SessionEntry {
                name: "previous".into(),
                last_attached: 100,
                activity: 500,
            },
        ];

        order_sessions(&mut entries, Some("current"), Some("previous"));

        let names = entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["previous", "recent", "current"]);
    }

    #[test]
    fn orders_sessions_by_activity_when_last_attached_is_missing() {
        let mut entries = vec![
            SessionEntry {
                name: "quiet".into(),
                last_attached: 0,
                activity: 100,
            },
            SessionEntry {
                name: "busy".into(),
                last_attached: 0,
                activity: 200,
            },
        ];

        order_sessions(&mut entries, None, None);

        let names = entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["busy", "quiet"]);
    }

    #[test]
    fn orders_windows_by_session_stack_and_moves_active_to_end() {
        let mut entries = vec![
            WindowEntry {
                index: 1,
                name: "current".into(),
                active: true,
                last: false,
            },
            WindowEntry {
                index: 2,
                name: "recent".into(),
                active: false,
                last: true,
            },
            WindowEntry {
                index: 3,
                name: "older".into(),
                active: false,
                last: false,
            },
        ];

        order_windows(&mut entries, &[1, 2, 3]);

        let indexes = entries.iter().map(|entry| entry.index).collect::<Vec<_>>();
        assert_eq!(indexes, vec![2, 3, 1]);
    }

    #[test]
    fn uses_last_window_flag_when_stack_is_missing() {
        let mut entries = vec![
            WindowEntry {
                index: 3,
                name: "third".into(),
                active: false,
                last: false,
            },
            WindowEntry {
                index: 2,
                name: "second".into(),
                active: false,
                last: true,
            },
            WindowEntry {
                index: 1,
                name: "current".into(),
                active: true,
                last: false,
            },
        ];

        order_windows(&mut entries, &[]);

        let indexes = entries.iter().map(|entry| entry.index).collect::<Vec<_>>();
        assert_eq!(indexes, vec![2, 3, 1]);
    }

    #[test]
    fn orders_panes_with_last_first_and_active_last() {
        let mut entries = vec![
            PaneEntry {
                index: 1,
                label: "shell".into(),
                active: true,
                last: false,
            },
            PaneEntry {
                index: 2,
                label: "logs".into(),
                active: false,
                last: true,
            },
            PaneEntry {
                index: 3,
                label: "editor".into(),
                active: false,
                last: false,
            },
        ];

        order_panes(&mut entries);

        let indexes = entries.iter().map(|entry| entry.index).collect::<Vec<_>>();
        assert_eq!(indexes, vec![2, 3, 1]);
    }
}
