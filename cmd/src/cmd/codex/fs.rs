use super::app_server::{ConfigOverride, SessionControl, SessionMarker, SessionThread};
use super::*;
use crate::fsutil;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

const SHARED_GROUP_NAME: &str = "shared";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupKind {
    Config,
    Resume,
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
            fsutil::remove_existing_path(&path)?;
        }
    }

    Ok(active)
}

fn read_session_marker(path: &Path) -> Result<SessionMarker> {
    Ok(serde_json::from_slice(&stdfs::read(path)?)?)
}

fn session_marker_is_active(marker: &SessionMarker) -> Result<bool> {
    Ok(std::process::Command::new("ps")
        .args(["-p", &marker.owner_pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?
        .success())
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
    stdfs::create_dir_all(&group_home)?;
    seed_group_home(shared_codex_home, &group_home, kind)?;
    Ok(group_home)
}

fn seed_group_home(shared_codex_home: &Path, group_home: &Path, kind: GroupKind) -> Result<()> {
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

        copy_seed_entry(&entry.path(), &target)?;
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
        if !path_exists(&source)? {
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
    ) || name.starts_with("state_5.sqlite")
        || is_sqlite_log_entry(name)
}

fn is_sqlite_log_entry(name: &str) -> bool {
    name.starts_with("logs_") && name.contains(".sqlite")
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

pub(super) fn delete_profile_home(profile_home: &Path) -> Result<()> {
    if !profile_home.exists() {
        return Err(eyre!("Profile home not found: {}", profile_home.display()));
    }

    fsutil::remove_existing_path(profile_home)
}
