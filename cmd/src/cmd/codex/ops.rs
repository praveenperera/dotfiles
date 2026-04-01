use super::*;
use crate::{fsutil, runtime};

pub(super) fn launch(profile_or_arg: Option<&OsString>, args: &[OsString]) -> Result<()> {
    let mut profiles = load_saved_profiles(&profiles_dir()?)?;
    let target = resolve_launch_target(profile_or_arg, args, &profiles);

    match target {
        LaunchTarget::Explicit { profile, args } => {
            let label = profiles
                .iter()
                .find(|saved_profile| saved_profile.name == profile)
                .map(saved_profile_label)
                .unwrap_or_else(|| "-".into());
            launch_with_profile(&profile, &label, &args)
        }
        LaunchTarget::Auto { args } => {
            enrich_profile_usage(&mut profiles)?;
            let profile = select_auto_launch_profile(&profiles)?;
            let label = saved_profile_label(profile);
            launch_with_profile(&profile.name, &label, &args)
        }
    }
}

fn launch_with_profile(profile: &str, label: &str, args: &[OsString]) -> Result<()> {
    println!("{}", format_launch_banner(profile, label));

    let shared_codex_home = codex_dir()?;
    let profile_home = profile_codex_home(profile)?;
    let profile_auth = profile_home.join("auth.json");
    if !profile_auth.exists() {
        return Err(eyre!(
            "Profile '{profile}' not found. Run: cmd codex login {profile}"
        ));
    }

    sync_profile_codex_home(&profile_home, &shared_codex_home)?;
    let launch_home = create_launch_home(&profile_home)?;
    let launch_auth = read_auth_snapshot(&profile_auth)?;
    sync_launch_codex_home(&launch_home, &shared_codex_home, &profile_auth)?;
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
    let status = child.wait()?;
    fsutil::remove_existing_path(&session_marker_path)?;
    promote_launch_auth_if_unchanged(&profile_auth, &launch_auth, &launch_home.join("auth.json"))?;
    std::process::exit(status.code().unwrap_or(1));
}

pub(super) fn resolve_launch_target(
    profile_or_arg: Option<&OsString>,
    args: &[OsString],
    profiles: &[SavedProfile],
) -> LaunchTarget {
    let explicit_profile = profile_or_arg
        .and_then(|value| value.to_str())
        .filter(|profile| {
            profiles
                .iter()
                .any(|saved_profile| saved_profile.name == *profile)
        });

    if let Some(profile) = explicit_profile {
        return LaunchTarget::Explicit {
            profile: profile.into(),
            args: args.to_vec(),
        };
    }

    let mut resolved_args = Vec::with_capacity(args.len() + usize::from(profile_or_arg.is_some()));
    if let Some(value) = profile_or_arg {
        resolved_args.push(value.clone());
    }
    resolved_args.extend(args.iter().cloned());

    LaunchTarget::Auto {
        args: resolved_args,
    }
}

pub(super) fn select_auto_launch_profile(profiles: &[SavedProfile]) -> Result<&SavedProfile> {
    let mut cool_candidates = Vec::new();
    let mut hot_candidates = Vec::new();

    for profile in profiles {
        let Some(candidate) = launch_candidate(profile) else {
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
            eyre!(
                "No profiles with usable usage data found. Run: cmd codex list or specify a saved profile explicitly"
            )
        })
}

pub(super) fn saved_profile_label(profile: &SavedProfile) -> String {
    profile
        .identity
        .as_ref()
        .map(best_label)
        .unwrap_or_else(|| "-".into())
}

pub(super) fn format_launch_banner(profile: &str, label: &str) -> String {
    format!(
        "{} {} {} - {}",
        "launching".green().bold(),
        "profile".blue().bold(),
        profile.cyan().bold(),
        label.yellow()
    )
}

struct LaunchCandidate<'a> {
    profile: &'a SavedProfile,
    weekly: f64,
    five_hour: f64,
    score: f64,
}

fn launch_candidate(profile: &SavedProfile) -> Option<LaunchCandidate<'_>> {
    if profile.invalid_auth {
        return None;
    }

    let ProfileUsageState::Available(usage) = &profile.usage else {
        return None;
    };

    let five_hour = usage.primary.as_ref()?.used_percent;
    let weekly = usage.secondary.as_ref()?.used_percent;

    Some(LaunchCandidate {
        profile,
        weekly,
        five_hour,
        score: weekly * 2.0 + five_hour,
    })
}

fn compare_launch_candidates(
    left: &LaunchCandidate<'_>,
    right: &LaunchCandidate<'_>,
) -> std::cmp::Ordering {
    left.score
        .total_cmp(&right.score)
        .then_with(|| left.weekly.total_cmp(&right.weekly))
        .then_with(|| left.five_hour.total_cmp(&right.five_hour))
        .then_with(|| left.profile.name.cmp(&right.profile.name))
}

pub(super) fn login(profile: &str, device_auth: bool) -> Result<()> {
    let shared_codex_home = codex_dir()?;
    let staged_home = tempfile::tempdir()?;

    sync_profile_codex_home(staged_home.path(), &shared_codex_home)?;

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

    let identity = read_auth_identity(&auth).wrap_err("Failed to read codex auth identity")?;
    let profiles_dir = profiles_dir()?;
    let profiles = load_saved_profiles(&profiles_dir)?;
    let conflicts = conflicting_profiles(&profiles, profile, &identity);

    let replace_conflicts = if conflicts.is_empty() {
        false
    } else {
        println!(
            "This OpenAI user is already saved as {}",
            conflicts.join(", ")
        );
        prompt_for_replacement(&conflicts, profile)?
    };

    let outcome = if conflicts.is_empty() || replace_conflicts {
        let profile_home = profile_codex_home(profile)?;
        sync_profile_codex_home(&profile_home, &shared_codex_home)?;
        save_profile_auth(profile, &auth, &profiles_dir, &conflicts, replace_conflicts)?
    } else {
        SaveProfileOutcome::SkippedConflict
    };

    match outcome {
        SaveProfileOutcome::Saved => println!("Saved codex profile: {profile}"),
        SaveProfileOutcome::SkippedConflict => {
            println!("Skipped saving codex profile: {profile}");
        }
    }

    Ok(())
}

pub(super) fn list(verbose: bool) -> Result<()> {
    let mut profiles = load_saved_profiles(&profiles_dir()?)?;
    if profiles.is_empty() {
        println!("No profiles. Run: cmd codex login <name>");
        return Ok(());
    }

    enrich_profile_usage(&mut profiles)?;

    let active_identity = auth_path()
        .ok()
        .and_then(|path| read_auth_identity(&path).ok());
    let rows = build_profile_rows(&profiles, active_identity.as_ref());
    if verbose {
        print_verbose_profile_table(&rows);
    } else {
        print_compact_profile_table(&rows);
    }

    Ok(())
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
