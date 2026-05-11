use std::{path::PathBuf, process::Command};

use color_eyre::eyre::{eyre, Result, WrapErr};
use serde::Deserialize;

use super::types::{CodexThread, ResolutionSource, ThreadId};
use crate::codex_voice::config;

#[derive(Debug, Clone)]
pub struct ThreadStore {
    codex_home: PathBuf,
}

impl ThreadStore {
    pub fn from_default_home() -> Result<Self> {
        Ok(Self {
            codex_home: config::home_dir()?.join(".codex"),
        })
    }

    pub fn find_by_id(&self, thread_id: &str, source: ResolutionSource) -> Result<CodexThread> {
        let sql = format!(
            "select id, title, cwd, rollout_path from threads where id = {} limit 1;",
            sqlite_quote(thread_id)
        );
        let rows = self.query_threads(&sql)?;
        rows.into_iter()
            .next()
            .map(|row| row.into_thread(source))
            .ok_or_else(|| eyre!("Codex thread not found: {thread_id}"))
    }

    pub fn latest_for_cwd(&self, cwd: &std::path::Path) -> Result<Vec<CodexThread>> {
        let cwd = cwd
            .to_str()
            .ok_or_else(|| eyre!("Pane cwd is not valid UTF-8: {}", cwd.display()))?;
        let sql = format!(
            "select id, title, cwd, rollout_path from threads \
             where source = 'cli' \
             and archived = 0 \
             and (agent_role is null or agent_role = '') \
             and cwd = {} \
             order by updated_at_ms desc, updated_at desc limit 5;",
            sqlite_quote(cwd)
        );
        Ok(self
            .query_threads(&sql)?
            .into_iter()
            .map(|row| row.into_thread(ResolutionSource::CwdLatest))
            .collect())
    }

    fn query_threads(&self, sql: &str) -> Result<Vec<ThreadRow>> {
        let state_db = self.codex_home.join("state_5.sqlite");
        if !state_db.exists() {
            return Err(eyre!(
                "Codex state database not found: {}",
                state_db.display()
            ));
        }

        let output = Command::new("sqlite3")
            .arg("-json")
            .arg(format!("file:{}?mode=ro", state_db.display()))
            .arg(sql)
            .output()
            .wrap_err("Failed to run sqlite3 for Codex state")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eyre!("Codex state query failed: {}", stderr.trim()));
        }

        let stdout = output.stdout.trim_ascii();
        if stdout.is_empty() {
            return Ok(Vec::new());
        }

        serde_json::from_slice::<Vec<ThreadRow>>(stdout)
            .wrap_err("Failed to parse Codex state query JSON")
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ThreadRow {
    id: String,
    #[serde(default)]
    title: String,
    cwd: PathBuf,
    rollout_path: PathBuf,
}

impl ThreadRow {
    fn into_thread(self, source: ResolutionSource) -> CodexThread {
        CodexThread {
            id: ThreadId(self.id),
            title: self.title,
            cwd: self.cwd,
            rollout_path: self.rollout_path,
            source,
        }
    }
}

fn sqlite_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
