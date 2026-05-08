use super::*;
use crate::{fsutil, runtime};

pub(super) fn launch(
    profile_or_arg: Option<&OsString>,
    resume_group: Option<&str>,
    config_group: Option<&str>,
    other: bool,
    args: &[OsString],
) -> Result<()> {
    let mut profiles = load_saved_profiles(&profiles_dir()?)?;
    let active_auth = active_global_auth();
    let target = resolve_launch_target(
        profile_or_arg,
        resume_group,
        config_group,
        other,
        args,
        &profiles,
    )?;
    enrich_profile_usage(&mut profiles)?;
    let loader = ProfileUsageLoader::with_timeout(LAUNCH_ACTIVE_USAGE_FETCH_TIMEOUT)?;
    enrich_active_profiles_from_global_auth(&mut profiles, &loader, active_auth.as_ref(), true)?;

    match target {
        LaunchTarget::Explicit {
            profile,
            resume_group,
            config_group,
            args,
        } => {
            let details = profiles
                .iter()
                .find(|saved_profile| saved_profile.name == profile)
                .map(launch_banner_details)
                .unwrap_or_else(LaunchBannerDetails::fallback);
            launch_with_profile(
                &profile,
                resume_group.as_deref(),
                config_group.as_deref(),
                &details,
                &args,
            )
        }
        LaunchTarget::Auto {
            resume_group,
            config_group,
            other,
            args,
        } => {
            let excluded_profile = if other {
                active_profile_name(&profiles)
            } else {
                None
            };
            let profile = select_auto_launch_profile_except(&profiles, excluded_profile)?;
            let details = launch_banner_details(profile);
            launch_with_profile(
                &profile.name,
                resume_group.as_deref(),
                config_group.as_deref(),
                &details,
                &args,
            )
        }
    }
}

fn launch_with_profile(
    profile: &str,
    resume_group: Option<&str>,
    config_group: Option<&str>,
    details: &LaunchBannerDetails,
    args: &[OsString],
) -> Result<()> {
    let shared_codex_home = codex_dir()?;
    let global_auth = auth_path()?;
    let profile_home = profile_codex_home(profile)?;
    stdfs::create_dir_all(&profile_home)?;
    let profile_auth = profile_home.join("auth.json");
    if !profile_auth.exists() {
        return Err(eyre!(
            "Profile '{profile}' not found. Run: cmd codex login {profile}"
        ));
    }

    let groups = resolve_launch_groups(profile, resume_group, config_group)?;
    println!("{}", format_launch_banner(profile, &groups, details));

    let config_home = prepare_config_group_home(&shared_codex_home, &groups.config)?;
    let resume_home = prepare_resume_group_home(&shared_codex_home, &groups.resume)?;
    let launch_home = create_launch_home(&profile_home)?;
    let launch_auth_mode = resolve_launch_auth_mode(&profile_auth, &global_auth)?;
    sync_launch_codex_home(
        &launch_home,
        &shared_codex_home,
        &launch_auth_mode,
        &config_home,
        &resume_home,
    )?;
    let mut child = codex_command(&launch_home);
    child.args(args);
    let mut child = child.spawn()?;
    let session_marker_path = match write_session_marker(&profile_home, child.id(), &launch_home) {
        Ok(marker_path) => marker_path,
        Err(err) => {
            child.kill().ok();
            let _ = child.wait();
            return Err(err);
        }
    };
    let thread_capture =
        start_session_marker_thread_capture(session_marker_path.clone(), launch_home.clone());
    let status = child.wait()?;
    thread_capture.stop();
    fsutil::remove_existing_path(&session_marker_path)?;
    if let LaunchAuthMode::ProfileCopy {
        profile_auth,
        launch_auth,
    } = &launch_auth_mode
    {
        promote_launch_auth_if_unchanged(
            profile_auth,
            launch_auth,
            &launch_home.join("auth.json"),
        )?;
    }
    std::process::exit(status.code().unwrap_or(1));
}

const THREAD_CAPTURE_TIMEOUT: Duration = Duration::from_secs(10);
const THREAD_CAPTURE_POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CapturedThread {
    pub(super) id: String,
    pub(super) rollout_path: PathBuf,
}

struct SessionMarkerThreadCapture {
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    handle: std::thread::JoinHandle<()>,
}

