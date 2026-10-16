use std::process::Command;

use eyre::{bail, Context, Result};
use serde::Deserialize;

use super::transcript::{Provider, Session};

/// A Claude Code background agent (`claude --bg`) that still owns a session.
/// The CLI refuses `--resume` on a session a background agent holds, so every
/// T3 turn on the imported thread fails until the agent stops
pub struct BackgroundAgent {
    pub id: String,
    pub busy: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentEntry {
    kind: String,
    session_id: String,
    id: Option<String>,
    status: Option<String>,
}

impl BackgroundAgent {
    /// The background agent running `session`, if any
    pub fn holding(session: &Session) -> Result<Option<Self>> {
        if session.provider != Provider::Claude {
            return Ok(None);
        }

        let output = Command::new("claude")
            .args(["agents", "--json"])
            .output()
            .wrap_err("run claude agents --json")?;

        if !output.status.success() {
            bail!(
                "claude agents --json failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }

        let entries: Vec<AgentEntry> =
            serde_json::from_slice(&output.stdout).wrap_err("parse claude agents --json")?;

        let agent = entries
            .into_iter()
            .find(|entry| entry.kind == "background" && entry.session_id == session.id)
            .map(|entry| Self {
                // the short id is what `claude stop` takes, and it is a session id prefix
                id: entry.id.unwrap_or_else(|| session.id.clone()),
                busy: entry.status.as_deref() == Some("busy"),
            });

        Ok(agent)
    }

    /// Stop the agent. Claude keeps the conversation, and `claude attach` reopens it
    pub fn stop(&self) -> Result<()> {
        let output = Command::new("claude")
            .args(["stop", &self.id])
            .output()
            .wrap_err("run claude stop")?;

        if !output.status.success() {
            bail!(
                "claude stop {} failed: {}",
                self.id,
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }

        Ok(())
    }
}
