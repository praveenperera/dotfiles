use clap::{Args, Parser, Subcommand, ValueEnum};
use eyre::{eyre, Result, WrapErr};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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
    let fmt = format!(
        "#S{FIELD_SEP}#{{session_last_attached}}{FIELD_SEP}#{{session_activity}}{FIELD_SEP}#{{session_active}}"
    );
    let sessions = cmd!(sh, "tmux list-sessions -F {fmt}").quiet().read()?;
    let mut sessions = sessions
        .lines()
        .filter_map(SessionEntry::parse)
        .collect::<Vec<_>>();
    order_sessions(&mut sessions);
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
    active: bool,
}

impl SessionEntry {
    fn parse(line: &str) -> Option<Self> {
        let mut parts = line.split(FIELD_SEP);
        let name = parts.next()?.to_string();
        let last_attached = parse_num(parts.next()?);
        let activity = parse_num(parts.next()?);
        let active = parse_flag(parts.next()?);
        Some(Self {
            name,
            last_attached,
            activity,
            active,
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

fn order_sessions(entries: &mut [SessionEntry]) {
    entries.sort_by(|a, b| {
        demote_active(a.active, b.active)
            .then_with(|| b.recency_key().cmp(&a.recency_key()))
            .then_with(|| a.name.cmp(&b.name))
    });
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
            cmd!(sh, "tmux display-message 'Naming Codex pane...'")
                .quiet()
                .run()?;
            let command = "cmd tmux name-codex-pane";
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
const MAX_FIRST_USER_CHARS: usize = 1500;
const MAX_RECENT_USER_CHARS: usize = 1500;
const MAX_VISIBLE_TEXT_CHARS: usize = 1500;
const MAX_NAME_WORDS: usize = 6;

#[derive(Debug, Clone)]
struct PaneTarget {
    id: String,
    tty: String,
    cwd: PathBuf,
    current_command: String,
    visible_text: String,
}

#[derive(Debug, Clone)]
struct PaneProcess {
    pid: u32,
    ppid: u32,
    command: String,
    args: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct CodexSessionMarker {
    pid: u32,
    launch_home: PathBuf,
    #[serde(default)]
    thread_id: Option<String>,
    #[serde(default)]
    rollout_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct ActiveCodexSession {
    launch_home: PathBuf,
    thread_id: Option<String>,
    rollout_path: Option<PathBuf>,
}

#[derive(Debug, Default)]
struct NamingContext {
    first_user_request: Option<String>,
    recent_user_requests: Vec<String>,
    pane_cwd: PathBuf,
    git_branch: Option<String>,
    visible_text: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum PaneNameAction {
    Named,
    Renamed,
}

#[derive(Debug, PartialEq, Eq)]
enum CodexThreadNameOutcome {
    Updated,
    SessionNotResolved(String),
    NoThreadId,
    UpdateFailed(String),
}

fn name_codex_pane(sh: &Shell, target_pane: Option<&str>) -> Result<()> {
    let pane = read_pane_target(sh, target_pane)?;
    let processes = processes_on_tty(&pane.tty)?;
    if !pane_runs_codex(&pane, &processes) {
        return Err(eyre!("Target pane is not running Codex"));
    }

    let session = resolve_active_codex_session(sh, &processes, &pane.cwd);
    let context = build_naming_context(&pane, session.as_ref().ok())?;
    let prompt = build_naming_prompt(&context);
    let launch_home = session
        .as_ref()
        .ok()
        .map(|session| session.launch_home.as_path());
    let raw_name = run_codex_name_model(&pane.cwd, launch_home, &prompt)
        .wrap_err("Failed to generate pane name")?;
    let name = sanitize_generated_name(&raw_name)
        .or_else(|| fallback_pane_name(&pane))
        .ok_or_else(|| eyre!("Generated pane name was empty"))?;

    set_tmux_pane_name(sh, &pane.id, &name)?;
    let outcome = match session.as_ref() {
        Ok(session) => set_codex_thread_name_for_session(session, &name),
        Err(err) => CodexThreadNameOutcome::SessionNotResolved(err.to_string()),
    };
    report_codex_thread_name_result(sh, PaneNameAction::Named, &name, outcome);

    Ok(())
}

fn rename_pane(sh: &Shell, target_pane: Option<&str>, name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return Ok(());
    }

    let pane = read_pane_target(sh, target_pane)?;
    set_tmux_pane_name(sh, &pane.id, name)?;

    let processes = processes_on_tty(&pane.tty)?;
    if !pane_runs_codex(&pane, &processes) {
        report_name_result(sh, "Pane renamed; target pane is not running Codex");
        return Ok(());
    }

    let session = resolve_active_codex_session(sh, &processes, &pane.cwd);
    let outcome = match session.as_ref() {
        Ok(session) => set_codex_thread_name_for_session(session, name),
        Err(err) => CodexThreadNameOutcome::SessionNotResolved(err.to_string()),
    };
    report_codex_thread_name_result(sh, PaneNameAction::Renamed, name, outcome);

    Ok(())
}

fn set_codex_thread_name_for_session(
    session: &ActiveCodexSession,
    name: &str,
) -> CodexThreadNameOutcome {
    let Some(thread_id) = session.thread_id.as_deref() else {
        return CodexThreadNameOutcome::NoThreadId;
    };

    match set_codex_thread_name(&session.launch_home, thread_id, name) {
        Ok(()) => CodexThreadNameOutcome::Updated,
        Err(err) => CodexThreadNameOutcome::UpdateFailed(err.to_string()),
    }
}

fn report_codex_thread_name_result(
    sh: &Shell,
    action: PaneNameAction,
    name: &str,
    outcome: CodexThreadNameOutcome,
) {
    report_name_result(sh, &codex_thread_name_message(action, name, outcome));
}

fn codex_thread_name_message(
    action: PaneNameAction,
    name: &str,
    outcome: CodexThreadNameOutcome,
) -> String {
    let verb = match action {
        PaneNameAction::Named => "named",
        PaneNameAction::Renamed => "renamed",
    };

    match outcome {
        CodexThreadNameOutcome::Updated => {
            format!("Pane and Codex session {verb}: {name}")
        }
        CodexThreadNameOutcome::SessionNotResolved(err) => {
            format!("Pane {verb}; Codex session not resolved: {err}")
        }
        CodexThreadNameOutcome::NoThreadId => {
            format!("Pane {verb}; Codex session has no thread id")
        }
        CodexThreadNameOutcome::UpdateFailed(err) => {
            format!("Pane {verb}; Codex session rename failed: {err}")
        }
    }
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
        "#{{pane_id}}{FIELD_SEP}#{{pane_tty}}{FIELD_SEP}#{{pane_current_path}}{FIELD_SEP}#{{pane_current_command}}"
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
    let visible_text = capture_visible_pane_text(sh, &id);

    Ok(PaneTarget {
        id,
        tty,
        cwd,
        current_command,
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
    _sh: &Shell,
    processes: &[PaneProcess],
    pane_cwd: &Path,
) -> Result<ActiveCodexSession> {
    let codex_pids = codex_process_family(processes);
    let markers = active_codex_session_markers()?;
    if let Some(marker) = markers
        .into_iter()
        .find(|marker| codex_pids.contains(&marker.pid))
    {
        let fallback_thread = newest_thread_for_cwd(&marker.launch_home, pane_cwd);
        let thread_id = marker
            .thread_id
            .or_else(|| fallback_thread.as_ref().map(|thread| thread.id.clone()));
        let rollout_path = marker
            .rollout_path
            .or_else(|| fallback_thread.map(|thread| thread.rollout_path));

        return Ok(ActiveCodexSession {
            launch_home: marker.launch_home,
            thread_id,
            rollout_path,
        });
    }

    Err(eyre!("Codex session marker not found"))
}

#[derive(Debug, Clone)]
struct StateThread {
    id: String,
    rollout_path: PathBuf,
}

fn newest_thread_for_cwd(codex_home: &Path, cwd: &Path) -> Option<StateThread> {
    newest_thread_for_cwd_with_filter(codex_home, cwd, Some(codex_home))
        .or_else(|| newest_thread_for_cwd_with_filter(codex_home, cwd, None))
}

fn newest_thread_for_cwd_with_filter(
    codex_home: &Path,
    cwd: &Path,
    rollout_home: Option<&Path>,
) -> Option<StateThread> {
    let state_db = codex_home.join("state_5.sqlite");
    if !state_db.exists() {
        return None;
    }

    let mut sql = format!(
        "select id, rollout_path from threads \
         where source = 'cli' \
         and (agent_role is null or agent_role = '') \
         and cwd = {}",
        sqlite_quote(cwd.to_str()?)
    );
    if let Some(rollout_home) = rollout_home {
        let sessions_prefix = rollout_home.join("sessions");
        sql.push_str(&format!(
            " and rollout_path like {}",
            sqlite_quote(&format!("{}/%", sessions_prefix.display()))
        ));
    }
    sql.push_str(" order by updated_at_ms desc, updated_at desc limit 1;");

    let output = Command::new("sqlite3")
        .arg("-separator")
        .arg("\t")
        .arg(format!("file:{}?mode=ro", state_db.display()))
        .arg(sql)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    parse_state_thread(String::from_utf8_lossy(&output.stdout).trim())
}

fn parse_state_thread(line: &str) -> Option<StateThread> {
    let (id, rollout_path) = line.split_once('\t')?;
    if id.is_empty() || rollout_path.is_empty() {
        return None;
    }

    Some(StateThread {
        id: id.to_owned(),
        rollout_path: PathBuf::from(rollout_path),
    })
}

fn sqlite_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
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

fn active_codex_session_markers() -> Result<Vec<CodexSessionMarker>> {
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
                .and_then(|bytes| serde_json::from_slice::<CodexSessionMarker>(&bytes).ok())
            {
                Some(marker) if process_exists(marker.pid) => marker,
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

fn build_naming_context(
    pane: &PaneTarget,
    session: Option<&ActiveCodexSession>,
) -> Result<NamingContext> {
    let mut context = NamingContext {
        pane_cwd: pane.cwd.clone(),
        git_branch: git_branch(&pane.cwd),
        ..NamingContext::default()
    };

    if let Some(rollout_path) = session.and_then(|session| session.rollout_path.as_deref()) {
        let user_requests = user_requests_from_rollout(rollout_path);
        context.first_user_request = user_requests
            .first()
            .map(|value| clip_chars(value, MAX_FIRST_USER_CHARS));
        context.recent_user_requests = recent_user_requests(&user_requests);
    }

    if context.first_user_request.is_none() && context.recent_user_requests.is_empty() {
        context.visible_text = Some(clip_chars(&pane.visible_text, MAX_VISIBLE_TEXT_CHARS));
    }

    Ok(context)
}

fn user_requests_from_rollout(path: &Path) -> Vec<String> {
    read_rollout_lines(path, |value| {
        value
            .get("type")
            .and_then(serde_json::Value::as_str)
            .filter(|kind| *kind == "response_item")?;
        let payload = value.get("payload")?;
        payload
            .get("type")
            .and_then(serde_json::Value::as_str)
            .filter(|kind| *kind == "message")?;
        payload
            .get("role")
            .and_then(serde_json::Value::as_str)
            .filter(|role| *role == "user")?;

        let text = payload
            .get("content")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|item| item.get("text").or_else(|| item.get("input_text")))
            .filter_map(serde_json::Value::as_str)
            .collect::<Vec<_>>()
            .join("\n");
        (!text.trim().is_empty()).then(|| text.trim().to_owned())
    })
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

fn recent_user_requests(user_requests: &[String]) -> Vec<String> {
    let mut total = 0;
    let mut recent = Vec::new();
    for request in user_requests.iter().rev().take(3) {
        let remaining = MAX_RECENT_USER_CHARS.saturating_sub(total);
        if remaining == 0 {
            break;
        }
        let clipped = clip_chars(request, remaining);
        total += clipped.chars().count();
        recent.push(clipped);
    }
    recent.reverse();
    recent
}

fn git_branch(cwd: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!branch.is_empty()).then_some(branch)
}

fn build_naming_prompt(context: &NamingContext) -> String {
    let mut prompt = String::from(
        "Generate one short title for this Codex pane.\n\
         Output only the title, with 4 to 6 words. No quotes, labels, punctuation-only lines, or explanation.\n",
    );
    prompt.push_str("\nContext:\n");
    prompt.push_str(&format!("cwd: {}\n", context.pane_cwd.display()));
    if let Some(branch) = &context.git_branch {
        prompt.push_str(&format!("git branch: {branch}\n"));
    }
    if let Some(first) = &context.first_user_request {
        prompt.push_str("\nfirst user request:\n");
        prompt.push_str(first);
        prompt.push('\n');
    }
    if !context.recent_user_requests.is_empty() {
        prompt.push_str("\nrecent user requests:\n");
        for request in &context.recent_user_requests {
            prompt.push_str("- ");
            prompt.push_str(&request.replace('\n', "\n  "));
            prompt.push('\n');
        }
    }
    if let Some(visible_text) = &context.visible_text {
        prompt.push_str("\nvisible pane text fallback:\n");
        prompt.push_str(visible_text);
        prompt.push('\n');
    }
    prompt
}

fn run_codex_name_model(cwd: &Path, launch_home: Option<&Path>, prompt: &str) -> Result<String> {
    let output_file = TempFileBuilder::new()
        .prefix("codex-pane-name")
        .tempfile()?;
    let output_path = output_file.path().to_path_buf();
    let codex_home = launch_home
        .map(Path::to_path_buf)
        .unwrap_or(home_dir()?.join(".codex"));

    let mut child = Command::new("codex")
        .args(["exec", "--ephemeral", "--model", NAME_MODEL, "--cd"])
        .arg(cwd)
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

fn sanitize_generated_name(raw: &str) -> Option<String> {
    let line = raw.lines().find(|line| !line.trim().is_empty())?.trim();
    let mut value = line
        .trim_matches(|ch| matches!(ch, '"' | '\'' | '`' | ' ' | '\t'))
        .to_owned();
    let lower = value.to_ascii_lowercase();
    for label in ["pane:", "title:", "name:", "session:"] {
        if lower.starts_with(label) {
            value = value[label.len()..]
                .trim()
                .trim_matches(|ch| matches!(ch, '"' | '\'' | '`' | ' ' | '\t'))
                .to_owned();
            break;
        }
    }
    value.retain(|ch| !ch.is_control());
    let words = value
        .split_whitespace()
        .take(MAX_NAME_WORDS)
        .collect::<Vec<_>>();
    (!words.is_empty()).then(|| words.join(" "))
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

fn set_codex_thread_name(codex_home: &Path, thread_id: &str, name: &str) -> Result<()> {
    let state_db = codex_home.join("state_5.sqlite");
    if !state_db.exists() {
        return Err(eyre!("Codex state db not found"));
    }

    let sql = format!(
        "update threads set title = {} where id = {}; select changes();",
        sqlite_quote(name),
        sqlite_quote(thread_id)
    );
    let output = Command::new("sqlite3")
        .arg(format!("file:{}?mode=rw", state_db.display()))
        .arg(sql)
        .output()?;
    if !output.status.success() {
        return Err(eyre!("Failed to update Codex thread title"));
    }

    let changes = String::from_utf8_lossy(&output.stdout);
    if changes.trim() == "0" {
        Err(eyre!("Codex thread not found"))
    } else {
        Ok(())
    }
}

fn clip_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
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
        codex_thread_name_message, order_panes, order_sessions, order_windows, parse_index_list,
        parse_pane_process, sanitize_generated_name, CodexSessionMarker, CodexThreadNameOutcome,
        PaneEntry, PaneNameAction, SessionEntry, WindowEntry,
    };
    use std::path::PathBuf;

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
    fn sanitizes_generated_name_to_first_six_words() {
        let name =
            sanitize_generated_name("Title: `Implement Codex Pane Session Auto Naming`\nextra")
                .unwrap();

        assert_eq!(name, "Implement Codex Pane Session Auto Naming");
    }

    #[test]
    fn codex_session_marker_reads_old_markers_without_thread_id() {
        let marker = serde_json::from_str::<CodexSessionMarker>(
            r#"{"pid":71979,"launch_home":"/tmp/launch"}"#,
        )
        .unwrap();

        assert_eq!(marker.pid, 71979);
        assert_eq!(marker.launch_home, PathBuf::from("/tmp/launch"));
        assert_eq!(marker.thread_id, None);
        assert_eq!(marker.rollout_path, None);
    }

    #[test]
    fn codex_session_marker_reads_captured_thread_id() {
        let marker = serde_json::from_str::<CodexSessionMarker>(
            r#"{"pid":71979,"launch_home":"/tmp/launch","thread_id":"thread-1","rollout_path":"/tmp/launch/sessions/rollout.jsonl"}"#,
        )
        .unwrap();

        assert_eq!(marker.thread_id.as_deref(), Some("thread-1"));
        assert_eq!(
            marker.rollout_path.as_deref(),
            Some(PathBuf::from("/tmp/launch/sessions/rollout.jsonl").as_path())
        );
    }

    #[test]
    fn codex_thread_name_message_reports_missing_thread_id() {
        let message = codex_thread_name_message(
            PaneNameAction::Named,
            "New Pane Name",
            CodexThreadNameOutcome::NoThreadId,
        );

        assert_eq!(message, "Pane named; Codex session has no thread id");
    }

    #[test]
    fn codex_thread_name_message_reports_update_failure() {
        let message = codex_thread_name_message(
            PaneNameAction::Renamed,
            "New Pane Name",
            CodexThreadNameOutcome::UpdateFailed("Codex thread not found".to_owned()),
        );

        assert_eq!(
            message,
            "Pane renamed; Codex session rename failed: Codex thread not found"
        );
    }

    #[test]
    fn codex_thread_name_message_reports_resolution_failure() {
        let message = codex_thread_name_message(
            PaneNameAction::Named,
            "New Pane Name",
            CodexThreadNameOutcome::SessionNotResolved("Codex session marker not found".to_owned()),
        );

        assert_eq!(
            message,
            "Pane named; Codex session not resolved: Codex session marker not found"
        );
    }

    #[test]
    fn parses_session_stack_indexes_from_mixed_separators() {
        let indexes = parse_index_list("@5, 3 1:9");
        assert_eq!(indexes, vec![5, 3, 1, 9]);
    }

    #[test]
    fn orders_sessions_by_last_attached_and_moves_active_to_end() {
        let mut entries = vec![
            SessionEntry {
                name: "current".into(),
                last_attached: 300,
                activity: 300,
                active: true,
            },
            SessionEntry {
                name: "recent".into(),
                last_attached: 200,
                activity: 200,
                active: false,
            },
            SessionEntry {
                name: "older".into(),
                last_attached: 100,
                activity: 500,
                active: false,
            },
        ];

        order_sessions(&mut entries);

        let names = entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["recent", "older", "current"]);
    }

    #[test]
    fn orders_sessions_by_activity_when_last_attached_is_missing() {
        let mut entries = vec![
            SessionEntry {
                name: "quiet".into(),
                last_attached: 0,
                activity: 100,
                active: false,
            },
            SessionEntry {
                name: "busy".into(),
                last_attached: 0,
                activity: 200,
                active: false,
            },
        ];

        order_sessions(&mut entries);

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