impl SessionMarkerThreadCapture {
    fn stop(self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        self.handle.join().ok();
    }
}

fn start_session_marker_thread_capture(
    marker_path: PathBuf,
    launch_home: PathBuf,
) -> SessionMarkerThreadCapture {
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let thread_stop = stop.clone();
    let handle = std::thread::spawn(move || {
        let Some(thread) = wait_for_launch_thread(&launch_home, &thread_stop) else {
            return;
        };
        update_session_marker_thread(&marker_path, thread.id, thread.rollout_path).ok();
    });

    SessionMarkerThreadCapture { stop, handle }
}

fn wait_for_launch_thread(
    launch_home: &Path,
    stop: &std::sync::atomic::AtomicBool,
) -> Option<CapturedThread> {
    let deadline = std::time::Instant::now() + THREAD_CAPTURE_TIMEOUT;
    loop {
        if let Some(thread) = capture_launch_thread(launch_home) {
            return Some(thread);
        }
        if stop.load(std::sync::atomic::Ordering::Relaxed) || std::time::Instant::now() >= deadline
        {
            return None;
        }
        std::thread::sleep(THREAD_CAPTURE_POLL_INTERVAL);
    }
}

fn capture_launch_thread(launch_home: &Path) -> Option<CapturedThread> {
    let state_db = launch_home.join("state_5.sqlite");
    if !state_db.exists() {
        return None;
    }

    let sessions_prefix = launch_home.join("sessions");
    let sessions_prefix = sessions_prefix.to_str()?;
    let output = std::process::Command::new("sqlite3")
        .arg(format!("file:{}?mode=ro", state_db.display()))
        .arg(format!(
            "select id || char(31) || rollout_path from threads \
             where source = 'cli' \
             and (agent_role is null or agent_role = '') \
             and rollout_path like {} \
             order by created_at_ms asc, created_at asc;",
            sqlite_quote(&format!("{sessions_prefix}/%"))
        ))
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    parse_captured_threads(&String::from_utf8_lossy(&output.stdout))
}

pub(super) fn parse_captured_threads(output: &str) -> Option<CapturedThread> {
    let threads = output
        .lines()
        .filter_map(parse_captured_thread)
        .collect::<Vec<_>>();
    match threads.as_slice() {
        [thread] => Some(thread.clone()),
        _ => None,
    }
}

fn parse_captured_thread(line: &str) -> Option<CapturedThread> {
    let (id, rollout_path) = line.split_once('\x1f')?;
    if id.is_empty() || rollout_path.is_empty() {
        return None;
    }

    Some(CapturedThread {
        id: id.to_owned(),
        rollout_path: PathBuf::from(rollout_path),
    })
}

fn sqlite_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

pub(super) fn resolve_launch_auth_mode(
    profile_auth: &Path,
    global_auth: &Path,
) -> Result<LaunchAuthMode> {
    let launch_auth = read_auth_snapshot(profile_auth)?;
    let uses_global_auth = read_auth_identity(global_auth)
        .map(|global_identity| is_same_user(&global_identity, &launch_auth.identity))
        .unwrap_or(false);

    if uses_global_auth {
        return Ok(LaunchAuthMode::GlobalShared {
            global_auth: global_auth.to_path_buf(),
        });
    }

    Ok(LaunchAuthMode::ProfileCopy {
        profile_auth: profile_auth.to_path_buf(),
        launch_auth,
    })
}

pub(super) fn resolve_launch_target(
    profile_or_arg: Option<&OsString>,
    resume_group: Option<&str>,
    config_group: Option<&str>,
    other: bool,
    args: &[OsString],
    profiles: &[SavedProfile],
) -> Result<LaunchTarget> {
    let explicit_profile = profile_or_arg
        .and_then(|value| value.to_str())
        .filter(|profile| {
            profiles
                .iter()
                .any(|saved_profile| saved_profile.name == *profile)
        });

    if let Some(profile) = explicit_profile {
        if other {
            return Err(eyre!("--other cannot be combined with an explicit profile"));
        }

        return Ok(LaunchTarget::Explicit {
            profile: profile.into(),
            resume_group: resume_group.map(str::to_owned),
            config_group: config_group.map(str::to_owned),
            args: args.to_vec(),
        });
    }

    let mut resolved_args = Vec::with_capacity(args.len() + usize::from(profile_or_arg.is_some()));
    if let Some(value) = profile_or_arg {
        resolved_args.push(value.clone());
    }
    resolved_args.extend(args.iter().cloned());

    Ok(LaunchTarget::Auto {
        resume_group: resume_group.map(str::to_owned),
        config_group: config_group.map(str::to_owned),
        other,
        args: resolved_args,
    })
}

