use super::*;
use crate::fsutil;
use std::collections::BTreeSet;

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

fn profile_launches_dir(profile_home: &Path) -> PathBuf {
    profile_home.join(".launch")
}

pub(super) fn create_launch_home(profile_home: &Path) -> Result<PathBuf> {
    let launches_dir = profile_launches_dir(profile_home);
    stdfs::create_dir_all(&launches_dir)?;

    for attempt in 0..10 {
        let timestamp = Utc::now().format("%Y%m%dT%H%M%S%fZ");
        let suffix = if attempt == 0 {
            format!("pid{}", std::process::id())
        } else {
            format!("pid{}-{attempt}", std::process::id())
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
    pid: u32,
    launch_home: &Path,
) -> Result<PathBuf> {
    let markers_dir = profile_session_markers_dir(profile_home);
    stdfs::create_dir_all(&markers_dir)?;
    let marker_path = markers_dir.join(format!("{pid}.json"));
    let marker = SessionMarker {
        pid,
        started_at: Utc::now(),
        launch_home: launch_home.to_path_buf(),
    };
    stdfs::write(&marker_path, serde_json::to_vec_pretty(&marker)?)?;
    Ok(marker_path)
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
    let pid = marker.pid.to_string();
    let output = std::process::Command::new("ps")
        .args(["-o", "comm=", "-p", &pid])
        .output()?;
    if !output.status.success() {
        return Ok(false);
    }

    let command = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if command.is_empty() {
        return Ok(false);
    }

    Ok(command.contains("codex"))
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

        if !kind.owns(entry_owner(name)) {
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
    if matches!(name, "profiles" | "config-groups" | "resume-groups") {
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
}

pub(super) fn delete_profile_home(profile_home: &Path) -> Result<()> {
    if !profile_home.exists() {
        return Err(eyre!("Profile home not found: {}", profile_home.display()));
    }

    fsutil::remove_existing_path(profile_home)
}
