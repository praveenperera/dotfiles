use super::app_server::{
    control_socket_path, ConfigOverride, SessionControl, SessionMarker, SessionThread,
};
use super::*;
use crate::fsutil;
use rusqlite::{Connection, OpenFlags};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration as StdDuration;

const SHARED_GROUP_NAME: &str = "shared";
const STALE_PROCESS_STOP_TIMEOUT: StdDuration = StdDuration::from_secs(2);
const STALE_PROCESS_POLL_INTERVAL: StdDuration = StdDuration::from_millis(50);
const SEED_SNAPSHOT_BUSY_TIMEOUT: StdDuration = StdDuration::from_secs(5);
// a new group is built as `.<group>.creating-<pid>-<random>` beside its final path, and the pid
// tells a later launch whether the builder is still alive
const GROUP_STAGING_MARKER: &str = ".creating-";
// codex keys every row in these databases by thread, so they must live beside the resume group's
// sessions; names carry a schema version (`state_5`, `memories_v2_1`) that codex bumps over time
const RESUME_GROUP_DB_FAMILIES: [&str; 7] = [
    "state",
    "logs",
    "thread_history",
    "goals",
    "queue",
    "memories",
    "memories_v2",
];
const SQLITE_SIDECAR_SUFFIXES: [&str; 3] = ["-wal", "-shm", "-journal"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ThreadWriterRecovery {
    CurrentSession,
    Ready {
        stopped_pids: Vec<u32>,
    },
    Active {
        pid: u32,
        pane_id: Option<String>,
        thread_name: Option<String>,
    },
    Unmanaged {
        pids: Vec<u32>,
    },
}

struct SessionMarkerRecord {
    path: PathBuf,
    marker: SessionMarker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupKind {
    Config,
    Resume,
}

/// A versioned Codex runtime SQLite file such as `goals_1.sqlite` or `state_5.sqlite-wal`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RuntimeDb<'a> {
    family: &'a str,
    sidecar: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryOwner {
    LocalAuth,
    Ignored,
    SharedStatic,
    ConfigGroup,
    ResumeGroup,
}

pub(super) fn codex_dir() -> Result<PathBuf> {
    Ok(fsutil::home_dir()?.join(".codex"))
}

pub(super) fn profiles_dir() -> Result<PathBuf> {
    Ok(codex_dir()?.join("profiles"))
}

pub(super) fn profile_codex_home(profile: &str) -> Result<PathBuf> {
    Ok(profiles_dir()?.join(profile))
}

pub(super) fn validate_group_name(group: &str, kind: &str) -> Result<()> {
    if group.is_empty() {
        return Err(eyre!("{kind} group name cannot be empty"));
    }
    if matches!(group, "." | "..") {
        return Err(eyre!("{kind} group name cannot be '.' or '..'"));
    }
    if group.contains('/') || group.contains('\\') {
        return Err(eyre!("{kind} group name cannot contain path separators"));
    }
    // stale staging cleanup deletes dirs with this marker, so a real group must never carry it
    if group.contains(GROUP_STAGING_MARKER) {
        return Err(eyre!(
            "{kind} group name cannot contain '{GROUP_STAGING_MARKER}'"
        ));
    }

    Ok(())
}

fn remodex_state_dir() -> Result<PathBuf> {
    Ok(std::env::var_os("REMODEX_DEVICE_STATE_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or(fsutil::home_dir()?.join(".remodex")))
}

fn remodex_bridge_control_socket_path() -> Result<PathBuf> {
    Ok(remodex_state_dir()?.join("bridge-control.sock"))
}

fn launches_dir(profile_home: &Path) -> Result<PathBuf> {
    let profiles_dir = profile_home
        .parent()
        .ok_or_else(|| eyre!("Profile home has no profiles directory"))?;
    let codex_home = profiles_dir
        .parent()
        .ok_or_else(|| eyre!("Profiles directory has no Codex home"))?;
    Ok(codex_home.join("l"))
}

pub(super) fn create_launch_home(profile_home: &Path) -> Result<PathBuf> {
    let launches_dir = launches_dir(profile_home)?;
    stdfs::create_dir_all(&launches_dir)?;

    for attempt in 0..10 {
        let timestamp = Utc::now().timestamp_millis();
        let suffix = if attempt == 0 {
            std::process::id().to_string()
        } else {
            format!("{}-{attempt}", std::process::id())
        };
        let launch_home = launches_dir.join(format!("{timestamp}-{suffix}"));

        match stdfs::create_dir(&launch_home) {
            Ok(()) => return Ok(launch_home),
            Err(err) if err.kind() == ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err.into()),
        }
    }

    Err(eyre!(
        "Failed to create a unique launch home in {}",
        launches_dir.display()
    ))
}

pub(super) fn auth_path() -> Result<PathBuf> {
    Ok(codex_dir()?.join("auth.json"))
}

pub(super) fn profile_session_markers_dir(profile_home: &Path) -> PathBuf {
    profile_home.join(".session-markers")
}

pub(super) fn write_session_marker(
    profile_home: &Path,
    owner_pid: u32,
    launch_home: &Path,
    pane_id: Option<String>,
    control: SessionControl,
) -> Result<SessionMarkerHandle> {
    let markers_dir = profile_session_markers_dir(profile_home);
    stdfs::create_dir_all(&markers_dir)?;
    let marker_path = markers_dir.join(format!("{owner_pid}.json"));
    let marker = SessionMarker::new(owner_pid, launch_home.to_path_buf(), pane_id, control);
    stdfs::write(&marker_path, serde_json::to_vec_pretty(&marker)?)?;
    Ok(SessionMarkerHandle {
        path: marker_path,
        write_lock: Arc::new(Mutex::new(())),
    })
}

#[derive(Clone)]
pub(crate) struct SessionMarkerHandle {
    path: PathBuf,
    write_lock: Arc<Mutex<()>>,
}

impl SessionMarkerHandle {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn set_session_pid(&self, session_pid: u32) -> Result<bool> {
        self.update(|marker| marker.session_pid = Some(session_pid))
    }

    pub(crate) fn set_app_server_pid(&self, app_server_pid: u32) -> Result<bool> {
        self.update(|marker| marker.app_server_pid = Some(app_server_pid))
    }

    pub(crate) fn set_current_thread(&self, thread: Option<SessionThread>) -> Result<bool> {
        self.update(|marker| marker.current_thread = thread)
    }

    pub(crate) fn update_current_thread_name(&self, name: Option<String>) -> Result<bool> {
        self.update(|marker| {
            if let Some(thread) = marker.current_thread.as_mut() {
                thread.name = name;
            }
        })
    }

    fn update(&self, update: impl FnOnce(&mut SessionMarker)) -> Result<bool> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| eyre!("Session marker lock is poisoned"))?;
        if !self.path.exists() {
            return Ok(false);
        }

        let mut marker = read_session_marker(&self.path)?;
        update(&mut marker);
        write_session_marker_atomic(&self.path, &marker)?;
        Ok(true)
    }
}

fn write_session_marker_atomic(marker_path: &Path, marker: &SessionMarker) -> Result<()> {
    let parent = marker_path
        .parent()
        .ok_or_else(|| eyre!("Session marker path has no parent"))?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer_pretty(&mut temp, marker)?;
    temp.persist(marker_path)?;
    Ok(())
}

pub(super) fn materialize_config_overrides(
    launch_home: &Path,
    overrides: &[ConfigOverride],
) -> Result<()> {
    if overrides.is_empty() {
        return Ok(());
    }

    let config_path = launch_home.join("config.toml");
    let mut config = if config_path.exists() {
        toml::from_str::<toml::Value>(&stdfs::read_to_string(&config_path)?)?
    } else {
        toml::Value::Table(toml::Table::new())
    };
    for config_override in overrides {
        apply_config_override(&mut config, config_override)?;
    }

    fsutil::remove_existing_path(&config_path)?;
    stdfs::write(&config_path, toml::to_string_pretty(&config)?)?;
    Ok(())
}

fn apply_config_override(config: &mut toml::Value, config_override: &ConfigOverride) -> Result<()> {
    let source = format!("{} = {}", config_override.key, config_override.value);
    let parsed = match toml::from_str::<toml::Value>(&source) {
        Ok(parsed) => parsed,
        Err(_) => toml::from_str::<toml::Value>(&format!(
            "{} = {}",
            config_override.key,
            serde_json::to_string(&config_override.value)?
        ))
        .wrap_err_with(|| format!("Invalid Codex config override: {}", config_override.key))?,
    };
    merge_toml(config, parsed);
    Ok(())
}

fn merge_toml(target: &mut toml::Value, overlay: toml::Value) {
    match (target, overlay) {
        (toml::Value::Table(target), toml::Value::Table(overlay)) => {
            for (key, value) in overlay {
                if let Some(target_value) = target.get_mut(&key) {
                    merge_toml(target_value, value);
                } else {
                    target.insert(key, value);
                }
            }
        }
        (target, overlay) => *target = overlay,
    }
}

pub(super) fn active_session_markers(profile_home: &Path) -> Result<Vec<SessionMarker>> {
    let markers_dir = profile_session_markers_dir(profile_home);
    if !markers_dir.exists() {
        return Ok(Vec::new());
    }

    let mut active = Vec::new();
    for entry in stdfs::read_dir(&markers_dir)? {
        let entry = entry?;
        let path = entry.path();
        let marker = match read_session_marker(&path) {
            Ok(marker) => marker,
            Err(_) => {
                fsutil::remove_existing_path(&path)?;
                continue;
            }
        };

        if session_marker_is_active(&marker)? {
            active.push(marker);
        } else {
            stop_stale_app_server(&marker)?;
            fsutil::remove_existing_path(&path)?;
        }
    }

    Ok(active)
}

fn read_session_marker(path: &Path) -> Result<SessionMarker> {
    Ok(serde_json::from_slice(&stdfs::read(path)?)?)
}

fn session_marker_is_active(marker: &SessionMarker) -> Result<bool> {
    if process_is_running(marker.owner_pid)? {
        return Ok(true);
    }

    marker
        .session_pid
        .map(process_is_running)
        .transpose()
        .map(Option::unwrap_or_default)
}

fn process_is_running(pid: u32) -> Result<bool> {
    Ok(std::process::Command::new("ps")
        .args(["-p", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?
        .success())
}

fn stop_stale_app_server(marker: &SessionMarker) -> Result<()> {
    let SessionControl::Local { socket_path } = &marker.control else {
        return Ok(());
    };

    let expected_socket_path = control_socket_path(&marker.launch_home);
    if socket_path != &expected_socket_path {
        return Err(eyre!(
            "Refusing to stop stale Codex app server: unexpected control socket {}",
            socket_path.display()
        ));
    }

    let socket_owners = pids_holding_path(socket_path)?;
    for pid in socket_owners {
        stop_process(pid)?;
        println!("Stopped stale Codex app server (pid {pid})");
    }

    Ok(())
}

pub(super) fn recover_thread_writer(
    launch_home: &Path,
    profiles_root: &Path,
    thread_id: &str,
    policy: ThreadWriterPolicy,
    current_server_pid: Option<u32>,
) -> Result<ThreadWriterRecovery> {
    let lock_path = launch_home
        .join("thread-writer-locks")
        .join(format!("{thread_id}.lock"));
    let holder_pids = pids_holding_path(&lock_path)?;
    // a retry can succeed before its earlier conflict is read from the log
    if current_server_pid.is_some_and(|pid| holder_pids.contains(&pid)) {
        return Ok(ThreadWriterRecovery::CurrentSession);
    }

    if holder_pids.is_empty() {
        return Ok(ThreadWriterRecovery::Ready {
            stopped_pids: Vec::new(),
        });
    }

    if policy == ThreadWriterPolicy::StopConflicting {
        let mut stopped_pids = Vec::with_capacity(holder_pids.len());
        for pid in holder_pids {
            let marker_path = find_writer_session_marker(profiles_root, thread_id, pid)?
                .map(|record| record.path);

            stop_process(pid)?;
            if let Some(marker_path) = marker_path {
                fsutil::remove_existing_path(&marker_path)?;
            }

            stopped_pids.push(pid);
        }

        return Ok(ThreadWriterRecovery::Ready { stopped_pids });
    }

    let mut stale_writers = Vec::new();
    let mut unmanaged_pids = Vec::new();
    for pid in holder_pids {
        let Some(record) = find_writer_session_marker(profiles_root, thread_id, pid)? else {
            unmanaged_pids.push(pid);
            continue;
        };
        if managed_tui_is_running(&record.marker)? {
            return Ok(ThreadWriterRecovery::Active {
                pid,
                pane_id: record.marker.pane_id,
                thread_name: record.marker.current_thread.and_then(|thread| thread.name),
            });
        }

        stale_writers.push((pid, record.path));
    }

    if !unmanaged_pids.is_empty() {
        return Ok(ThreadWriterRecovery::Unmanaged {
            pids: unmanaged_pids,
        });
    }

    let mut stopped_pids = Vec::with_capacity(stale_writers.len());
    for (pid, marker_path) in stale_writers {
        stop_process(pid)?;
        fsutil::remove_existing_path(&marker_path)?;
        stopped_pids.push(pid);
    }

    Ok(ThreadWriterRecovery::Ready { stopped_pids })
}

fn find_writer_session_marker(
    profiles_root: &Path,
    thread_id: &str,
    writer_pid: u32,
) -> Result<Option<SessionMarkerRecord>> {
    if !profiles_root.exists() {
        return Ok(None);
    }

    for profile_entry in stdfs::read_dir(profiles_root)? {
        let profile_entry = profile_entry?;
        let markers_dir = profile_session_markers_dir(&profile_entry.path());
        if !markers_dir.exists() {
            continue;
        }

        for marker_entry in stdfs::read_dir(markers_dir)? {
            let marker_entry = marker_entry?;
            let path = marker_entry.path();
            let Ok(marker) = read_session_marker(&path) else {
                continue;
            };
            if marker
                .current_thread
                .as_ref()
                .map(|thread| thread.id.as_str())
                != Some(thread_id)
            {
                continue;
            }
            let SessionControl::Local { socket_path } = &marker.control else {
                continue;
            };
            if socket_path != &control_socket_path(&marker.launch_home) {
                continue;
            }
            if process_owns_socket(writer_pid, socket_path)? {
                return Ok(Some(SessionMarkerRecord { path, marker }));
            }
        }
    }

    Ok(None)
}

fn managed_tui_is_running(marker: &SessionMarker) -> Result<bool> {
    let SessionControl::Local { socket_path } = &marker.control else {
        return Ok(false);
    };
    let Some(session_pid) = marker.session_pid else {
        return Ok(false);
    };
    let Some(command) = process_command(session_pid)? else {
        return Ok(false);
    };

    Ok(command.contains("--remote")
        && command.contains(&socket_path.to_string_lossy().into_owned()))
}

fn process_command(pid: u32) -> Result<Option<String>> {
    let output = std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "command="])
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }

    Ok(Some(
        String::from_utf8_lossy(&output.stdout).trim().to_owned(),
    ))
}