#[cfg(test)]
pub(super) fn select_auto_launch_profile(profiles: &[SavedProfile]) -> Result<&SavedProfile> {
    select_auto_launch_profile_except(profiles, None)
}

pub(super) fn select_auto_launch_profile_except<'a>(
    profiles: &'a [SavedProfile],
    excluded_profile: Option<&str>,
) -> Result<&'a SavedProfile> {
    let now = Utc::now();
    let mut cool_candidates = Vec::new();
    let mut hot_candidates = Vec::new();

    for profile in profiles {
        if excluded_profile.is_some_and(|excluded| excluded == profile.name) {
            continue;
        }

        let Some(candidate) = launch_candidate(profile, now) else {
            continue;
        };

        if candidate.five_hour >= 80.0 {
            hot_candidates.push(candidate);
        } else {
            cool_candidates.push(candidate);
        }
    }

    cool_candidates.sort_by(compare_launch_candidates);
    hot_candidates.sort_by(compare_launch_candidates);

    cool_candidates
        .first()
        .or_else(|| hot_candidates.first())
        .map(|candidate| candidate.profile)
        .ok_or_else(|| {
            let target = if excluded_profile.is_some() {
                "other profiles"
            } else {
                "profiles"
            };

            eyre!("No {target} with usable usage data found. Run: cmd codex list or specify a saved profile explicitly")
        })
}

fn active_profile_name(profiles: &[SavedProfile]) -> Option<&str> {
    let active_identity = auth_path()
        .ok()
        .and_then(|path| read_auth_identity(&path).ok())?;

    profiles.iter().find_map(|profile| {
        profile
            .identity
            .as_ref()
            .filter(|identity| is_same_user(&active_identity, identity))
            .map(|_| profile.name.as_str())
    })
}

pub(super) fn saved_profile_label(profile: &SavedProfile) -> String {
    profile
        .identity
        .as_ref()
        .map(best_label)
        .unwrap_or_else(|| "-".into())
}

pub(super) fn format_launch_banner(
    profile: &str,
    groups: &LaunchGroups,
    details: &LaunchBannerDetails,
) -> String {
    format!(
        "{}\n{} {} {} {} {} {} {} {} {}\n{}",
        details.label.yellow(),
        "launching".green().bold(),
        "profile".blue().bold(),
        profile.cyan().bold(),
        "|".white().dimmed(),
        "config".blue().bold(),
        groups.config.cyan().bold(),
        "|".white().dimmed(),
        "resume".blue().bold(),
        groups.resume.cyan().bold(),
        details.render_usage_line(),
    )
}

pub(super) struct LaunchBannerDetails {
    label: String,
    five_hour_compact: String,
    five_hour_style: LimitStyleKind,
    weekly_compact: String,
    weekly_style: LimitStyleKind,
}

impl LaunchBannerDetails {
    fn fallback() -> Self {
        Self {
            label: "-".into(),
            five_hour_compact: "-".into(),
            five_hour_style: LimitStyleKind::Normal,
            weekly_compact: "-".into(),
            weekly_style: LimitStyleKind::Normal,
        }
    }

    fn render_usage_line(&self) -> String {
        format!(
            "{}  {}  {}",
            self.render_usage_segment("5H:", &self.five_hour_compact, self.five_hour_style),
            "|".white().dimmed(),
            self.render_usage_segment("Weekly:", &self.weekly_compact, self.weekly_style),
        )
    }

    fn render_usage_segment(&self, label: &str, value: &str, style: LimitStyleKind) -> String {
        format!(
            "{} {}",
            label.blue().bold(),
            render_usage_limit_cell(value, value.len(), style),
        )
    }
}

