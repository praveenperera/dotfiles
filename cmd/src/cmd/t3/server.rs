use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use eyre::{bail, eyre, Context, Result};
use futures::{SinkExt, StreamExt};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;

use super::transcript::{same_dir, Provider, Session};
use crate::{fsutil, runtime};

/// Most restrictive mode, for when T3 has no thread to copy the user's choice from
const FALLBACK_RUNTIME_MODE: &str = "approval-required";

/// Thread commands are only accepted over the v2 orchestration socket
const ORCHESTRATION_PROTOCOL: u32 = 2;

/// Upper bound on each project-create and thread-command request, so a hung T3 server fails the
/// calling command instead of blocking it indefinitely
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

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

        // the pid comes from a file, and the binary is run with admin rights, so
        // only T3's own downloaded releases are trusted
        let versions = userdata
            .parent()
            .map(|home| home.join("runtime/versions"))
            .and_then(|dir| fs::canonicalize(dir).ok());

        // the resolved path is what gets run, so a symlink swapped after the
        // check cannot redirect it
        let trusted = match (versions, fs::canonicalize(&bin)) {
            (Some(versions), Ok(real)) if real.starts_with(&versions) => real,
            _ => bail!(
                "refusing to run {}: not a T3 release under runtime/versions",
                bin.display()
            ),
        };

        Ok(Self {
            userdata,
            origin: state.origin,
            bin: trusted,
        })
    }

    pub fn db(&self, read_only: bool) -> Result<Db> {
        let path = self.userdata.join("statev2.sqlite");
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

    /// Short-lived admin token labelled `label`, revoked when the returned client drops
    pub fn client(&self, label: &str, ttl: &str) -> Result<Client> {
        let output = Command::new(&self.bin)
            .args(["auth", "session", "issue", "--json", "--ttl", ttl])
            .args(["--label", label, "--log-level", "error"])
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
    pub fn create_project(&self, root: &Path) -> Result<String> {
        let project_id = uuid::Uuid::new_v4().to_string();
        let title = root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.display().to_string());

        let url = format!("{}/api/projects/mutate", self.origin);
        let token = self.token.clone();
        let mutation = json!({
            "type": "project.create",
            "commandId": uuid::Uuid::new_v4().to_string(),
            "projectId": project_id,
            "title": title,
            "workspaceRoot": root,
            "createWorkspaceRootIfMissing": false,
        });

        let (status, body) = runtime::block_on(async move {
            let response = reqwest::Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .build()?
                .post(url)
                .bearer_auth(token)
                .json(&mutation)
                .send()
                .await?;

            let status = response.status();
            let body = response.text().await?;
            Ok::<_, reqwest::Error>((status, body))
        })??;

        if !status.is_success() {
            bail!("T3 rejected project.create ({status}): {body}");
        }

        Ok(project_id)
    }

    /// Create the thread with the CLI session as its provider thread, so T3
    /// resumes it on the first turn instead of starting a fresh one
    pub fn create_thread(&self, thread: &NewThread, session: &Session) -> Result<()> {
        self.dispatch(json!({
            "type": "thread.create",
            "commandId": uuid::Uuid::new_v4().to_string(),
            "threadId": thread.id,
            "projectId": thread.project_id,
            "title": thread.title,
            "createdBy": "user",
            "creationSource": "server",
            "modelSelection": {
                "instanceId": thread.provider.instance_id(),
                "model": thread.model,
            },
            "runtimeMode": thread.runtime_mode,
            "interactionMode": "default",
            "branch": thread.branch,
            "worktreePath": thread.worktree_path,
            "importedNativeThread": {
                "ref": {
                    "driver": session.provider.instance_id(),
                    "nativeId": session.id,
                    "strength": "strong",
                },
                "metadata": { "title": thread.title },
            },
        }))
    }

    pub fn delete_thread(&self, thread_id: &str) -> Result<()> {
        self.dispatch(json!({
            "type": "thread.delete",
            "commandId": uuid::Uuid::new_v4().to_string(),
            "threadId": thread_id,
        }))
    }

    /// Send one `orchestration.dispatchCommand` request over T3's Effect RPC
    /// socket, which frames each message as a plain JSON object
    fn dispatch(&self, command: Value) -> Result<()> {
        let ws_origin = match self.origin.split_once("://") {
            Some(("https", rest)) => format!("wss://{rest}"),
            Some((_, rest)) => format!("ws://{rest}"),
            None => bail!("T3 origin {} has no scheme", self.origin),
        };

        let mut request = format!("{ws_origin}/ws?orchestrationProtocol={ORCHESTRATION_PROTOCOL}")
            .into_client_request()?;

        request.headers_mut().insert(
            "authorization",
            HeaderValue::from_str(&format!("Bearer {}", self.token))?,
        );

        let frame = json!({
            "_tag": "Request",
            "id": "1",
            "tag": "orchestration.dispatchCommand",
            "payload": command,
            "headers": [],
        });

        let command_type = command["type"].as_str().unwrap_or("command").to_owned();
        let exit = runtime::block_on(async move {
            tokio::time::timeout(REQUEST_TIMEOUT, async move {
                let (mut socket, _) = tokio_tungstenite::connect_async(request).await?;
                socket.send(Message::text(frame.to_string())).await?;

                while let Some(message) = socket.next().await {
                    let Message::Text(text) = message? else {
                        continue;
                    };

                    if let Some(exit) = rpc_exit(&serde_json::from_str(&text)?, "1") {
                        let _ = socket.close(None).await;
                        return Ok(exit);
                    }
                }

                bail!("T3 closed the socket before answering")
            })
            .await
            .map_err(|_| eyre!("T3 did not answer within {}s", REQUEST_TIMEOUT.as_secs()))?
        })??;

        if exit.get("_tag").and_then(Value::as_str) != Some("Success") {
            let detail = exit.get("cause").unwrap_or(&exit);
            bail!("T3 rejected {command_type}: {detail}");
        }

        Ok(())
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

/// The `exit` of the RPC reply to `request_id`, skipping pings and other traffic.
/// The JSON codec may batch messages into an array
fn rpc_exit(message: &Value, request_id: &str) -> Option<Value> {
    let messages = match message {
        Value::Array(messages) => messages.as_slice(),
        single => std::slice::from_ref(single),
    };

    messages.iter().find_map(
        |message| match message.get("_tag").and_then(Value::as_str) {
            Some("Exit")
                if message.get("requestId").and_then(Value::as_str) == Some(request_id) =>
            {
                message.get("exit").cloned()
            }
            Some("Defect") => Some(json!({ "_tag": "Failure", "cause": message.get("defect") })),
            _ => None,
        },
    )
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

pub struct StaleThread {
    pub id: String,
    pub title: String,
    pub updated_at: String,
    pub project: Option<String>,
}

/// T3's v2 state database. The only write is the Claude conversation head on a
/// provider thread this tool just created, which T3 replaces after every turn
pub struct Db(Connection);

impl Db {
    /// Id of the live T3 thread already holding this provider session, whether
    /// imported by this tool, by T3's own importer, or started natively
    pub fn thread_for_session(&self, session: &Session) -> Result<Option<String>> {
        let instance = session.provider.instance_id();

        // T3's built-in importer names its threads after the session
        let builtin_import = format!("import:{instance}:{}", session.id);

        self.0
            .query_row(
                "SELECT thread.thread_id
                 FROM orchestration_v2_projection_threads AS thread
                 WHERE thread.deleted_at IS NULL
                   AND (thread.thread_id = ?3
                        OR EXISTS (
                          SELECT 1 FROM orchestration_v2_projection_provider_threads AS provider
                          WHERE provider.thread_id = thread.thread_id
                            AND provider.driver = ?1
                            AND json_valid(provider.payload_json)
                            AND json_extract(provider.payload_json, '$.nativeThreadRef.nativeId') = ?2
                        ))
                 LIMIT 1",
                params![instance, session.id, builtin_import],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    /// Permission mode of the newest user-created T3 thread on this provider, so
    /// imports follow the user's T3 choice instead of granting a fixed level
    pub fn runtime_mode(&self, provider: Provider) -> Result<String> {
        let mode: Option<String> = self
            .0
            .query_row(
                "SELECT runtime_mode
                 FROM orchestration_v2_projection_threads
                 WHERE provider_instance_id = ?1
                   AND deleted_at IS NULL
                   AND json_valid(payload_json)
                   AND json_extract(payload_json, '$.createdBy') = 'user'
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
                "SELECT json_extract(payload_json, '$.modelSelection.model')
                 FROM orchestration_v2_projection_threads
                 WHERE provider_instance_id = ?1
                   AND json_valid(payload_json)
                   AND json_extract(payload_json, '$.modelSelection.model') IS NOT NULL
                 ORDER BY created_at DESC
                 LIMIT 1",
                [provider.instance_id()],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    /// Live threads that are settled, not archived, and had no activity since
    /// `cutoff` (an ISO timestamp, which T3 stores in a sortable UTC format),
    /// oldest first. Settled means a stored `settledAt`, matching T3's own
    /// settled list, which leaves archived threads out
    pub fn stale_settled_threads(&self, cutoff: &str) -> Result<Vec<StaleThread>> {
        let mut statement = self.0.prepare(
            "SELECT thread.thread_id, thread.title, thread.updated_at, project.title
             FROM orchestration_v2_projection_threads AS thread
             LEFT JOIN projection_projects AS project ON project.project_id = thread.project_id
             WHERE thread.deleted_at IS NULL
               AND thread.archived_at IS NULL
               AND json_valid(thread.payload_json)
               AND json_extract(thread.payload_json, '$.settledAt') IS NOT NULL
               AND thread.updated_at < ?1
             ORDER BY thread.updated_at",
        )?;

        let threads = statement.query_map([cutoff], |row| {
            Ok(StaleThread {
                id: row.get(0)?,
                title: row.get(1)?,
                updated_at: row.get(2)?,
                project: row.get(3)?,
            })
        })?;

        threads.collect::<Result<_, _>>().map_err(Into::into)
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

    /// Point a freshly imported Claude provider thread at the transcript's last
    /// message. `thread.create` always leaves the head empty, and with no T3 turns
    /// behind it the Claude adapter then opens the session with `--session-id`,
    /// which the CLI rejects for an existing session. A head makes it pass
    /// `--resume --resume-session-at` instead. The first completed turn clears it
    pub fn set_claude_head(&self, thread_id: &str, session: &Session) -> Result<()> {
        let Some(head) = &session.head_message_id else {
            bail!(
                "claude session {} has no message to resume from",
                session.id
            );
        };

        let head_ref = json!({
            "driver": session.provider.instance_id(),
            "nativeId": head,
            "strength": "weak",
        });

        // thread.create commits its projections before replying, so the row is there
        let updated = self.0.execute(
            "UPDATE orchestration_v2_projection_provider_threads
             SET payload_json = json_set(payload_json, '$.nativeConversationHeadRef', json(?3))
             WHERE thread_id = ?1
               AND driver = ?4
               AND status = 'not_loaded'
               AND json_valid(payload_json)
               AND json_extract(payload_json, '$.nativeThreadRef.nativeId') = ?2
               AND json_extract(payload_json, '$.nativeConversationHeadRef') IS NULL",
            params![
                thread_id,
                session.id,
                head_ref.to_string(),
                session.provider.instance_id()
            ],
        )?;

        if updated != 1 {
            bail!(
                "T3 thread {thread_id} has no fresh provider thread for claude session {}",
                session.id
            );
        }

        Ok(())
    }
}