fn pids_holding_path(path: &Path) -> Result<Vec<u32>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let output = std::process::Command::new("lsof")
        .args(["-nP", "-t"])
        .arg(path)
        .output()
        .wrap_err_with(|| format!("Failed to inspect {}", path.display()))?;
    let mut pids = BTreeSet::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let pid = line.trim().parse::<u32>().wrap_err_with(|| {
            format!("lsof returned an invalid process id for {}", path.display())
        })?;
        pids.insert(pid);
    }

    Ok(pids.into_iter().collect())
}

fn stop_process(pid: u32) -> Result<()> {
    if !process_is_running(pid)? {
        return Ok(());
    }

    signal_process(pid, "TERM")?;
    if wait_for_process_exit(pid, STALE_PROCESS_STOP_TIMEOUT)? {
        return Ok(());
    }

    signal_process(pid, "KILL")?;
    if !wait_for_process_exit(pid, STALE_PROCESS_STOP_TIMEOUT)? {
        return Err(eyre!("Failed to stop Codex app server {pid}"));
    }

    Ok(())
}

fn process_owns_socket(pid: u32, socket_path: &Path) -> Result<bool> {
    let status = std::process::Command::new("lsof")
        .args(["-nP", "-a", "-p", &pid.to_string(), "-U"])
        .arg(socket_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .wrap_err("Failed to inspect Codex app server")?;
    Ok(status.success())
}

fn signal_process(pid: u32, signal: &str) -> Result<()> {
    let status = std::process::Command::new("kill")
        .args([format!("-{signal}"), pid.to_string()])
        .status()
        .wrap_err_with(|| format!("Failed to send {signal} to Codex app server {pid}"))?;
    if status.success() || !process_is_running(pid)? {
        return Ok(());
    }

    Err(eyre!("Failed to send {signal} to Codex app server {pid}"))
}

fn wait_for_process_exit(pid: u32, timeout: StdDuration) -> Result<bool> {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if !process_is_running(pid)? {
            return Ok(true);
        }

        thread::sleep(STALE_PROCESS_POLL_INTERVAL);
    }

    Ok(!process_is_running(pid)?)
}