pub(super) fn launch_banner_details(profile: &SavedProfile) -> LaunchBannerDetails {
    let current_local = Local::now();
    let current_utc = Utc::now();

    LaunchBannerDetails {
        label: saved_profile_label(profile),
        five_hour_compact: current_usage_window_compact(
            &profile.usage,
            UsageWindowKind::Primary,
            current_local,
            current_utc,
        ),
        five_hour_style: five_hour_limit_style(&profile.usage),
        weekly_compact: current_usage_window_compact(
            &profile.usage,
            UsageWindowKind::Secondary,
            current_local,
            current_utc,
        ),
        weekly_style: usage_window_style(&profile.usage, UsageWindowKind::Secondary),
    }
}

struct LaunchCandidate<'a> {
    profile: &'a SavedProfile,
    weekly_pace_delta: f64,
    five_hour_pace_delta: f64,
    five_hour: f64,
    five_hour_reset_at: Option<i64>,
    score: f64,
}

fn launch_candidate(
    profile: &SavedProfile,
    now: chrono::DateTime<Utc>,
) -> Option<LaunchCandidate<'_>> {
    if profile.invalid_auth {
        return None;
    }

    let ProfileUsageState::Available(usage) = &profile.usage else {
        return None;
    };

    let primary = usage.primary.as_ref()?;
    let secondary = usage.secondary.as_ref()?;
    if primary.used_percent >= 100.0 || secondary.used_percent >= 100.0 {
        return None;
    }

    let five_hour = primary.used_percent;
    let five_hour_pace_delta =
        pace_delta_percent(primary, now, UsageWindowKind::Primary).unwrap_or(five_hour);
    let weekly_pace_delta = pace_delta_percent(secondary, now, UsageWindowKind::Secondary)?;

    Some(LaunchCandidate {
        profile,
        weekly_pace_delta,
        five_hour_pace_delta,
        five_hour,
        five_hour_reset_at: primary.reset_at,
        score: weekly_pace_delta * 3.0 + five_hour_pace_delta,
    })
}

fn compare_launch_candidates(
    left: &LaunchCandidate<'_>,
    right: &LaunchCandidate<'_>,
) -> std::cmp::Ordering {
    left.score
        .total_cmp(&right.score)
        .then_with(|| left.weekly_pace_delta.total_cmp(&right.weekly_pace_delta))
        .then_with(|| {
            left.five_hour_pace_delta
                .total_cmp(&right.five_hour_pace_delta)
        })
        .then_with(|| compare_reset_timestamps(left.five_hour_reset_at, right.five_hour_reset_at))
        .then_with(|| left.five_hour.total_cmp(&right.five_hour))
        .then_with(|| left.profile.name.cmp(&right.profile.name))
}

fn compare_reset_timestamps(left: Option<i64>, right: Option<i64>) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(&right),
        _ => std::cmp::Ordering::Equal,
    }
}

pub(super) fn login(profile: &str, device_auth: bool) -> Result<()> {
    let shared_codex_home = codex_dir()?;
    let staged_home = tempfile::tempdir()?;

    sync_login_codex_home(staged_home.path(), &shared_codex_home)?;

    codex_command(staged_home.path())
        .arg("logout")
        .status()
        .ok();

    let mut login_command = codex_command(staged_home.path());
    login_command.arg("login");

    if device_auth {
        login_command.arg("--device-auth");
    }

    let status = login_command.status()?;
    if !status.success() {
        return Err(eyre!("codex login failed"));
    }

    let auth = staged_home.path().join("auth.json");
    if !auth.exists() {
        return Err(eyre!("No auth.json found after login"));
    }

    read_auth_identity(&auth).wrap_err("Failed to read codex auth identity")?;
    save_profile_auth(profile, &auth, &profiles_dir()?)?;
    let profile_home = profile_codex_home(profile)?;
    stdfs::create_dir_all(&profile_home)?;
    println!("Saved codex profile: {profile}");

    Ok(())
}

pub(super) fn list(verbose: bool) -> Result<()> {
    let mut profiles = load_saved_profiles(&profiles_dir()?)?;
    if profiles.is_empty() {
        println!("No profiles. Run: cmd codex login <name>");
        return Ok(());
    }

    let active_auth = active_global_auth();

    enrich_profile_usage(&mut profiles)?;
    let loader = ProfileUsageLoader::new()?;
    enrich_active_profiles_from_global_auth(&mut profiles, &loader, active_auth.as_ref(), false)?;

    let rows = build_profile_rows(
        &profiles,
        active_auth.as_ref().map(|(_, identity)| identity),
    );
    if verbose {
        print_verbose_profile_table(&rows);
    } else {
        print_compact_profile_table(&rows);
    }

    Ok(())
}

