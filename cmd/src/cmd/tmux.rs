use clap::{Args, Parser, Subcommand, ValueEnum};
use eyre::Result;
use std::ffi::OsString;
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
#[command(name = "notf", about = "Send terminal notification (bell, macos, or both)", arg_required_else_help = true)]
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
}

pub fn run_with_flags(sh: &Shell, flags: Tmux) -> Result<()> {
    match flags.subcommand {
        TmuxCmd::MoveAfter { position } => move_after(sh, position),
        TmuxCmd::ClearBell => clear_bell(sh),
        TmuxCmd::Notify {
            kind,
            message,
            title,
            force,
        } => notify(sh, &kind, message.as_deref(), title.as_deref(), force),
    }
}

pub fn notify_run(sh: &Shell, args: &[OsString]) -> Result<()> {
    let flags = NotifyArgs::parse_from(args);
    notify(sh, &flags.kind, flags.message.as_deref(), flags.title.as_deref(), flags.force)
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