pub(super) fn codex_command(codex_home: &Path) -> std::process::Command {
    let mut command = std::process::Command::new("codex");
    command.env("CODEX_HOME", codex_home);
    command
}

pub(super) fn sync_login_codex_home(login_home: &Path, shared_codex_home: &Path) -> Result<()> {
    stdfs::create_dir_all(login_home)?;

    for entry in stdfs::read_dir(shared_codex_home)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };

        if !matches!(
            entry_owner(name),
            EntryOwner::SharedStatic | EntryOwner::ConfigGroup | EntryOwner::ResumeGroup
        ) {
            continue;
        }

        sync_shared_entry(&entry.path(), &login_home.join(name))?;
    }

    Ok(())
}

pub(super) fn prepare_config_group_home(shared_codex_home: &Path, group: &str) -> Result<PathBuf> {
    prepare_group_home(
        shared_codex_home,
        &shared_codex_home.join("config-groups"),
        group,
        GroupKind::Config,
    )
}

pub(super) fn prepare_resume_group_home(shared_codex_home: &Path, group: &str) -> Result<PathBuf> {
    prepare_group_home(
        shared_codex_home,
        &shared_codex_home.join("resume-groups"),
        group,
        GroupKind::Resume,
    )
}

fn prepare_group_home(
    shared_codex_home: &Path,
    groups_dir: &Path,
    group: &str,
    kind: GroupKind,
) -> Result<PathBuf> {
    validate_group_name(group, kind.label())?;
    if group == SHARED_GROUP_NAME {
        return Ok(shared_codex_home.to_path_buf());
    }

    let group_home = groups_dir.join(group);
    if !path_exists(&group_home)? {
        remove_abandoned_group_stagings(groups_dir)?;
        let staging = stage_group_home(shared_codex_home, groups_dir, group, kind)?;
        publish_group_home(staging, &group_home)?;
    }

    seed_group_home(shared_codex_home, &group_home, kind, false)?;
    Ok(group_home)
}