fn active_global_auth() -> Option<(StoredAuth, AuthIdentity)> {
    auth_path()
        .ok()
        .and_then(|path| read_stored_auth(&path).ok())
        .and_then(|auth| {
            stored_auth_identity(&auth)
                .ok()
                .map(|identity| (auth, identity))
        })
}

pub(super) fn enrich_active_profiles_from_global_auth(
    profiles: &mut [SavedProfile],
    loader: &ProfileUsageLoader,
    active_auth: Option<&(StoredAuth, AuthIdentity)>,
    preserve_existing_on_unavailable: bool,
) -> Result<()> {
    if let Some((auth, identity)) = active_auth {
        enrich_active_profiles_with_global_auth(
            profiles,
            loader,
            auth,
            identity,
            preserve_existing_on_unavailable,
        )?;
    }

    Ok(())
}

pub(super) fn enrich_active_profiles_with_global_auth(
    profiles: &mut [SavedProfile],
    loader: &ProfileUsageLoader,
    active_auth: &StoredAuth,
    active_identity: &AuthIdentity,
    preserve_existing_on_unavailable: bool,
) -> Result<()> {
    if !profiles.iter().any(|profile| {
        profile
            .identity
            .as_ref()
            .is_some_and(|identity| is_same_user(active_identity, identity))
    }) {
        return Ok(());
    }

    let result =
        runtime::block_on(loader.fetch_profile_usage(active_auth, Some(active_identity)))??;
    let (identity, usage) = match result {
        UsageFetchResult::Available { identity, usage } => (
            identity.or_else(|| Some(active_identity.clone())),
            ProfileUsageState::Available(usage),
        ),
        UsageFetchResult::ReauthNeeded => (
            Some(active_identity.clone()),
            ProfileUsageState::ReauthNeeded,
        ),
        UsageFetchResult::Unavailable { identity } => (
            identity.or_else(|| Some(active_identity.clone())),
            ProfileUsageState::Unavailable,
        ),
    };
    if preserve_existing_on_unavailable && matches!(usage, ProfileUsageState::Unavailable) {
        return Ok(());
    }

    for profile in profiles.iter_mut().filter(|profile| {
        profile
            .identity
            .as_ref()
            .is_some_and(|identity| is_same_user(active_identity, identity))
    }) {
        profile.identity = identity.clone();
        profile.invalid_auth = false;
        profile.usage = usage.clone();
    }

    Ok(())
}

pub(super) fn usage() -> Result<()> {
    let loader = ProfileUsageLoader::new()?;
    let (label, usage) = current_usage_view(&loader, &auth_path()?)?;

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    print_current_usage_table(&mut stdout, &label, &usage)?;

    Ok(())
}

pub(super) fn current_usage_view(
    loader: &ProfileUsageLoader,
    auth_path: &Path,
) -> Result<(String, ProfileUsageState)> {
    if !auth_path.exists() {
        return Err(eyre!("No current codex auth found. Run: cmd codex login"));
    }

    let auth = read_stored_auth(auth_path).wrap_err("Failed to read current codex auth")?;
    let identity =
        read_auth_identity(auth_path).wrap_err("Failed to read current codex auth identity")?;
    let runtime = runtime::current_thread_runtime()?;
    let result = runtime.block_on(loader.fetch_profile_usage(&auth, Some(&identity)))?;

    match result {
        UsageFetchResult::Available { usage, .. } => {
            let label = usage
                .email
                .clone()
                .or_else(|| identity.email.clone())
                .unwrap_or_else(|| "-".into());
            Ok((label, ProfileUsageState::Available(usage)))
        }
        UsageFetchResult::Unavailable { .. } => {
            let label = identity.email.clone().unwrap_or_else(|| "-".into());
            Ok((label, ProfileUsageState::Unavailable))
        }
        UsageFetchResult::ReauthNeeded => Err(eyre!(
            "Current codex auth does not match usage identity. Reauthenticate or switch profiles"
        )),
    }
}

