use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use eyre::{bail, eyre, Context, Result};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Value};

use super::transcript::{same_dir, Provider, Session};
use crate::{fsutil, runtime};

/// Most restrictive mode, for when T3 has no thread to copy the user's choice from
const FALLBACK_RUNTIME_MODE: &str = "approval-required";

/// The running T3 server, found through the state file it writes once listening
#[derive(Debug)]
pub struct Server {
    userdata: PathBuf,
    origin: String,
    bin: PathBuf,
}

#[derive(Deserialize)]
struct RuntimeState {
    pid: u32,
    origin: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IssuedSession {
    session_id: String,
    token: String,
}

impl Server {
    pub fn locate() -> Result<Self> {
        let userdata = match std::env::var_os("T3CODE_HOME") {
            Some(home) => PathBuf::from(home).join("userdata"),
            None => fsutil::home_dir()?.join(".t3/userdata"),
        };

        let state_path = userdata.join("server-runtime.json");
        let state: RuntimeState = serde_json::from_slice(
            &fs::read(&state_path).wrap_err("T3 server is not running (no server-runtime.json)")?,
        )
        .wrap_err_with(|| format!("parse {}", state_path.display()))?;

        // the state file survives crashes, so the pid is the liveness check, and
        // its executable is the t3 build that matches the database schema
        let bin = Command::new("ps")
            .args(["-o", "comm=", "-p", &state.pid.to_string()])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()))
            .filter(|path| path.ends_with("t3"))
            .ok_or_else(|| eyre!("T3 server pid {} is not a running t3 binary", state.pid))?;

        Ok(Self {
            userdata,
            origin: state.origin,
            bin,
        })
    }

    pub fn db(&self, read_only: bool) -> Result<Db> {
        let path = self.userdata.join("state.sqlite");
        let flags = if read_only {
            OpenFlags::SQLITE_OPEN_READ_ONLY
        } else {
            OpenFlags::SQLITE_OPEN_READ_WRITE
        };

        let conn = Connection::open_with_flags(&path, flags)
            .wrap_err_with(|| format!("open {}", path.display()))?;

        conn.busy_timeout(Duration::from_secs(10))?;
        Ok(Db(conn))
    }

    /// Short-lived admin token, revoked when the returned client drops
    pub fn client(&self) -> Result<Client> {
        let output = Command::new(&self.bin)
            .args(["auth", "session", "issue", "--json", "--ttl", "5m"])
            .args(["--label", "cmd t3 import", "--log-level", "error"])
            .output()
            .wrap_err_with(|| format!("run {}", self.bin.display()))?;

        if !output.status.success() {
            bail!(
                "t3 auth session issue failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }

        let issued: IssuedSession =
            serde_json::from_slice(&output.stdout).wrap_err("parse t3 auth session output")?;

        Ok(Client {
            bin: self.bin.clone(),
            origin: self.origin.clone(),
            session_id: issued.session_id,
            token: issued.token,
        })
    }
}

pub struct Client {
    bin: PathBuf,
    origin: String,
    session_id: String,
    token: String,
}

impl Client {
    /// Send a client orchestration command through the HTTP API
    pub fn dispatch(&self, command: Value) -> Result<()> {
        let url = format!("{}/api/orchestration/dispatch", self.origin);
        let token = self.token.clone();
        let (status, body) = runtime::block_on(async move {
            // the detached hook child has no terminal, so a hung server must not hold it forever
            let response = reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()?
                .post(url)
                .bearer_auth(token)
                .json(&command)
                .send()
                .await?;

            let status = response.status();
            let body = response.text().await?;
            Ok::<_, reqwest::Error>((status, body))
        })??;

        if !status.is_success() {
            bail!("T3 rejected the command ({status}): {body}");
        }

        Ok(())
    }

    pub fn create_project(&self, root: &Path) -> Result<String> {
        let project_id = uuid::Uuid::new_v4().to_string();
        let title = root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.display().to_string());

        self.dispatch(json!({
            "type": "project.create",
            "commandId": uuid::Uuid::new_v4().to_string(),
            "projectId": project_id,
            "title": title,
            "workspaceRoot": root,
            "createWorkspaceRootIfMissing": false,
            "createdAt": now(),
        }))?;

        Ok(project_id)
    }

    pub fn create_thread(&self, thread: &NewThread) -> Result<()> {
        self.dispatch(json!({
            "type": "thread.create",
            "commandId": uuid::Uuid::new_v4().to_string(),
            "threadId": thread.id,
            "projectId": thread.project_id,
            "title": thread.title,
            "modelSelection": {
                "instanceId": thread.provider.instance_id(),
                "model": thread.model,
            },
            "runtimeMode": thread.runtime_mode,
            "branch": thread.branch,
            "worktreePath": thread.worktree_path,
            "createdAt": now(),
        }))
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        // an unrevoked token still expires after its ttl, so failure here is harmless
        let _ = Command::new(&self.bin)
            .args(["auth", "session", "revoke", &self.session_id])
            .args(["--log-level", "error"])
            .output();
    }
}

pub struct BoundThread {
    pub id: String,
    pub deleted: bool,
}

pub struct NewThread {
    pub id: String,
    pub project_id: String,
    pub provider: Provider,
    pub title: String,
    pub model: String,
    pub branch: Option<String>,
    pub worktree_path: Option<PathBuf>,
    pub runtime_mode: String,
}