/// Builds a complete new group, database snapshots included, in a hidden sibling directory
///
/// The directory is removed when the returned handle drops, so a failed seed leaves nothing that
/// a later launch could mistake for a finished group
fn stage_group_home(
    shared_codex_home: &Path,
    groups_dir: &Path,
    group: &str,
    kind: GroupKind,
) -> Result<tempfile::TempDir> {
    stdfs::create_dir_all(groups_dir)?;
    let staging = tempfile::Builder::new()
        .prefix(&group_staging_prefix(group, std::process::id()))
        .tempdir_in(groups_dir)?;
    seed_group_home(shared_codex_home, staging.path(), kind, true)?;
    Ok(staging)
}

/// Moves a staged group into place with one rename, or discards it when another launch won
fn publish_group_home(staging: tempfile::TempDir, group_home: &Path) -> Result<()> {
    // without a no-replace rename, rename(2) still replaces an existing directory when it is
    // empty; that only happens when nothing has been written into the winner's group yet, so no
    // launch can be holding a file inside it
    match stdfs::rename(staging.path(), group_home) {
        Ok(()) => {
            let _ = staging.keep();
            Ok(())
        }
        Err(_) if path_exists(group_home)? => Ok(()),
        Err(err) => Err(err).wrap_err_with(|| {
            format!(
                "Failed to publish {} as {}",
                staging.path().display(),
                group_home.display()
            )
        }),
    }
}

fn group_staging_prefix(group: &str, pid: u32) -> String {
    format!(".{group}{GROUP_STAGING_MARKER}{pid}-")
}

/// Removes staging directories whose creator process has exited
///
/// A creator that is still running owns its directory and may publish it at any moment, so only
/// directories whose embedded pid no longer runs are removed
fn remove_abandoned_group_stagings(groups_dir: &Path) -> Result<()> {
    if !groups_dir.exists() {
        return Ok(());
    }

    for entry in stdfs::read_dir(groups_dir)? {
        let entry = entry?;
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let Some(pid) = group_staging_pid(&name) else {
            continue;
        };

        // a reused pid keeps the directory until the unrelated process exits, which is safe
        if process_is_running(pid)? {
            continue;
        }

        // another launch may be removing the same directory concurrently
        match fsutil::remove_existing_path(&entry.path()) {
            Err(err) if path_exists(&entry.path())? => return Err(err),
            _ => {}
        }
    }

    Ok(())
}

fn group_staging_pid(name: &str) -> Option<u32> {
    let (_, rest) = name.strip_prefix('.')?.rsplit_once(GROUP_STAGING_MARKER)?;
    let (pid, _) = rest.split_once('-')?;
    pid.parse().ok()
}