pub(super) fn refresh_profile(profile: &str) -> Result<()> {
    let profile_home = profile_codex_home(profile)?;
    let active_sessions = active_session_markers(&profile_home)?;
    if !active_sessions.is_empty() {
        return Err(eyre!(
            "Profile '{profile}' has {} active codex session(s)",
            active_sessions.len()
        ));
    }

    let profile_auth = profile_home.join("auth.json");
    if !profile_auth.exists() {
        return Err(eyre!("Profile '{profile}' not found. Run: cmd codex list"));
    }

    let launch_auth = read_auth_snapshot(&profile_auth)?;
    let auth = read_stored_auth(&profile_auth)?;
    let refresher = ProfileAuthRefresher::new()?;
    let refreshed_auth =
        runtime::block_on(refresher.refresh_profile_auth(&auth, Some(&launch_auth.identity)))??;
    let refreshed_raw = serde_json::to_vec_pretty(&refreshed_auth)?;

    if write_auth_raw_if_unchanged(&profile_auth, &launch_auth.raw, &refreshed_raw)? {
        println!("Refreshed codex profile: {profile}");
    } else {
        println!("Skipped refreshing codex profile: {profile} (profile auth changed)");
    }

    Ok(())
}

pub(super) fn refresh_all() -> Result<()> {
    let profiles = load_saved_profiles(&profiles_dir()?)?;
    if profiles.is_empty() {
        println!("No profiles. Run: cmd codex login <name>");
        return Ok(());
    }

    let runtime = runtime::current_thread_runtime()?;
    let refresher = ProfileAuthRefresher::new()?;
    let now = Utc::now();
    let mut rows = Vec::new();

    for profile in profiles {
        let profile_home = profile_codex_home(&profile.name)?;
        let active_sessions = active_session_markers(&profile_home)?;
        if !active_sessions.is_empty() {
            rows.push(RefreshAllRow {
                profile: profile.name,
                result: RefreshAllResultKind::Deferred,
                detail: format!("{} active session(s)", active_sessions.len()),
            });
            continue;
        }

        if profile.invalid_auth {
            rows.push(RefreshAllRow {
                profile: profile.name,
                result: RefreshAllResultKind::Invalid,
                detail: "invalid auth".into(),
            });
            continue;
        }

        let auth = match read_stored_auth(&profile.auth_path) {
            Ok(auth) => auth,
            Err(err) => {
                rows.push(RefreshAllRow {
                    profile: profile.name,
                    result: RefreshAllResultKind::Invalid,
                    detail: err.to_string(),
                });
                continue;
            }
        };

        if !needs_proactive_refresh(&auth, now)? {
            rows.push(RefreshAllRow {
                profile: profile.name,
                result: RefreshAllResultKind::Fresh,
                detail: "fresh".into(),
            });
            continue;
        }

        match refresh_profile_auth_if_unchanged(
            &runtime,
            &refresher,
            &profile.auth_path,
            &auth,
            profile.identity.as_ref(),
        ) {
            Ok(true) => rows.push(RefreshAllRow {
                profile: profile.name,
                result: RefreshAllResultKind::Refreshed,
                detail: "refreshed".into(),
            }),
            Ok(false) => rows.push(RefreshAllRow {
                profile: profile.name,
                result: RefreshAllResultKind::Failed,
                detail: "profile auth changed".into(),
            }),
            Err(err) => rows.push(RefreshAllRow {
                profile: profile.name,
                result: RefreshAllResultKind::Failed,
                detail: err.to_string(),
            }),
        }
    }

    print_refresh_all_rows(&rows);

    if rows.iter().any(|row| {
        matches!(
            row.result,
            RefreshAllResultKind::Failed | RefreshAllResultKind::Invalid
        )
    }) {
        return Err(eyre!("Some profiles could not be refreshed"));
    }

    Ok(())
}

pub(super) fn switch_default_profile(profile: &str) -> Result<()> {
    let profile_auth = profile_codex_home(profile)?.join("auth.json");
    if !profile_auth.exists() {
        return Err(eyre!("Profile '{profile}' not found. Run: cmd codex list"));
    }

    replace_global_auth_with_profile(&profile_auth, &auth_path()?)?;

    match notify_remodex_bridge_profile_switch(profile) {
        Ok(RemodexBridgeSwitchOutcome::Switched) => {
            println!("Switched codex default profile to {profile} and updated phodex-bridge");
        }
        Ok(RemodexBridgeSwitchOutcome::NotRunning) => {
            println!("Switched codex default profile to {profile}");
        }
        Err(err) => {
            return Err(eyre!(
                "Switched codex default profile to {profile}, but failed to update phodex-bridge: {err}"
            ));
        }
    }

    Ok(())
}

