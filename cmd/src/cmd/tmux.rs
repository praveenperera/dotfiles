use clap::{Args, Parser, Subcommand, ValueEnum};
use eyre::Result;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::ffi::OsString;
use std::process::{Command, Stdio};
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
    "Toggle Pane Names",
    "Scroll Back",
    "Move Tab to Session",
];

fn action_picker(sh: &Shell) -> Result<()> {
    let menu = ACTIONS.join("\n");
    let selection = run_fzf(sh, "Action > ", &menu)?;
    action(sh, &selection)
}

fn spawn_command_prompt(sh: &Shell, prompt: &str, action: &str) -> Result<()> {
    let client_fmt = "#{client_tty}";
    let client = cmd!(sh, "tmux display-message -p {client_fmt}")
        .quiet()
        .read()
        .unwrap_or_default();
    let client = client.trim();

    let mut command = Command::new("tmux");
    command.stdin(Stdio::null());
    command.stdout(Stdio::null());
    command.stderr(Stdio::null());
    command.arg("command-prompt");
    if !client.is_empty() {
        command.args(["-t", client]);
    }
    command.args(["-b", "-p", prompt, action]);
    command.spawn()?;
    Ok(())
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
            let prompt = "Window name:";
            let action = "rename-window '%1'";
            spawn_command_prompt(sh, prompt, action)?;
        }
        "Rename Session" => {
            let prompt = "Session name:";
            let action = "rename-session '%1'";
            spawn_command_prompt(sh, prompt, action)?;
        }
        "Rename Pane" => {
            let prompt = "Pane name:";
            let action = "set -p @pane_name '%1'";
            spawn_command_prompt(sh, prompt, action)?;
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
        order_panes, order_sessions, order_windows, parse_index_list, PaneEntry, SessionEntry,
        WindowEntry,
    };

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