fn seed_group_home(
    shared_codex_home: &Path,
    group_home: &Path,
    kind: GroupKind,
    staged: bool,
) -> Result<()> {
    for entry in stdfs::read_dir(shared_codex_home)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };

        if !kind.seeds(name, entry_owner(name)) {
            continue;
        }

        let target = group_home.join(name);
        if path_exists(&target)? {
            continue;
        }

        // databases are forked only with the group itself; an existing group that lacks one gets
        // a fresh database from codex rather than another group's rows for threads it never had
        match runtime_db(name) {
            Some(db) if staged && db.seeds_by_snapshot() => {
                snapshot_sqlite_db(&entry.path(), &target)?;
            }
            Some(_) => {}
            None => copy_seed_entry(&entry.path(), &target)?,
        }
    }

    Ok(())
}

pub(super) fn sync_launch_codex_home(
    launch_home: &Path,
    shared_codex_home: &Path,
    launch_auth_mode: &LaunchAuthMode,
    config_home: &Path,
    resume_home: &Path,
) -> Result<()> {
    let mut entry_names = BTreeSet::new();
    collect_entry_names(shared_codex_home, &mut entry_names)?;
    collect_entry_names(config_home, &mut entry_names)?;
    collect_entry_names(resume_home, &mut entry_names)?;

    for name in entry_names {
        let Some(source_root) =
            entry_source_root(&name, shared_codex_home, config_home, resume_home)
        else {
            continue;
        };

        let source = source_root.join(&name);
        if !path_exists(&source)? && !links_missing_source(&name) {
            continue;
        }

        sync_shared_entry(&source, &launch_home.join(&name))?;
    }

    sync_launch_auth(launch_auth_mode, &launch_home.join("auth.json"))
}

fn collect_entry_names(root: &Path, entry_names: &mut BTreeSet<String>) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }

    for entry in stdfs::read_dir(root)? {
        let entry = entry?;
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        entry_names.insert(name);
    }

    Ok(())
}

fn entry_source_root<'a>(
    name: &str,
    shared_codex_home: &'a Path,
    config_home: &'a Path,
    resume_home: &'a Path,
) -> Option<&'a Path> {
    match entry_owner(name) {
        EntryOwner::LocalAuth | EntryOwner::Ignored => None,
        EntryOwner::SharedStatic => Some(shared_codex_home),
        EntryOwner::ConfigGroup => Some(config_home),
        EntryOwner::ResumeGroup => Some(resume_home),
    }
}

fn entry_owner(name: &str) -> EntryOwner {
    if name == "auth.json" {
        return EntryOwner::LocalAuth;
    }
    if matches!(
        name,
        "profiles" | "config-groups" | "resume-groups" | "app-server-control" | "l"
    ) {
        return EntryOwner::Ignored;
    }
    if is_config_group_entry(name) {
        return EntryOwner::ConfigGroup;
    }
    if is_resume_group_entry(name) {
        return EntryOwner::ResumeGroup;
    }

    EntryOwner::SharedStatic
}

fn is_config_group_entry(name: &str) -> bool {
    matches!(
        name,
        "config.toml" | ".codex-global-state.json" | "history.jsonl" | ".personality_migration"
    ) || name.starts_with("config.toml.bak")
}

fn is_resume_group_entry(name: &str) -> bool {
    matches!(
        name,
        "session_index.jsonl"
            | "sessions"
            | "archived_sessions"
            | "shell_snapshots"
            | "worktrees"
            | "memories"
            | "log"
    ) || runtime_db(name).is_some()
}

fn runtime_db(name: &str) -> Option<RuntimeDb<'_>> {
    let (base, sidecar) = SQLITE_SIDECAR_SUFFIXES
        .iter()
        .find_map(|suffix| name.strip_suffix(suffix))
        .map_or((name, false), |base| (base, true));
    let (family, version) = base.strip_suffix(".sqlite")?.rsplit_once('_')?;
    if version.is_empty() || !version.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    RESUME_GROUP_DB_FAMILIES
        .contains(&family)
        .then_some(RuntimeDb { family, sidecar })
}

// a group database that does not exist yet is still linked so codex creates it inside the group
// instead of inside the throwaway launch home; sqlite resolves the link and keeps its -wal and
// -shm beside the real file
fn links_missing_source(name: &str) -> bool {
    runtime_db(name).is_some_and(|db| !db.sidecar)
}

/// Copies a live SQLite database as one consistent snapshot
///
/// Copying the main file and its WAL one at a time can capture them at different moments, and
/// codex would replay the mismatched frames onto the copy when it opens it. The target must sit in
/// a group that is still staged, since no launch can have opened a database there yet
fn snapshot_sqlite_db(source: &Path, target: &Path) -> Result<()> {
    let target_path = target
        .to_str()
        .ok_or_else(|| eyre!("Invalid database path: {}", target.display()))?;
    let connection = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .wrap_err_with(|| format!("Failed to open {}", source.display()))?;
    connection.busy_timeout(SEED_SNAPSHOT_BUSY_TIMEOUT)?;
    connection
        .execute("VACUUM INTO ?1", [target_path])
        .wrap_err_with(|| format!("Failed to snapshot {}", source.display()))?;
    Ok(())
}

fn copy_auth_file(source: &Path, target: &Path) -> Result<()> {
    fsutil::remove_existing_path(target)?;
    fsutil::ensure_parent_dir(target)?;
    stdfs::copy(source, target)?;
    Ok(())
}