fn resolve_group_name(group: Option<&str>, default: &str, kind: &str) -> Result<String> {
    let group = group.unwrap_or(default);
    validate_group_name(group, kind)?;
    Ok(group.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchGroups {
    pub config: String,
    pub resume: String,
}

pub fn resolve_launch_groups(
    profile: &str,
    resume_group: Option<&str>,
    config_group: Option<&str>,
) -> Result<LaunchGroups> {
    Ok(LaunchGroups {
        config: resolve_group_name(config_group, profile, "config")?,
        resume: resolve_group_name(resume_group, "shared", "resume")?,
    })
}

fn refresh_profile_auth_if_unchanged(
    runtime: &tokio::runtime::Runtime,
    refresher: &ProfileAuthRefresher,
    profile_auth: &Path,
    auth: &StoredAuth,
    expected_identity: Option<&AuthIdentity>,
) -> Result<bool> {
    let launch_auth = read_auth_snapshot(profile_auth)?;
    let refreshed_auth =
        runtime.block_on(refresher.refresh_profile_auth(auth, expected_identity))?;
    let refreshed_raw = serde_json::to_vec_pretty(&refreshed_auth)?;
    write_auth_raw_if_unchanged(profile_auth, &launch_auth.raw, &refreshed_raw)
}

pub(super) fn delete(profile: &str, yes: bool) -> Result<()> {
    let profile_home = profile_codex_home(profile)?;
    if !profile_home.exists() {
        return Err(eyre!("Profile '{profile}' not found. Run: cmd codex list"));
    }

    if !yes && !prompt_for_confirmation(&format!("Delete codex profile '{profile}'?"))? {
        println!("Skipped deleting codex profile: {profile}");
        return Ok(());
    }

    delete_profile_home(&profile_home)?;
    println!("Deleted codex profile: {profile}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_launch_selection_uses_displayed_percent_delta() {
        let now = Utc::now();
        let profiles = vec![
            saved_profile_with_usage(
                "raw-overpaced",
                25.0,
                reset_at_for_elapsed(now, UsageWindowKind::Primary, 0.2),
                50.0,
                reset_at_for_elapsed(now, UsageWindowKind::Secondary, 0.5),
                10.0,
            ),
            saved_profile_with_usage(
                "raw-underpaced",
                15.0,
                reset_at_for_elapsed(now, UsageWindowKind::Primary, 0.2),
                50.0,
                reset_at_for_elapsed(now, UsageWindowKind::Secondary, 0.5),
                1.0,
            ),
        ];

        let selected = select_auto_launch_profile(&profiles).expect("selected profile");

        assert_eq!(selected.name, "raw-underpaced");
    }

    fn saved_profile_with_usage(
        name: &str,
        five_hour: f64,
        five_hour_reset_at: i64,
        weekly: f64,
        weekly_reset_at: i64,
        limit_multiplier: f64,
    ) -> SavedProfile {
        SavedProfile {
            name: name.into(),
            auth_path: PathBuf::from(format!("/tmp/{name}.json")),
            identity: None,
            invalid_auth: false,
            usage: ProfileUsageState::Available(ProfileUsageSnapshot {
                user_id: None,
                account_id: None,
                email: None,
                plan_type: None,
                primary: Some(UsageWindowSnapshot {
                    used_percent: five_hour,
                    reset_at: Some(five_hour_reset_at),
                    limit_multiplier,
                }),
                secondary: Some(UsageWindowSnapshot {
                    used_percent: weekly,
                    reset_at: Some(weekly_reset_at),
                    limit_multiplier,
                }),
            }),
        }
    }

    fn reset_at_for_elapsed(
        now: chrono::DateTime<Utc>,
        kind: UsageWindowKind,
        elapsed_fraction: f64,
    ) -> i64 {
        let duration = match kind {
            UsageWindowKind::Primary => chrono::Duration::hours(5),
            UsageWindowKind::Secondary => chrono::Duration::days(7),
        };
        let remaining_seconds =
            ((1.0 - elapsed_fraction) * duration.num_seconds() as f64).round() as i64;

        (now + chrono::Duration::seconds(remaining_seconds)).timestamp()
    }
}
