use super::fs::SessionMarkerHandle;
use eyre::{eyre, Result, WrapErr};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{File, OpenOptions};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::thread::JoinHandle;
use std::time::Duration;
use tokio::net::UnixStream;
use tokio_tungstenite::{client_async, tungstenite::Message, WebSocketStream};

const APP_SERVER_START_TIMEOUT: Duration = Duration::from_secs(10);
const APP_SERVER_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const APP_SERVER_CONNECT_ATTEMPT_TIMEOUT: Duration = Duration::from_millis(500);
const APP_SERVER_RETRY_INTERVAL: Duration = Duration::from_millis(50);
const APP_SERVER_STOP_TIMEOUT: Duration = Duration::from_secs(2);
const APP_SERVER_SOCKET_DIR: &str = "app-server-control";
const APP_SERVER_SOCKET_NAME: &str = "app-server-control.sock";
const WRITER_CONFLICT_PREFIX: &str = "thread-store conflict: thread ";
const WRITER_CONFLICT_SUFFIX: &str = " already has an active writer";
#[cfg(unix)]
const MAX_UNIX_SOCKET_PATH_BYTES: usize = 103;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum SessionControl {
    Local {
        socket_path: PathBuf,
    },
    External,
    Embedded,
    #[default]
    Legacy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SessionThread {
    pub(crate) id: String,
    pub(crate) rollout_path: Option<PathBuf>,
    pub(crate) name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SessionMarker {
    #[serde(default)]
    pub(crate) version: u8,
    #[serde(alias = "pid")]
    pub(crate) owner_pid: u32,
    #[serde(default)]
    pub(crate) session_pid: Option<u32>,
    #[serde(default)]
    pub(crate) app_server_pid: Option<u32>,
    pub(crate) started_at: chrono::DateTime<chrono::Utc>,
    pub(crate) launch_home: PathBuf,
    #[serde(default)]
    pub(crate) pane_id: Option<String>,
    #[serde(default)]
    pub(crate) control: SessionControl,
    #[serde(default)]
    pub(crate) current_thread: Option<SessionThread>,
}

impl SessionMarker {
    pub(crate) const VERSION: u8 = 3;

    pub(crate) fn new(
        owner_pid: u32,
        launch_home: PathBuf,
        pane_id: Option<String>,
        control: SessionControl,
    ) -> Self {
        Self {
            version: Self::VERSION,
            owner_pid,
            session_pid: None,
            app_server_pid: None,
            started_at: chrono::Utc::now(),
            launch_home,
            pane_id,
            control,
            current_thread: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AppServerLaunch {
    Managed {
        tui_args: Vec<std::ffi::OsString>,
        config_overrides: Vec<ConfigOverride>,
        strict_config: bool,
    },
    External,
    Embedded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfigOverride {
    pub(crate) key: String,
    pub(crate) value: String,
}

pub(crate) fn plan_app_server_launch(args: &[std::ffi::OsString]) -> AppServerLaunch {
    if has_option(args, |value| {
        value == "--remote"
            || value.starts_with("--remote=")
            || value == "--remote-auth-token-env"
            || value.starts_with("--remote-auth-token-env=")
    }) {
        return AppServerLaunch::External;
    }
    if has_option(args, |value| {
        value == "--profile"
            || value.starts_with("--profile=")
            || value == "-p"
            || (value.starts_with("-p") && value.len() > 2)
            || value == "--dangerously-bypass-hook-trust"
    }) {
        return AppServerLaunch::Embedded;
    }

    let mut tui_args = Vec::with_capacity(args.len());
    let mut config_overrides = Vec::new();
    let mut strict_config = false;
    let mut web_search = false;
    let mut saw_positional = false;
    let mut index = 0;

    while index < args.len() {
        let value = args[index].to_string_lossy();
        if value == "--" {
            tui_args.extend_from_slice(&args[index..]);
            break;
        }
        if value == "--strict-config" {
            strict_config = true;
            index += 1;
            continue;
        }
        if value == "--search" {
            web_search = true;
            index += 1;
            continue;
        }
        if matches!(value.as_ref(), "-c" | "--config" | "--enable" | "--disable") {
            let Some(raw_value) = args.get(index + 1).and_then(|arg| arg.to_str()) else {
                return AppServerLaunch::Embedded;
            };
            let Some(config_override) = config_override(&value, raw_value) else {
                return AppServerLaunch::Embedded;
            };
            config_overrides.push(config_override);
            index += 2;
            continue;
        }
        if let Some(raw_value) = value.strip_prefix("--config=") {
            let Some(config_override) = config_override("--config", raw_value) else {
                return AppServerLaunch::Embedded;
            };
            config_overrides.push(config_override);
            index += 1;
            continue;
        }
        if let Some(raw_value) = value.strip_prefix("--enable=") {
            config_overrides.push(feature_override(raw_value, true));
            index += 1;
            continue;
        }
        if let Some(raw_value) = value.strip_prefix("--disable=") {
            config_overrides.push(feature_override(raw_value, false));
            index += 1;
            continue;
        }
        if let Some(raw_value) = value.strip_prefix("-c").filter(|value| !value.is_empty()) {
            let Some(config_override) = config_override("-c", raw_value) else {
                return AppServerLaunch::Embedded;
            };
            config_overrides.push(config_override);
            index += 1;
            continue;
        }

        if !value.starts_with('-') {
            if !saw_positional && is_noninteractive_subcommand(&value) {
                return AppServerLaunch::Embedded;
            }
            saw_positional = true;
        }

        tui_args.push(args[index].clone());
        if option_takes_one_value(&value) {
            let Some(option_value) = args.get(index + 1) else {
                return AppServerLaunch::Embedded;
            };
            tui_args.push(option_value.clone());
            index += 2;
        } else {
            index += 1;
        }
    }

    if web_search {
        config_overrides.push(ConfigOverride {
            key: "web_search".into(),
            value: "\"live\"".into(),
        });
    }

    AppServerLaunch::Managed {
        tui_args,
        config_overrides,
        strict_config,
    }
}

pub(crate) fn writer_conflict_retry_args(
    args: &[std::ffi::OsString],
    conflict: &ThreadWriterConflict,
) -> Vec<std::ffi::OsString> {
    let had_last = args.iter().any(|arg| arg.to_str() == Some("--last"));
    let mut retry_args = args
        .iter()
        .filter(|arg| arg.to_str() != Some("--last"))
        .cloned()
        .collect::<Vec<_>>();
    let Some(resume_index) = retry_args
        .iter()
        .position(|arg| arg.to_str() == Some("resume"))
    else {
        return retry_args;
    };
    let target_index = (!had_last)
        .then(|| session_target_index(&retry_args, resume_index + 1))
        .flatten();
    let thread_id = std::ffi::OsString::from(conflict.thread_id());
    match target_index {
        Some(target_index) => retry_args[target_index] = thread_id,
        None => retry_args.insert(resume_index + 1, thread_id),
    }

    retry_args
}

fn session_target_index(args: &[std::ffi::OsString], start: usize) -> Option<usize> {
    let mut index = start;
    while index < args.len() {
        let value = args[index].to_string_lossy();
        if value == "--" {
            return (index + 1 < args.len()).then_some(index + 1);
        }
        if option_takes_one_value(&value) {
            index += 2;
            continue;
        }
        if value.starts_with('-') {
            index += 1;
            continue;
        }

        return Some(index);
    }

    None
}

fn has_option(args: &[std::ffi::OsString], predicate: impl Fn(&str) -> bool) -> bool {
    args.iter()
        .map(|arg| arg.to_string_lossy())
        .take_while(|arg| arg != "--")
        .any(|arg| predicate(&arg))
}

fn config_override(flag: &str, raw_value: &str) -> Option<ConfigOverride> {
    match flag {
        "--enable" => Some(feature_override(raw_value, true)),
        "--disable" => Some(feature_override(raw_value, false)),
        "-c" | "--config" => {
            let (key, value) = raw_value.split_once('=')?;
            (!key.trim().is_empty()).then(|| ConfigOverride {
                key: key.trim().to_owned(),
                value: value.to_owned(),
            })
        }
        _ => None,
    }
}

fn feature_override(feature: &str, enabled: bool) -> ConfigOverride {
    ConfigOverride {
        key: format!("features.{feature}"),
        value: enabled.to_string(),
    }
}

fn option_takes_one_value(value: &str) -> bool {
    matches!(
        value,
        "-i" | "--image"
            | "-m"
            | "--model"
            | "--local-provider"
            | "-s"
            | "--sandbox"
            | "-C"
            | "--cd"
            | "--add-dir"
            | "-a"
            | "--ask-for-approval"
    )
}

fn is_noninteractive_subcommand(value: &str) -> bool {
    matches!(
        value,
        "exec"
            | "e"
            | "review"
            | "login"
            | "logout"
            | "mcp"
            | "plugin"
            | "mcp-server"
            | "app-server"
            | "remote-control"
            | "app"
            | "completion"
            | "update"
            | "doctor"
            | "sandbox"
            | "debug"
            | "apply"
            | "a"
            | "archive"
            | "delete"
            | "unarchive"
            | "cloud"
            | "exec-server"
            | "features"
            | "help"
    )
}

pub(crate) fn control_socket_path(launch_home: &Path) -> PathBuf {
    launch_home
        .join(APP_SERVER_SOCKET_DIR)
        .join(APP_SERVER_SOCKET_NAME)
}

fn remote_endpoint(socket_path: &Path) -> std::ffi::OsString {
    let mut endpoint = std::ffi::OsString::from("unix://");
    endpoint.push(socket_path);
    endpoint
}

pub(crate) fn managed_tui_connection_args(
    socket_path: &Path,
    cwd: &Path,
) -> [std::ffi::OsString; 4] {
    [
        "--remote".into(),
        remote_endpoint(socket_path),
        "--cd".into(),
        cwd.into(),
    ]
}

pub(crate) struct ManagedAppServer {
    process: ManagedProcess,
    monitor: SessionMonitor,
    log_path: PathBuf,
}

impl ManagedAppServer {
    pub(crate) fn start(
        launch_home: &Path,
        strict_config: bool,
        marker: SessionMarkerHandle,
        pane_id: Option<String>,
    ) -> Result<Self> {
        let socket_path = control_socket_path(launch_home);
        validate_socket_path(&socket_path)?;
        let log_path = launch_home.join("app-server.log");
        let stdout = app_server_log(&log_path)?;
        let stderr = stdout.try_clone()?;
        let mut command = Command::new("codex");
        command
            .arg("app-server")
            .arg("--listen")
            .arg("unix://")
            .env("CODEX_HOME", launch_home)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));
        if strict_config {
            command.arg("--strict-config");
        }

        let mut process =
            ManagedProcess::spawn(&mut command).wrap_err("Failed to start Codex app server")?;
        if let Err(err) = marker.set_app_server_pid(process.id()) {
            process.stop();
            return Err(err).wrap_err("Failed to record Codex app server process");
        }

        let monitor = match SessionMonitor::start(socket_path, marker, pane_id) {
            Ok(monitor) => monitor,
            Err(err) => {
                process.stop();
                return Err(err).wrap_err_with(|| {
                    format!(
                        "Codex app server failed to become ready; log: {}",
                        log_path.display()
                    )
                });
            }
        };

        Ok(Self {
            process,
            monitor,
            log_path,
        })
    }

    pub(crate) fn log_checkpoint(&self) -> Result<u64> {
        Ok(std::fs::metadata(&self.log_path)?.len())
    }

    pub(crate) fn latest_writer_conflict_since(
        &self,
        checkpoint: u64,
    ) -> Result<Option<ThreadWriterConflict>> {
        let log = std::fs::read_to_string(&self.log_path).wrap_err_with(|| {
            format!(
                "Failed to inspect Codex app-server log at {}",
                self.log_path.display()
            )
        })?;
        let checkpoint = usize::try_from(checkpoint)?;
        let attempt_log = log.get(checkpoint..).ok_or_else(|| {
            eyre!(
                "Codex app-server log became shorter while inspecting {}",
                self.log_path.display()
            )
        })?;

        Ok(parse_latest_writer_conflict(attempt_log))
    }

    pub(crate) fn stop(mut self) {
        self.monitor.stop();
        self.process.stop();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ThreadWriterConflict(String);

impl ThreadWriterConflict {
    pub(crate) fn thread_id(&self) -> &str {
        &self.0
    }
}

fn parse_latest_writer_conflict(log: &str) -> Option<ThreadWriterConflict> {
    log.lines().rev().find_map(|line| {
        let (_, message) = line.rsplit_once(WRITER_CONFLICT_PREFIX)?;
        let thread_id = message.strip_suffix(WRITER_CONFLICT_SUFFIX)?;
        is_uuid(thread_id).then(|| ThreadWriterConflict(thread_id.to_owned()))
    })
}

fn is_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

struct ManagedProcess {
    child: Child,
    #[cfg(unix)]
    process_group_id: u32,
}

impl ManagedProcess {
    fn spawn(command: &mut Command) -> std::io::Result<Self> {
        #[cfg(unix)]
        command.process_group(0);

        let child = command.spawn()?;
        #[cfg(unix)]
        let process_group_id = child.id();

        Ok(Self {
            child,
            #[cfg(unix)]
            process_group_id,
        })
    }

    fn id(&self) -> u32 {
        self.child.id()
    }

    #[cfg(unix)]
    fn stop(&mut self) {
        signal_process_group(self.process_group_id, "TERM").ok();
        if self.wait_for_group_exit(APP_SERVER_STOP_TIMEOUT) {
            return;
        }

        signal_process_group(self.process_group_id, "KILL").ok();
        if self.wait_for_group_exit(APP_SERVER_STOP_TIMEOUT) {
            return;
        }

        self.child.kill().ok();
        self.child.wait().ok();
    }

    #[cfg(not(unix))]
    fn stop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }

    #[cfg(unix)]
    fn wait_for_group_exit(&mut self, timeout: Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        while std::time::Instant::now() < deadline {
            match self.child.try_wait() {
                Ok(_) if !process_group_is_running(self.process_group_id) => return true,
                Ok(_) => std::thread::sleep(APP_SERVER_RETRY_INTERVAL),
                Err(_) => return false,
            }
        }

        !process_group_is_running(self.process_group_id)
    }
}

#[cfg(unix)]
fn signal_process_group(process_group_id: u32, signal: &str) -> std::io::Result<()> {
    let status = Command::new("kill")
        .args([format!("-{signal}"), format!("-{process_group_id}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if status.success() {
        return Ok(());
    }

    Err(std::io::Error::other(format!(
        "failed to send {signal} to process group {process_group_id}"
    )))
}

#[cfg(unix)]
fn process_group_is_running(process_group_id: u32) -> bool {
    Command::new("kill")
        .args(["-0", &format!("-{process_group_id}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(unix)]
fn validate_socket_path(socket_path: &Path) -> Result<()> {
    use std::os::unix::ffi::OsStrExt;

    let path_bytes = socket_path.as_os_str().as_bytes().len();
    if path_bytes > MAX_UNIX_SOCKET_PATH_BYTES {
        return Err(eyre!(
            "Codex app-server socket path is {path_bytes} bytes; maximum is {MAX_UNIX_SOCKET_PATH_BYTES}: {}",
            socket_path.display()
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_socket_path(_socket_path: &Path) -> Result<()> {
    Ok(())
}

fn app_server_log(path: &Path) -> Result<File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .wrap_err_with(|| format!("Failed to open app-server log at {}", path.display()))
}

struct SessionMonitor {
    stop: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

impl SessionMonitor {
    fn start(
        socket_path: PathBuf,
        marker: SessionMarkerHandle,
        pane_id: Option<String>,
    ) -> Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let handle = std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(err) => {
                    ready_tx.send(Err(err.to_string())).ok();
                    return;
                }
            };
            runtime.block_on(run_monitor(
                &socket_path,
                marker,
                pane_id.as_deref(),
                &thread_stop,
                ready_tx,
            ));
        });

        match ready_rx.recv_timeout(APP_SERVER_START_TIMEOUT) {
            Ok(Ok(())) => Ok(Self { stop, handle }),
            Ok(Err(err)) => {
                stop.store(true, Ordering::Relaxed);
                handle.join().ok();
                Err(eyre!(err))
            }
            Err(err) => {
                stop.store(true, Ordering::Relaxed);
                handle.join().ok();
                Err(eyre!("Timed out waiting for Codex app server: {err}"))
            }
        }
    }

    fn stop(self) {
        self.stop.store(true, Ordering::Relaxed);
        self.handle.join().ok();
    }
}

async fn run_monitor(
    socket_path: &Path,
    marker: SessionMarkerHandle,
    pane_id: Option<&str>,
    stop: &AtomicBool,
    ready: mpsc::SyncSender<std::result::Result<(), String>>,
) {
    let mut socket = match connect_initialized_with_retry(socket_path, stop).await {
        Ok(socket) => socket,
        Err(err) => {
            ready.send(Err(err.to_string())).ok();
            return;
        }
    };
    ready.send(Ok(())).ok();

    let mut current_thread_id = None;
    while !stop.load(Ordering::Relaxed) {
        let message = tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(100)) => continue,
            message = socket.next() => message,
        };
        let Some(Ok(message)) = message else {
            return;
        };
        if let Message::Ping(payload) = message {
            socket.send(Message::Pong(payload)).await.ok();
            continue;
        }
        let Some(event) = session_event(message) else {
            continue;
        };

        match event {
            SessionEvent::ThreadStarted(thread) if thread.is_top_level() => {
                current_thread_id = Some(thread.id.clone());
                let session_thread = SessionThread {
                    id: thread.id,
                    rollout_path: thread.path,
                    name: thread.name.clone(),
                };
                marker.set_current_thread(Some(session_thread)).ok();
                sync_pane_name(pane_id, thread.name.as_deref());
            }
            SessionEvent::ThreadNameUpdated { thread_id, name }
                if current_thread_id.as_deref() == Some(thread_id.as_str()) =>
            {
                marker.update_current_thread_name(name.clone()).ok();
                sync_pane_name(pane_id, name.as_deref());
            }
            _ => {}
        }
    }
}

fn sync_pane_name(pane_id: Option<&str>, name: Option<&str>) {
    let Some(pane_id) = pane_id else {
        return;
    };
    let mut command = Command::new("tmux");
    command.args(["set", "-p"]);
    match name.filter(|name| !name.trim().is_empty()) {
        Some(name) => {
            command.args(["-t", pane_id, "@pane_name", name]);
        }
        None => {
            command.args(["-u", "-t", pane_id, "@pane_name"]);
        }
    }
    command
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok();
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppServerThread {
    id: String,
    path: Option<PathBuf>,
    parent_thread_id: Option<String>,
    agent_role: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThreadLoadedListResponse {
    data: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ThreadReadResponse {
    thread: AppServerThread,
}

impl AppServerThread {
    fn is_top_level(&self) -> bool {
        self.parent_thread_id.is_none() && self.agent_role.is_none()
    }
}

enum SessionEvent {
    ThreadStarted(AppServerThread),
    ThreadNameUpdated {
        thread_id: String,
        name: Option<String>,
    },
}

fn session_event(message: Message) -> Option<SessionEvent> {
    let Message::Text(text) = message else {
        return None;
    };
    let message = serde_json::from_str::<RpcMessage>(&text).ok()?;
    match message.method.as_deref()? {
        "thread/started" => {
            #[derive(Deserialize)]
            struct Params {
                thread: AppServerThread,
            }
            let params = serde_json::from_value::<Params>(message.params?).ok()?;
            Some(SessionEvent::ThreadStarted(params.thread))
        }
        "thread/name/updated" => {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Params {
                thread_id: String,
                thread_name: Option<String>,
            }
            let params = serde_json::from_value::<Params>(message.params?).ok()?;
            Some(SessionEvent::ThreadNameUpdated {
                thread_id: params.thread_id,
                name: params.thread_name,
            })
        }
        _ => None,
    }
}

type AppServerSocket = WebSocketStream<UnixStream>;

async fn connect_initialized_with_retry(
    socket_path: &Path,
    stop: &AtomicBool,
) -> Result<AppServerSocket> {
    let deadline = tokio::time::Instant::now() + APP_SERVER_START_TIMEOUT;
    loop {
        let attempt = tokio::time::timeout(
            APP_SERVER_CONNECT_ATTEMPT_TIMEOUT,
            connect_initialized(socket_path),
        )
        .await
        .unwrap_or_else(|_| Err(eyre!("Timed out connecting to Codex app server")));
        match attempt {
            Ok(socket) => return Ok(socket),
            Err(err) if tokio::time::Instant::now() < deadline && !stop.load(Ordering::Relaxed) => {
                let _ = err;
                tokio::time::sleep(APP_SERVER_RETRY_INTERVAL).await;
            }
            Err(err) => return Err(err),
        }
    }
}

async fn connect_initialized(socket_path: &Path) -> Result<AppServerSocket> {
    let stream = UnixStream::connect(socket_path)
        .await
        .wrap_err_with(|| format!("Failed to connect to {}", socket_path.display()))?;
    let (mut socket, _) = client_async("ws://localhost", stream)
        .await
        .wrap_err("Failed to upgrade Codex app-server connection")?;
    send_json(
        &mut socket,
        json!({
            "method": "initialize",
            "id": 1,
            "params": {
                "clientInfo": {
                    "name": "dotfiles_tmux",
                    "title": "Dotfiles tmux integration",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        }),
    )
    .await?;
    read_response(&mut socket, 1).await?;
    send_json(&mut socket, json!({ "method": "initialized" })).await?;
    Ok(socket)
}

pub(crate) fn set_thread_name(socket_path: &Path, thread_id: &str, name: &str) -> Result<()> {
    crate::runtime::block_on(set_thread_name_async(socket_path, thread_id, name))?
}

pub(crate) fn current_loaded_thread(socket_path: &Path) -> Result<Option<SessionThread>> {
    crate::runtime::block_on(current_loaded_thread_async(socket_path))?
}

async fn current_loaded_thread_async(socket_path: &Path) -> Result<Option<SessionThread>> {
    let future = async {
        let mut socket = connect_initialized(socket_path).await?;
        send_json(
            &mut socket,
            json!({
                "method": "thread/loaded/list",
                "id": 2,
                "params": {}
            }),
        )
        .await?;
        let response = serde_json::from_value::<ThreadLoadedListResponse>(
            read_response(&mut socket, 2).await?,
        )
        .wrap_err("Codex app server returned an invalid loaded thread list")?;

        let mut top_level_threads = Vec::new();
        for (index, thread_id) in response.data.into_iter().enumerate() {
            let request_id = i64::try_from(index)? + 3;
            send_json(
                &mut socket,
                json!({
                    "method": "thread/read",
                    "id": request_id,
                    "params": {
                        "threadId": thread_id,
                        "includeTurns": false
                    }
                }),
            )
            .await?;
            let response = serde_json::from_value::<ThreadReadResponse>(
                read_response(&mut socket, request_id).await?,
            )
            .wrap_err("Codex app server returned invalid thread metadata")?;
            if response.thread.is_top_level() {
                top_level_threads.push(response.thread);
            }
        }

        match top_level_threads.as_slice() {
            [] => Ok(None),
            [thread] => Ok(Some(SessionThread {
                id: thread.id.clone(),
                rollout_path: thread.path.clone(),
                name: thread.name.clone(),
            })),
            threads => Err(eyre!(
                "Codex app server reported {} top-level threads; cannot identify the active session",
                threads.len()
            )),
        }
    };
    tokio::time::timeout(APP_SERVER_REQUEST_TIMEOUT, future)
        .await
        .map_err(|_| eyre!("Timed out resolving the active Codex session"))?
}

async fn set_thread_name_async(socket_path: &Path, thread_id: &str, name: &str) -> Result<()> {
    let future = async {
        let mut socket = connect_initialized(socket_path).await?;
        send_json(
            &mut socket,
            json!({
                "method": "thread/name/set",
                "id": 2,
                "params": { "threadId": thread_id, "name": name }
            }),
        )
        .await?;
        read_response(&mut socket, 2).await
    };
    tokio::time::timeout(APP_SERVER_REQUEST_TIMEOUT, future)
        .await
        .map_err(|_| eyre!("Timed out renaming Codex session"))??;
    Ok(())
}

async fn send_json(socket: &mut AppServerSocket, value: Value) -> Result<()> {
    socket
        .send(Message::Text(serde_json::to_string(&value)?.into()))
        .await
        .wrap_err("Failed to send Codex app-server request")
}

async fn read_response(socket: &mut AppServerSocket, expected_id: i64) -> Result<Value> {
    while let Some(message) = socket.next().await {
        let message = message.wrap_err("Failed to read Codex app-server response")?;
        match message {
            Message::Text(text) => {
                let message = serde_json::from_str::<RpcMessage>(&text)
                    .wrap_err("Codex app server returned invalid JSON")?;
                if message.id.as_ref().and_then(Value::as_i64) != Some(expected_id) {
                    continue;
                }
                if let Some(error) = message.error {
                    return Err(eyre!(
                        "Codex app server error {}: {}",
                        error.code,
                        error.message
                    ));
                }
                return message
                    .result
                    .ok_or_else(|| eyre!("Codex app server response had no result"));
            }
            Message::Ping(payload) => {
                socket.send(Message::Pong(payload)).await?;
            }
            Message::Close(_) => break,
            Message::Binary(_) | Message::Pong(_) | Message::Frame(_) => {}
        }
    }
    Err(eyre!("Codex app server closed the connection"))
}

#[derive(Debug, Deserialize)]
struct RpcMessage {
    #[serde(default)]
    id: Option<Value>,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    params: Option<Value>,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<RpcError>,
}

#[derive(Debug, Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

#[cfg(test)]
mod tests {
    use super::{
        current_loaded_thread_async, managed_tui_connection_args, parse_latest_writer_conflict,
        plan_app_server_launch, session_event, set_thread_name_async, validate_socket_path,
        writer_conflict_retry_args, AppServerLaunch, SessionControl, SessionEvent, SessionMarker,
        SessionMonitor, ThreadWriterConflict,
    };
    #[cfg(unix)]
    use super::{process_group_is_running, ManagedProcess};
    use futures_util::{SinkExt, StreamExt};
    use serde_json::{json, Value};
    use std::ffi::OsString;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};
    use tempfile::tempdir;
    use tokio::net::UnixListener;
    use tokio_tungstenite::{accept_async, tungstenite::Message};

    #[test]
    fn latest_writer_conflict_uses_the_last_valid_thread_id() {
        let first = "019fddc8-e0e4-7992-a85d-84d5b50d0068";
        let last = "019fe286-71f0-7513-ba31-fa0433073e6b";
        let log = format!(
            "thread-store conflict: thread {first} already has an active writer\n\
             thread-store conflict: thread not-a-thread already has an active writer\n\
             thread-store conflict: thread {last} already has an active writer\n"
        );

        let conflict = parse_latest_writer_conflict(&log).unwrap();

        assert_eq!(conflict.thread_id(), last);
    }

    #[cfg(unix)]
    #[test]
    fn managed_process_stops_its_child_process_group() {
        let mut command = std::process::Command::new("sh");
        command.args(["-c", "sleep 30 & wait"]);
        let mut process = ManagedProcess::spawn(&mut command).unwrap();
        let process_group_id = process.process_group_id;

        assert!(process_group_is_running(process_group_id));

        process.stop();

        assert!(!process_group_is_running(process_group_id));
    }

    #[test]
    fn writer_conflict_retry_targets_picker_selection_directly() {
        let args = ["resume", "--all"].map(OsString::from);
        let conflict = ThreadWriterConflict("019fe286-71f0-7513-ba31-fa0433073e6b".into());

        let retry = writer_conflict_retry_args(&args, &conflict);

        assert_eq!(
            retry,
            ["resume", "019fe286-71f0-7513-ba31-fa0433073e6b", "--all"].map(OsString::from)
        );
    }

    #[test]
    fn writer_conflict_retry_replaces_last_and_preserves_prompt() {
        let args = ["resume", "--last", "continue here"].map(OsString::from);
        let conflict = ThreadWriterConflict("019fe286-71f0-7513-ba31-fa0433073e6b".into());

        let retry = writer_conflict_retry_args(&args, &conflict);

        assert_eq!(
            retry,
            [
                "resume",
                "019fe286-71f0-7513-ba31-fa0433073e6b",
                "continue here"
            ]
            .map(OsString::from)
        );
    }

    #[test]
    fn writer_conflict_retry_replaces_named_session() {
        let args = ["resume", "session name", "continue here"].map(OsString::from);
        let conflict = ThreadWriterConflict("019fe286-71f0-7513-ba31-fa0433073e6b".into());

        let retry = writer_conflict_retry_args(&args, &conflict);

        assert_eq!(
            retry,
            [
                "resume",
                "019fe286-71f0-7513-ba31-fa0433073e6b",
                "continue here"
            ]
            .map(OsString::from)
        );
    }

    #[test]
    fn launch_plan_extracts_replayable_config() {
        let args = [
            "-c",
            "model=\"gpt-5\"",
            "--enable",
            "web_search",
            "resume",
            "--last",
        ]
        .map(OsString::from);

        let AppServerLaunch::Managed {
            tui_args,
            config_overrides,
            strict_config,
        } = plan_app_server_launch(&args)
        else {
            panic!("expected managed launch");
        };

        assert_eq!(tui_args, ["resume", "--last"].map(OsString::from));
        assert_eq!(config_overrides[0].key, "model");
        assert_eq!(config_overrides[0].value, "\"gpt-5\"");
        assert_eq!(config_overrides[1].key, "features.web_search");
        assert_eq!(config_overrides[1].value, "true");
        assert!(!strict_config);
    }

    #[test]
    fn launch_plan_preserves_external_remote() {
        let args = ["--remote", "unix:///tmp/codex.sock"].map(OsString::from);
        assert_eq!(plan_app_server_launch(&args), AppServerLaunch::External);
    }

    #[test]
    fn launch_plan_keeps_nonreplayable_profile_launch_embedded() {
        let args = ["--profile", "work"].map(OsString::from);
        assert_eq!(plan_app_server_launch(&args), AppServerLaunch::Embedded);
    }

    #[test]
    fn launch_plan_extracts_config_after_prompt() {
        let args = ["start here", "-c", "model=\"gpt-5\""].map(OsString::from);
        let AppServerLaunch::Managed {
            tui_args,
            config_overrides,
            ..
        } = plan_app_server_launch(&args)
        else {
            panic!("expected managed launch");
        };

        assert_eq!(tui_args, ["start here"].map(OsString::from));
        assert_eq!(config_overrides[0].key, "model");
    }

    #[test]
    fn launch_plan_materializes_search_flag() {
        let args = ["--search", "resume", "--last"].map(OsString::from);
        let AppServerLaunch::Managed {
            tui_args,
            config_overrides,
            ..
        } = plan_app_server_launch(&args)
        else {
            panic!("expected managed launch");
        };

        assert_eq!(tui_args, ["resume", "--last"].map(OsString::from));
        assert_eq!(config_overrides[0].key, "web_search");
        assert_eq!(config_overrides[0].value, "\"live\"");
    }

    #[test]
    fn unix_socket_path_limit_is_checked_before_launch() {
        let long_path = std::path::PathBuf::from("/").join("x".repeat(104));
        assert!(validate_socket_path(&long_path).is_err());
    }

    #[test]
    fn managed_tui_connection_includes_socket_and_launch_cwd() {
        assert_eq!(
            managed_tui_connection_args(
                std::path::Path::new("/tmp/codex.sock"),
                std::path::Path::new("/tmp/project"),
            ),
            ["--remote", "unix:///tmp/codex.sock", "--cd", "/tmp/project",].map(OsString::from)
        );
    }

    #[test]
    fn thread_started_filters_subagents() {
        let message = Message::Text(
            json!({
                "method": "thread/started",
                "params": { "thread": {
                    "id": "thread-1",
                    "path": "/tmp/rollout.jsonl",
                    "source": "cli",
                    "parentThreadId": "parent",
                    "agentRole": "worker",
                    "name": null
                }}
            })
            .to_string()
            .into(),
        );

        let Some(SessionEvent::ThreadStarted(thread)) = session_event(message) else {
            panic!("expected thread event");
        };
        assert!(!thread.is_top_level());
    }

    #[test]
    fn thread_started_accepts_top_level_remote_tui_sessions() {
        let message = Message::Text(
            json!({
                "method": "thread/started",
                "params": { "thread": {
                    "id": "thread-1",
                    "path": "/tmp/rollout.jsonl",
                    "source": "vscode",
                    "parentThreadId": null,
                    "agentRole": null,
                    "name": null
                }}
            })
            .to_string()
            .into(),
        );

        let Some(SessionEvent::ThreadStarted(thread)) = session_event(message) else {
            panic!("expected thread event");
        };
        assert!(thread.is_top_level());
    }

    #[test]
    fn thread_name_updated_parses_optional_name() {
        let message = Message::Text(
            json!({
                "method": "thread/name/updated",
                "params": { "threadId": "thread-1", "threadName": "Live name" }
            })
            .to_string()
            .into(),
        );

        let Some(SessionEvent::ThreadNameUpdated { thread_id, name }) = session_event(message)
        else {
            panic!("expected thread name event");
        };
        assert_eq!(thread_id, "thread-1");
        assert_eq!(name.as_deref(), Some("Live name"));
    }

    #[tokio::test]
    async fn thread_name_set_uses_initialized_websocket_connection() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("control.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            let initialize = next_json(&mut socket).await;
            assert_eq!(initialize["method"], "initialize");
            socket
                .send(Message::Text(
                    json!({"id": 1, "result": {}}).to_string().into(),
                ))
                .await
                .unwrap();
            let initialized = next_json(&mut socket).await;
            assert_eq!(initialized, json!({"method": "initialized"}));
            let rename = next_json(&mut socket).await;
            assert_eq!(rename["method"], "thread/name/set");
            assert_eq!(rename["params"]["threadId"], "thread-1");
            assert_eq!(rename["params"]["name"], "Live name");
            socket
                .send(Message::Text(
                    json!({"id": 2, "result": {}}).to_string().into(),
                ))
                .await
                .unwrap();
        });

        set_thread_name_async(&socket_path, "thread-1", "Live name")
            .await
            .unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn thread_name_set_surfaces_protocol_errors() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("control.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            let _ = next_json(&mut socket).await;
            socket
                .send(Message::Text(
                    json!({"id": 1, "result": {}}).to_string().into(),
                ))
                .await
                .unwrap();
            let _ = next_json(&mut socket).await;
            let _ = next_json(&mut socket).await;
            socket
                .send(Message::Text(
                    json!({
                        "id": 2,
                        "error": {"code": -32602, "message": "thread not found"}
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();
        });

        let error = set_thread_name_async(&socket_path, "missing", "Live name")
            .await
            .unwrap_err();
        assert!(error.to_string().contains("thread not found"));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn current_loaded_thread_ignores_subagents() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("control.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            let _ = next_json(&mut socket).await;
            socket
                .send(Message::Text(
                    json!({"id": 1, "result": {}}).to_string().into(),
                ))
                .await
                .unwrap();
            let _ = next_json(&mut socket).await;

            let loaded = next_json(&mut socket).await;
            assert_eq!(loaded["method"], "thread/loaded/list");
            socket
                .send(Message::Text(
                    json!({
                        "id": 2,
                        "result": {"data": ["thread-1", "thread-2"]}
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();

            for (request_id, thread) in [
                (
                    3,
                    json!({
                        "id": "thread-1",
                        "path": "/tmp/root-rollout.jsonl",
                        "parentThreadId": null,
                        "agentRole": null,
                        "name": "Root thread"
                    }),
                ),
                (
                    4,
                    json!({
                        "id": "thread-2",
                        "path": "/tmp/child-rollout.jsonl",
                        "parentThreadId": "thread-1",
                        "agentRole": "worker",
                        "name": "Child thread"
                    }),
                ),
            ] {
                let read = next_json(&mut socket).await;
                assert_eq!(read["method"], "thread/read");
                socket
                    .send(Message::Text(
                        json!({"id": request_id, "result": {"thread": thread}})
                            .to_string()
                            .into(),
                    ))
                    .await
                    .unwrap();
            }
        });

        let thread = current_loaded_thread_async(&socket_path)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(thread.id, "thread-1");
        assert_eq!(
            thread.rollout_path.as_deref(),
            Some(std::path::Path::new("/tmp/root-rollout.jsonl"))
        );
        assert_eq!(thread.name.as_deref(), Some("Root thread"));
        server.await.unwrap();
    }

    #[test]
    fn monitor_tracks_top_level_thread_and_name_notifications() {
        let dir = tempdir().unwrap();
        let profile_home = dir.path().join("profiles/a");
        let socket_path = dir.path().join("control.sock");
        let (server_ready_tx, server_ready_rx) = mpsc::sync_channel(1);
        let server_socket_path = socket_path.clone();
        let server = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                let listener = UnixListener::bind(&server_socket_path).unwrap();
                server_ready_tx.send(()).unwrap();
                let (stream, _) = listener.accept().await.unwrap();
                let mut socket = accept_async(stream).await.unwrap();
                let _ = next_json(&mut socket).await;
                socket
                    .send(Message::Text(
                        json!({"id": 1, "result": {}}).to_string().into(),
                    ))
                    .await
                    .unwrap();
                let _ = next_json(&mut socket).await;
                socket
                    .send(Message::Text(
                        json!({
                            "method": "thread/started",
                            "params": {"thread": {
                                "id": "thread-1",
                                "path": "/tmp/rollout.jsonl",
                                "source": "cli",
                                "parentThreadId": null,
                                "agentRole": null,
                                "name": null
                            }}
                        })
                        .to_string()
                        .into(),
                    ))
                    .await
                    .unwrap();
                socket
                    .send(Message::Text(
                        json!({
                            "method": "thread/name/updated",
                            "params": {"threadId": "thread-1", "threadName": "Live name"}
                        })
                        .to_string()
                        .into(),
                    ))
                    .await
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(300)).await;
            });
        });
        server_ready_rx.recv().unwrap();
        let marker = super::super::fs::write_session_marker(
            &profile_home,
            std::process::id(),
            dir.path(),
            None,
            SessionControl::Local {
                socket_path: socket_path.clone(),
            },
        )
        .unwrap();

        let monitor = SessionMonitor::start(socket_path, marker.clone(), None).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let current_thread = loop {
            let saved =
                serde_json::from_slice::<SessionMarker>(&std::fs::read(marker.path()).unwrap())
                    .unwrap();
            if saved
                .current_thread
                .as_ref()
                .is_some_and(|thread| thread.name.as_deref() == Some("Live name"))
            {
                break saved.current_thread.unwrap();
            }
            assert!(Instant::now() < deadline, "monitor did not update marker");
            std::thread::sleep(Duration::from_millis(10));
        };

        assert_eq!(current_thread.id, "thread-1");
        assert_eq!(
            current_thread.rollout_path.as_deref(),
            Some(std::path::Path::new("/tmp/rollout.jsonl"))
        );
        monitor.stop();
        server.join().unwrap();
    }

    async fn next_json(
        socket: &mut tokio_tungstenite::WebSocketStream<tokio::net::UnixStream>,
    ) -> Value {
        let Message::Text(text) = socket.next().await.unwrap().unwrap() else {
            panic!("expected text websocket message");
        };
        serde_json::from_str(&text).unwrap()
    }
}