fn sync_launch_auth(launch_auth_mode: &LaunchAuthMode, target: &Path) -> Result<()> {
    match launch_auth_mode {
        LaunchAuthMode::GlobalShared { global_auth } => sync_shared_entry(global_auth, target),
        LaunchAuthMode::ProfileCopy { profile_auth, .. } => copy_auth_file(profile_auth, target),
    }
}

pub(super) fn replace_global_auth_with_profile(
    profile_auth: &Path,
    global_auth: &Path,
) -> Result<()> {
    copy_auth_file(profile_auth, global_auth)
}

pub(super) fn notify_remodex_bridge_profile_switch(
    profile: &str,
) -> Result<RemodexBridgeSwitchOutcome> {
    let socket_path = remodex_bridge_control_socket_path()?;
    let mut stream = match UnixStream::connect(&socket_path) {
        Ok(stream) => stream,
        Err(err)
            if matches!(
                err.kind(),
                ErrorKind::NotFound | ErrorKind::ConnectionRefused
            ) =>
        {
            return Ok(RemodexBridgeSwitchOutcome::NotRunning);
        }
        Err(err) => return Err(err.into()),
    };

    let request = RemodexBridgeSwitchRequest {
        method: "switchProfile",
        profile,
    };
    let request_raw = serde_json::to_vec(&request)?;
    stream.write_all(&request_raw)?;
    stream.write_all(b"\n")?;
    stream.shutdown(Shutdown::Write)?;

    let mut response_raw = Vec::new();
    stream.read_to_end(&mut response_raw)?;
    if response_raw.is_empty() {
        return Err(eyre!("phodex-bridge returned no response"));
    }

    let response: RemodexBridgeSwitchResponse = serde_json::from_slice(&response_raw)?;
    if response.ok {
        Ok(RemodexBridgeSwitchOutcome::Switched)
    } else {
        Err(eyre!(
            "{}",
            response
                .error
                .unwrap_or_else(|| "phodex-bridge rejected the profile switch".into())
        ))
    }
}

fn copy_seed_entry(source: &Path, target: &Path) -> Result<()> {
    let metadata = stdfs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() {
        let link_target = stdfs::read_link(source)?;
        fsutil::ensure_parent_dir(target)?;
        symlink(link_target, target)?;
        return Ok(());
    }

    if metadata.is_dir() {
        stdfs::create_dir_all(target)?;
        for entry in stdfs::read_dir(source)? {
            let entry = entry?;
            copy_seed_entry(&entry.path(), &target.join(entry.file_name()))?;
        }
        return Ok(());
    }

    fsutil::ensure_parent_dir(target)?;
    stdfs::copy(source, target)?;
    Ok(())
}

fn sync_shared_entry(source: &Path, target: &Path) -> Result<()> {
    if symlink_points_to(target, source)? {
        return Ok(());
    }

    fsutil::remove_existing_path(target)?;
    fsutil::ensure_parent_dir(target)?;
    symlink(source, target)?;
    Ok(())
}

fn symlink_points_to(target: &Path, source: &Path) -> Result<bool> {
    match stdfs::read_link(target) {
        Ok(existing) => Ok(existing == source),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(false),
        Err(err) if err.kind() == ErrorKind::InvalidInput => Ok(false),
        Err(err) => Err(err.into()),
    }
}

fn path_exists(path: &Path) -> Result<bool> {
    match stdfs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err.into()),
    }
}

impl GroupKind {
    fn label(self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::Resume => "resume",
        }
    }

    fn owns(self, owner: EntryOwner) -> bool {
        match self {
            Self::Config => owner == EntryOwner::ConfigGroup,
            Self::Resume => owner == EntryOwner::ResumeGroup,
        }
    }

    fn seeds(self, name: &str, owner: EntryOwner) -> bool {
        self.owns(owner) || matches!(self, Self::Config) && name == "config.toml"
    }
}

impl RuntimeDb<'_> {
    fn seeds_by_snapshot(self) -> bool {
        // a snapshot is self-contained, so the source's sidecars would only corrupt it; thread
        // history is a projection of the group's rollouts that codex rebuilds from byte zero for
        // any thread without a checkpoint, while a copied checkpoint that runs ahead of the copied
        // rollouts fails as "behind durable rollout"
        !self.sidecar && self.family != "thread_history"
    }
}

pub(super) fn delete_profile_home(profile_home: &Path) -> Result<()> {
    if !profile_home.exists() {
        return Err(eyre!("Profile home not found: {}", profile_home.display()));
    }

    fsutil::remove_existing_path(profile_home)
}

#[cfg(test)]
mod tests {
    use super::{
        group_staging_prefix, pids_holding_path, prepare_resume_group_home, process_is_running,
        publish_group_home, recover_thread_writer, stage_group_home, stop_process,
        sync_launch_codex_home, GroupKind, LaunchAuthMode, ThreadWriterPolicy,
        ThreadWriterRecovery,
    };
    use rusqlite::Connection;
    use std::fs;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    const THREAD_SCOPED_DBS: [&str; 7] = [
        "state_6.sqlite",
        "logs_3.sqlite",
        "thread_history_1.sqlite",
        "goals_1.sqlite",
        "queue_1.sqlite",
        "memories_1.sqlite",
        "memories_v2_1.sqlite",
    ];