/// T3's state database. The only write is the provider binding, a plain table
/// outside the event log that T3 itself rewrites whenever a session changes
pub struct Db(Connection);

impl Db {
    /// T3 thread already bound to this provider session, imported or native.
    /// A fresh binding with no thread yet is another import in flight. Deleted
    /// threads only count when `include_deleted` is set, and live ones win
    pub fn thread_for_session(
        &self,
        session: &Session,
        include_deleted: bool,
    ) -> Result<Option<BoundThread>> {
        // T3's Claude adapter still reads the older `sessionId` cursor key
        let session_key = match session.provider {
            Provider::Claude => {
                "COALESCE(json_extract(runtime.resume_cursor_json, '$.resume'),
                                          json_extract(runtime.resume_cursor_json, '$.sessionId'))"
            }
            Provider::Codex => "json_extract(runtime.resume_cursor_json, '$.threadId')",
        };

        let in_flight_since = (chrono::Utc::now() - chrono::Duration::minutes(5))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        self.0
            .query_row(
                &format!(
                    "SELECT runtime.thread_id, thread.deleted_at IS NOT NULL
                     FROM provider_session_runtime AS runtime
                     LEFT JOIN projection_threads AS thread
                       ON thread.thread_id = runtime.thread_id
                     WHERE runtime.provider_name = ?1
                       AND json_valid(runtime.resume_cursor_json)
                       AND {session_key} = ?2
                       AND (thread.thread_id IS NOT NULL AND (thread.deleted_at IS NULL OR ?4)
                            OR thread.thread_id IS NULL AND runtime.last_seen_at > ?3)
                     ORDER BY thread.deleted_at IS NOT NULL
                     LIMIT 1"
                ),
                params![
                    session.provider.instance_id(),
                    session.id,
                    in_flight_since,
                    include_deleted
                ],
                |row| {
                    Ok(BoundThread {
                        id: row.get(0)?,
                        deleted: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn thread_exists(&self, thread_id: &str) -> Result<bool> {
        Ok(self
            .0
            .query_row(
                "SELECT 1 FROM projection_threads WHERE thread_id = ?1",
                [thread_id],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    /// Permission mode of the newest T3 thread on this provider, so imports
    /// follow the user's T3 choice instead of granting a fixed level
    pub fn runtime_mode(&self, provider: Provider) -> Result<String> {
        let mode: Option<String> = self
            .0
            .query_row(
                "SELECT runtime_mode
                 FROM projection_threads
                 WHERE json_valid(model_selection_json)
                   AND json_extract(model_selection_json, '$.instanceId') = ?1
                   AND deleted_at IS NULL
                 ORDER BY created_at DESC
                 LIMIT 1",
                [provider.instance_id()],
                |row| row.get(0),
            )
            .optional()?;

        Ok(mode.unwrap_or_else(|| FALLBACK_RUNTIME_MODE.to_owned()))
    }

    /// Model of the newest T3 thread on this provider, for transcripts that never recorded one
    pub fn latest_model(&self, provider: Provider) -> Result<Option<String>> {
        self.0
            .query_row(
                "SELECT json_extract(model_selection_json, '$.model')
                 FROM projection_threads
                 WHERE json_valid(model_selection_json)
                   AND json_extract(model_selection_json, '$.instanceId') = ?1
                 ORDER BY created_at DESC
                 LIMIT 1",
                [provider.instance_id()],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    /// Project whose workspace root is exactly `root`
    pub fn project_at(&self, root: &Path) -> Result<Option<String>> {
        let mut statement = self.0.prepare(
            "SELECT project_id, workspace_root FROM projection_projects WHERE deleted_at IS NULL",
        )?;

        let projects = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for project in projects {
            let (project_id, workspace_root) = project?;
            if same_dir(Path::new(&workspace_root), root) {
                return Ok(Some(project_id));
            }
        }

        Ok(None)
    }

    /// Bind a not-yet-visible thread to the CLI session so its first turn
    /// resumes it, matching the cursor shapes T3's own importer writes
    pub fn bind(&self, thread: &NewThread, session: &Session) -> Result<()> {
        let thread_id = &thread.id;
        let cursor = match session.provider {
            Provider::Claude => json!({ "threadId": thread_id, "resume": session.id }),
            Provider::Codex => json!({ "threadId": session.id }),
        };

        let instance = session.provider.instance_id();
        let inserted = self.0.execute(
            "INSERT INTO provider_session_runtime (
                thread_id, provider_name, provider_instance_id, adapter_key, runtime_mode,
                status, last_seen_at, resume_cursor_json, runtime_payload_json
             )
             VALUES (?1, ?2, ?2, ?2, ?3, 'stopped', ?4, ?5, ?6)
             ON CONFLICT (thread_id) DO NOTHING",
            params![
                thread_id,
                instance,
                thread.runtime_mode,
                now(),
                cursor.to_string(),
                json!({ "cwd": session.cwd }).to_string(),
            ],
        )?;

        if inserted == 0 {
            bail!("thread {thread_id} already has a provider binding");
        }

        Ok(())
    }

    pub fn unbind(&self, thread_id: &str) -> Result<()> {
        self.0.execute(
            "DELETE FROM provider_session_runtime WHERE thread_id = ?1 AND status = 'stopped'",
            [thread_id],
        )?;

        Ok(())
    }
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
