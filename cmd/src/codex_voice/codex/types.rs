use std::{fmt, path::PathBuf};

use serde::Serialize;

use crate::codex_voice::tmux::PaneMetadata;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ThreadId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TurnId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PaneId(pub String);

impl ThreadId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ThreadId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for TurnId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone)]
pub struct CodexThread {
    pub id: ThreadId,
    pub title: String,
    pub cwd: PathBuf,
    pub rollout_path: PathBuf,
    pub source: ResolutionSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionSource {
    ExplicitThread,
    TerminalTitle,
    CwdLatest,
}

impl fmt::Display for ResolutionSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolutionSource::ExplicitThread => f.write_str("explicit_thread"),
            ResolutionSource::TerminalTitle => f.write_str("terminal_title"),
            ResolutionSource::CwdLatest => f.write_str("cwd_latest"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CodexContext {
    pub pane: Option<PaneMetadata>,
    pub thread: CodexThread,
}

impl CodexContext {
    pub fn format_human(&self) -> String {
        let mut lines = Vec::new();
        if let Some(pane) = &self.pane {
            lines.push(format!("pane_id: {}", pane.id));
            lines.push(format!("pane_cwd: {}", pane.cwd.display()));
            lines.push(format!("pane_command: {}", pane.current_command));
            lines.push(format!("pane_title: {}", pane.title));
        } else {
            lines.push("pane: none (--thread was provided)".to_owned());
        }
        lines.push(format!("thread_id: {}", self.thread.id));
        lines.push(format!("thread_title: {}", self.thread.title));
        lines.push(format!("thread_cwd: {}", self.thread.cwd.display()));
        lines.push(format!(
            "rollout_path: {}",
            self.thread.rollout_path.display()
        ));
        lines.push(format!("source: {}", self.thread.source));
        lines.join("\n")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadableKind {
    Plan,
    AgentMessage,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadableItem {
    pub thread_id: ThreadId,
    pub turn_id: TurnId,
    pub kind: ReadableKind,
    pub text: String,
    pub source: ResolutionSource,
}