    #[test]
    fn existing_resume_group_gets_its_own_thread_databases_instead_of_the_shared_ones() {
        let dir = tempdir().unwrap();
        let shared = dir.path().join(".codex");
        let group = shared.join("resume-groups").join("shared-work");
        let launch_home = dir.path().join("launch");
        let global_auth = shared.join("auth.json");

        fs::create_dir_all(shared.join("plugins")).unwrap();
        fs::create_dir_all(shared.join("sqlite")).unwrap();
        fs::create_dir_all(group.join("sessions")).unwrap();
        fs::write(&global_auth, "auth").unwrap();
        fs::write(shared.join("config.toml"), "model = \"gpt-5.4\"").unwrap();
        fs::write(shared.join("models_cache.json"), "{}").unwrap();
        for name in THREAD_SCOPED_DBS {
            fs::write(shared.join(name), "shared threads").unwrap();
        }
        fs::write(shared.join("thread_history_1.sqlite-wal"), "shared wal").unwrap();

        let resume_home = prepare_resume_group_home(&shared, "shared-work").unwrap();
        sync_launch_codex_home(
            &launch_home,
            &shared,
            &LaunchAuthMode::GlobalShared { global_auth },
            &shared,
            &resume_home,
        )
        .unwrap();

        for name in THREAD_SCOPED_DBS {
            assert!(!group.join(name).exists(), "{name} was seeded from shared");
            assert_eq!(
                fs::read_link(launch_home.join(name)).unwrap(),
                group.join(name)
            );
        }
        assert!(fs::symlink_metadata(launch_home.join("thread_history_1.sqlite-wal")).is_err());
        for name in ["plugins", "sqlite", "config.toml", "models_cache.json"] {
            assert_eq!(
                fs::read_link(launch_home.join(name)).unwrap(),
                shared.join(name)
            );
        }

        let codex = Connection::open(launch_home.join("goals_1.sqlite")).unwrap();
        codex
            .execute_batch(
                "PRAGMA journal_mode = WAL;
                 CREATE TABLE thread_goals (thread_id TEXT PRIMARY KEY);
                 INSERT INTO thread_goals VALUES ('group-thread');",
            )
            .unwrap();

        assert!(group.join("goals_1.sqlite-wal").exists());
        assert!(!launch_home.join("goals_1.sqlite-wal").exists());
        drop(codex);

        let group_goals = Connection::open(group.join("goals_1.sqlite")).unwrap();
        let thread_id: String = group_goals
            .query_row("SELECT thread_id FROM thread_goals", [], |row| row.get(0))
            .unwrap();

        assert_eq!(thread_id, "group-thread");
        assert_eq!(
            fs::read_to_string(shared.join("goals_1.sqlite")).unwrap(),
            "shared threads"
        );
    }

    #[test]
    fn new_resume_group_snapshots_live_databases_and_leaves_thread_history_to_rebuild() {
        let dir = tempdir().unwrap();
        let shared = dir.path().join(".codex");
        fs::create_dir_all(shared.join("sessions")).unwrap();

        // the shared writer stays open with its rows only in the WAL, as during a live session
        let writer = Connection::open(shared.join("goals_1.sqlite")).unwrap();
        writer
            .execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA wal_autocheckpoint = 0;
                 CREATE TABLE thread_goals (thread_id TEXT PRIMARY KEY);
                 INSERT INTO thread_goals VALUES ('forked-thread');",
            )
            .unwrap();
        let history = Connection::open(shared.join("thread_history_1.sqlite")).unwrap();
        history
            .execute_batch(
                "PRAGMA journal_mode = WAL;
                 CREATE TABLE thread_history_projection_state (thread_id TEXT PRIMARY KEY);",
            )
            .unwrap();

        assert!(shared.join("goals_1.sqlite-wal").exists());

        let group = prepare_resume_group_home(&shared, "shared-work").unwrap();
        let seeded = Connection::open(group.join("goals_1.sqlite")).unwrap();
        let thread_id: String = seeded
            .query_row("SELECT thread_id FROM thread_goals", [], |row| row.get(0))
            .unwrap();

        assert_eq!(thread_id, "forked-thread");
        assert!(!group.join("goals_1.sqlite-wal").exists());
        assert!(!group.join("goals_1.sqlite-shm").exists());
        assert!(!group.join("thread_history_1.sqlite").exists());
        assert!(!group.join("thread_history_1.sqlite-wal").exists());
        drop(writer);
        drop(history);
    }

    #[test]
    fn failed_snapshot_leaves_no_group_so_the_next_launch_seeds_it() {
        let dir = tempdir().unwrap();
        let shared = dir.path().join(".codex");
        let groups = shared.join("resume-groups");
        fs::create_dir_all(shared.join("sessions/2026")).unwrap();
        fs::write(shared.join("sessions/2026/rollout.jsonl"), "rollout").unwrap();
        fs::write(shared.join("state_5.sqlite"), "not a database").unwrap();

        assert!(prepare_resume_group_home(&shared, "work").is_err());
        assert_eq!(fs::read_dir(&groups).unwrap().count(), 0);

        fs::remove_file(shared.join("state_5.sqlite")).unwrap();
        Connection::open(shared.join("state_5.sqlite"))
            .unwrap()
            .execute_batch(
                "CREATE TABLE threads (id TEXT);
                 INSERT INTO threads VALUES ('forked-thread');",
            )
            .unwrap();

        let group = prepare_resume_group_home(&shared, "work").unwrap();
        let thread_id: String = Connection::open(group.join("state_5.sqlite"))
            .unwrap()
            .query_row("SELECT id FROM threads", [], |row| row.get(0))
            .unwrap();

        assert_eq!(thread_id, "forked-thread");
        assert_eq!(
            fs::read_to_string(group.join("sessions/2026/rollout.jsonl")).unwrap(),
            "rollout"
        );
        assert_eq!(fs::read_dir(&groups).unwrap().count(), 1);
    }

    #[test]
    fn group_published_first_by_another_launch_keeps_its_live_database() {
        let dir = tempdir().unwrap();
        let shared = dir.path().join(".codex");
        let groups = shared.join("resume-groups");
        let group = groups.join("work");
        fs::create_dir_all(&shared).unwrap();
        Connection::open(shared.join("state_5.sqlite"))
            .unwrap()
            .execute_batch(
                "CREATE TABLE threads (id TEXT);
                 INSERT INTO threads VALUES ('shared-thread');",
            )
            .unwrap();

        let staging = stage_group_home(&shared, &groups, "work", GroupKind::Resume).unwrap();
        let staging_path = staging.path().to_path_buf();

        // the other launch's codex creates the group database through its dangling launch link
        // while this launch is still snapshotting
        fs::create_dir_all(&group).unwrap();
        let live = Connection::open(group.join("state_5.sqlite")).unwrap();
        live.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE threads (id TEXT);
             INSERT INTO threads VALUES ('live-thread');",
        )
        .unwrap();

        publish_group_home(staging, &group).unwrap();
        assert_eq!(prepare_resume_group_home(&shared, "work").unwrap(), group);
        live.execute("INSERT INTO threads VALUES ('after-publish')", [])
            .unwrap();

        let reopened = Connection::open(group.join("state_5.sqlite")).unwrap();
        let mut query = reopened.prepare("SELECT id FROM threads").unwrap();
        let thread_ids: Vec<String> = query
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();

        assert_eq!(thread_ids, ["live-thread", "after-publish"]);
        assert!(!staging_path.exists());
        assert_eq!(fs::read_dir(&groups).unwrap().count(), 1);
    }

    #[test]
    fn new_group_removes_only_stagings_whose_creator_exited() {
        let dir = tempdir().unwrap();
        let shared = dir.path().join(".codex");
        let groups = shared.join("resume-groups");
        let mut exited = Command::new("true").spawn().unwrap();
        let exited_pid = exited.id();
        exited.wait().unwrap();

        let abandoned = groups.join(format!(
            "{}abc123",
            group_staging_prefix("other", exited_pid)
        ));
        let running = groups.join(format!(
            "{}abc123",
            group_staging_prefix("other", std::process::id())
        ));
        fs::create_dir_all(abandoned.join("sessions")).unwrap();
        fs::create_dir_all(&running).unwrap();

        prepare_resume_group_home(&shared, "work").unwrap();

        assert!(!abandoned.exists());
        assert!(running.exists());
    }

    #[test]
    fn force_stops_only_the_process_holding_the_thread_lock() {
        assert_force_stops_lock_holder("sleep 30 > \"$1\" 2>&1 & echo $!");
    }

    #[test]
    fn force_stops_lock_holder_that_ignores_termination() {
        assert_force_stops_lock_holder("(trap '' TERM; exec sleep 30) > \"$1\" 2>&1 & echo $!");
    }

    fn assert_force_stops_lock_holder(script: &str) {
        let dir = tempdir().unwrap();
        let launch_home = dir.path().join("launch");
        let locks_dir = launch_home.join("thread-writer-locks");
        fs::create_dir_all(&locks_dir).unwrap();
        let thread_id = "019fe286-71f0-7513-ba31-fa0433073e6b";
        let lock_path = locks_dir.join(format!("{thread_id}.lock"));
        let unrelated_lock_path = locks_dir.join("other-thread.lock");
        let unrelated = Command::new("sh")
            .args(["-c", "sleep 30 > \"$1\" 2>&1 & echo $!", "sh"])
            .arg(&unrelated_lock_path)
            .output()
            .unwrap();
        let unrelated_pid = String::from_utf8(unrelated.stdout)
            .unwrap()
            .trim()
            .parse::<u32>()
            .unwrap();
        let output = Command::new("sh")
            .args(["-c", script, "sh"])
            .arg(&lock_path)
            .output()
            .unwrap();

        let holder_pid = String::from_utf8(output.stdout)
            .unwrap()
            .trim()
            .parse::<u32>()
            .unwrap();

        for _ in 0..20 {
            if pids_holding_path(&lock_path).unwrap() == [holder_pid] {
                break;
            }

            thread::sleep(Duration::from_millis(50));
        }

        let current = recover_thread_writer(
            &launch_home,
            &dir.path().join("profiles"),
            thread_id,
            ThreadWriterPolicy::StopConflicting,
            Some(holder_pid),
        )
        .unwrap();

        assert_eq!(current, ThreadWriterRecovery::CurrentSession);
        assert!(process_is_running(holder_pid).unwrap());

        let preserved = recover_thread_writer(
            &launch_home,
            &dir.path().join("profiles"),
            thread_id,
            ThreadWriterPolicy::PreserveActive,
            None,
        )
        .unwrap();

        assert_eq!(
            preserved,
            ThreadWriterRecovery::Unmanaged {
                pids: vec![holder_pid]
            }
        );
        assert!(process_is_running(holder_pid).unwrap());

        let forced = recover_thread_writer(
            &launch_home,
            &dir.path().join("profiles"),
            thread_id,
            ThreadWriterPolicy::StopConflicting,
            None,
        )
        .unwrap();

        assert_eq!(
            forced,
            ThreadWriterRecovery::Ready {
                stopped_pids: vec![holder_pid]
            }
        );
        assert!(!process_is_running(holder_pid).unwrap());
        assert!(process_is_running(unrelated_pid).unwrap());
        stop_process(unrelated_pid).unwrap();
    }
}
