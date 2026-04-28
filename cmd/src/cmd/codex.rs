mod auth;
mod fs;
mod ops;
mod table;
mod usage;

use base64::{
    engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD},
    Engine as _,
};
use chrono::{Local, TimeZone, Utc};
use clap::{Args, Parser, Subcommand};
use colored::Colorize;
use eyre::{eyre, Result, WrapErr};
use futures::stream::{self, StreamExt};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE, USER_AGENT},
    Client as HttpClient, StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ffi::OsString;
use std::fs as stdfs;
use std::io::{self, ErrorKind, Read, Write};
use std::net::Shutdown;
use std::os::unix::fs::symlink;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;
use xshell::Shell;

use auth::*;
use fs::*;
use ops::*;
use table::*;
use usage::*;

#[derive(Debug, Clone, Args)]
pub struct Codex {
    #[command(subcommand)]
    pub subcommand: CodexCmd,
}

#[derive(Debug, Clone, Parser)]
struct CodexCli {
    #[command(subcommand)]
    subcommand: CodexCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum CodexCmd {
    /// Launch codex with an optional profile
    Launch {
        /// Optional profile name, or first codex argument when auto-selecting
        profile_or_arg: Option<OsString>,

        /// Resume-state group to use for session continuity
        #[arg(short = 'r', long)]
        resume_group: Option<String>,

        /// Config-state group to use for model and UI preferences
        #[arg(short = 'c', long)]
        config_group: Option<String>,

        /// Auto-select a profile other than the active one
        #[arg(long)]
        other: bool,

        /// Arguments to pass to codex
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },

    /// Login and save a new profile
    Login {
        /// Profile name to save
        profile: String,

        /// Use the device auth flow
        #[arg(short = 'd', long)]
        device_auth: bool,
    },

    /// List saved profiles and their identities
    #[command(visible_alias = "ls")]
    List {
        /// Show full profile details
        #[arg(short, long)]
        verbose: bool,
    },

    /// Show usage for the current global codex auth
    Usage,

    /// Refresh a saved profile's auth
    #[command(visible_alias = "rp")]
    RefreshProfile {
        /// Profile name to refresh
        profile: String,
    },

    /// Refresh stale saved profiles that are not currently in use
    #[command(visible_alias = "ra")]
    RefreshAll,

    /// Switch the default global codex profile
    #[command(visible_alias = "switch-default")]
    Switch {
        /// Profile name to switch to
        profile: String,
    },

    /// Delete a saved profile
    #[command(visible_alias = "rm")]
    Delete {
        /// Profile name to delete
        profile: String,

        /// Skip the confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct StoredAuth {
    #[serde(rename = "OPENAI_API_KEY")]
    openai_api_key: Option<String>,
    auth_mode: Option<String>,
    last_refresh: Option<String>,
    tokens: Option<StoredTokens>,
    #[serde(flatten, default)]
    extra: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct StoredTokens {
    account_id: Option<String>,
    id_token: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,
    #[serde(flatten, default)]
    extra: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct IdTokenClaims {
    sub: Option<String>,
    email: Option<String>,
    name: Option<String>,
    auth_provider: Option<String>,
    #[serde(rename = "https://api.openai.com/auth")]
    openai_auth: Option<OpenAiClaims>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct OpenAiClaims {
    user_id: Option<String>,
    chatgpt_account_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct RemodexBridgeSwitchRequest<'a> {
    method: &'a str,
    profile: &'a str,
}

#[derive(Debug, Deserialize)]
struct RemodexBridgeSwitchResponse {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemodexBridgeSwitchOutcome {
    Switched,
    NotRunning,
}

#[derive(Debug, Clone, Deserialize)]
struct StandardJwtClaims {
    exp: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AuthIdentity {
    auth_mode: Option<String>,
    subject: Option<String>,
    user_id: Option<String>,
    chatgpt_account_id: Option<String>,
    email: Option<String>,
    name: Option<String>,
    auth_provider: Option<String>,
}

#[derive(Debug, Clone)]
struct SavedProfile {
    name: String,
    auth_path: PathBuf,
    identity: Option<AuthIdentity>,
    invalid_auth: bool,
    usage: ProfileUsageState,
}

#[derive(Debug, Clone)]
struct ProfileRow {
    profile: String,
    label: String,
    total_dedupe_key: Option<String>,
    provider: String,
    user: String,
    account: String,
    plan: String,
    five_hour: String,
    five_hour_reset: String,
    five_hour_compact: String,
    five_hour_style: LimitStyleKind,
    five_hour_usage: Option<UsageWindowSnapshot>,
    weekly: String,
    weekly_reset: String,
    weekly_compact: String,
    weekly_style: LimitStyleKind,
    weekly_usage: Option<UsageWindowSnapshot>,
    status: ProfileStatus,
}

#[derive(Debug, Clone, Default)]
struct ProfileStatus {
    items: Vec<ProfileStatusItem>,
}

#[derive(Debug, Clone)]
enum ProfileStatusItem {
    Active,
    SameUser(Vec<String>),
    SharedAccount(Vec<String>),
    InvalidAuth,
    ReauthNeeded,
    UsageUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProfileStyleKind {
    Normal,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LimitStyleKind {
    Normal,
    Success,
    Warning,
    Caution,
    Error,
    Critical,
}

#[derive(Debug, Clone)]
enum ProfileUsageState {
    Unchecked,
    Available(ProfileUsageSnapshot),
    ReauthNeeded,
    Unavailable,
}

#[derive(Debug, Clone)]
struct ProfileUsageSnapshot {
    user_id: Option<String>,
    account_id: Option<String>,
    email: Option<String>,
    plan_type: Option<String>,
    primary: Option<UsageWindowSnapshot>,
    secondary: Option<UsageWindowSnapshot>,
}

#[derive(Debug, Clone)]
struct UsageWindowSnapshot {
    used_percent: f64,
    reset_at: Option<i64>,
    limit_multiplier: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionMarker {
    pid: u32,
    started_at: chrono::DateTime<Utc>,
    launch_home: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefreshAllResultKind {
    Refreshed,
    Fresh,
    Deferred,
    Invalid,
    Failed,
}

#[derive(Debug, Clone)]
struct RefreshAllRow {
    profile: String,
    result: RefreshAllResultKind,
    detail: String,
}

#[derive(Debug, Clone)]
struct ProfileUsageUpdate {
    profile: String,
    identity: Option<AuthIdentity>,
    invalid_auth: bool,
    usage: ProfileUsageState,
}

#[derive(Debug, Clone)]
struct ProfileUsageLoader {
    http: HttpClient,
    usage_url: String,
}

#[derive(Debug, Clone)]
struct ProfileAuthRefresher {
    http: HttpClient,
    refresh_url: String,
}

#[derive(Debug, Deserialize)]
struct UsageResponse {
    email: Option<String>,
    user_id: Option<String>,
    account_id: Option<String>,
    plan_type: Option<String>,
    rate_limit: Option<UsageRateLimit>,
}

#[derive(Debug, Deserialize)]
struct UsageRateLimit {
    primary_window: Option<UsageWindowResponse>,
    secondary_window: Option<UsageWindowResponse>,
}

#[derive(Debug, Deserialize)]
struct UsageWindowResponse {
    used_percent: f64,
    reset_at: i64,
}

#[derive(Debug, Deserialize)]
struct RefreshResponse {
    id_token: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,
}

#[derive(Debug, Serialize)]
struct RefreshRequest<'a> {
    client_id: &'static str,
    grant_type: &'static str,
    refresh_token: &'a str,
    scope: &'static str,
}

#[derive(Debug)]
enum UsageFetchResult {
    Available {
        identity: Option<AuthIdentity>,
        usage: ProfileUsageSnapshot,
    },
    ReauthNeeded,
    Unavailable {
        identity: Option<AuthIdentity>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProfileTableWidths {
    profile: usize,
    label: usize,
    provider: usize,
    user: usize,
    account: usize,
    plan: usize,
    five_hour: usize,
    five_hour_reset: usize,
    weekly: usize,
    weekly_reset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CompactProfileTableWidths {
    profile: usize,
    label: usize,
    five_hour: usize,
    weekly: usize,
}

#[derive(Debug, Clone)]
struct AuthSnapshot {
    raw: Vec<u8>,
    identity: AuthIdentity,
}

#[derive(Debug, Clone)]
enum LaunchAuthMode {
    GlobalShared {
        global_auth: PathBuf,
    },
    ProfileCopy {
        profile_auth: PathBuf,
        launch_auth: AuthSnapshot,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LaunchTarget {
    Explicit {
        profile: String,
        resume_group: Option<String>,
        config_group: Option<String>,
        args: Vec<OsString>,
    },
    Auto {
        resume_group: Option<String>,
        config_group: Option<String>,
        other: bool,
        args: Vec<OsString>,
    },
}

const CHATGPT_USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
const CHATGPT_REFRESH_URL: &str = "https://auth.openai.com/oauth/token";
const CHATGPT_REFRESH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const USAGE_FETCH_CONCURRENCY: usize = 4;
const USAGE_FETCH_TIMEOUT: Duration = Duration::from_secs(5);
const PROFILE_REFRESH_FALLBACK_DAYS: i64 = 7;

pub fn run_with_args(_sh: &Shell, args: &[OsString]) -> Result<()> {
    let flags = parse_raw_args(args)?;
    run_with_flags(_sh, flags)
}

pub fn run_with_flags(_sh: &Shell, flags: Codex) -> Result<()> {
    match flags.subcommand {
        CodexCmd::Launch {
            profile_or_arg,
            resume_group,
            config_group,
            other,
            args,
        } => launch(
            profile_or_arg.as_ref(),
            resume_group.as_deref(),
            config_group.as_deref(),
            other,
            &args,
        ),
        CodexCmd::Login {
            profile,
            device_auth,
        } => login(&profile, device_auth),
        CodexCmd::List { verbose } => list(verbose),
        CodexCmd::Usage => usage(),
        CodexCmd::RefreshProfile { profile } => refresh_profile(&profile),
        CodexCmd::RefreshAll => refresh_all(),
        CodexCmd::Switch { profile } => switch_default_profile(&profile),
        CodexCmd::Delete { profile, yes } => delete(&profile, yes),
    }
}

fn parse_raw_args(args: &[OsString]) -> Result<Codex> {
    if let Some(flags) = parse_launch_with_forced_auto_selection(args)? {
        return Ok(flags);
    }

    let mut full_args = vec![OsString::from("codex")];
    full_args.extend_from_slice(args);

    match CodexCli::try_parse_from(full_args) {
        Ok(flags) => Ok(Codex {
            subcommand: flags.subcommand,
        }),
        Err(err) => {
            let _ = err.print();
            std::process::exit(err.exit_code());
        }
    }
}

fn parse_launch_with_forced_auto_selection(args: &[OsString]) -> Result<Option<Codex>> {
    if args.first().and_then(|arg| arg.to_str()) != Some("launch") {
        return Ok(None);
    }

    let Some(separator_idx) = args.iter().position(|arg| arg.to_str() == Some("--")) else {
        return Ok(None);
    };

    let launch_prefix = &args[1..separator_idx];
    let mut parse_args = vec![OsString::from("codex"), OsString::from("launch")];
    parse_args.extend_from_slice(launch_prefix);
    let flags = match CodexCli::try_parse_from(parse_args) {
        Ok(flags) => flags,
        Err(err) => {
            let _ = err.print();
            std::process::exit(err.exit_code());
        }
    };

    let CodexCmd::Launch {
        profile_or_arg,
        resume_group,
        config_group,
        other,
        ..
    } = flags.subcommand
    else {
        unreachable!("launch prefix must parse into launch subcommand");
    };
    if profile_or_arg.is_some() {
        return Ok(None);
    }

    Ok(Some(Codex {
        subcommand: CodexCmd::Launch {
            profile_or_arg: None,
            resume_group,
            config_group,
            other,
            args: args[separator_idx + 1..].to_vec(),
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::{
        active_session_markers, build_profile_rows, create_launch_home, current_usage_view,
        delete_profile_home, enrich_active_profiles_with_global_auth, format_launch_banner,
        launch_banner_details, needs_proactive_refresh, parse_auth_identity, parse_jwt_expiration,
        parse_raw_args, prepare_config_group_home, prepare_resume_group_home,
        print_current_usage_table, promote_launch_auth_if_unchanged, read_auth_snapshot,
        read_stored_auth, replace_global_auth_with_profile, resolve_launch_auth_mode,
        resolve_launch_groups, resolve_launch_target, save_profile_auth,
        select_auto_launch_profile, select_auto_launch_profile_except, sync_launch_codex_home,
        sync_login_codex_home, validate_group_name, write_auth_raw_if_unchanged,
        write_session_marker, AuthIdentity, CodexCmd, LaunchAuthMode, LaunchGroups, LaunchTarget,
        LimitStyleKind, ProfileAuthRefresher, ProfileStyleKind, ProfileUsageLoader,
        ProfileUsageSnapshot, ProfileUsageState, SavedProfile, StoredAuth, UsageFetchResult,
        UsageWindowSnapshot,
    };
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use chrono::{Local, TimeZone, Utc};
    use serde_json::json;
    use std::os::unix::fs::symlink;
    use std::path::Path;
    use std::{ffi::OsString, fs, sync::Mutex};
    use tempfile::tempdir;
    use wiremock::{
        matchers::{header, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    static COLOR_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn parses_auth_identity_from_id_token() {
        let auth = auth_json(&TestIdentity {
            subject: "google-oauth2|1234",
            user_id: "user-1234",
            account_id: "acct-1234",
            email: Some("praveen@example.com"),
            name: Some("Praveen Perera"),
            auth_provider: Some("google"),
        });

        let identity = parse_auth_identity(&auth).unwrap();

        assert_eq!(identity.subject.as_deref(), Some("google-oauth2|1234"));
        assert_eq!(identity.user_id.as_deref(), Some("user-1234"));
        assert_eq!(identity.chatgpt_account_id.as_deref(), Some("acct-1234"));
        assert_eq!(identity.email.as_deref(), Some("praveen@example.com"));
        assert_eq!(identity.name.as_deref(), Some("Praveen Perera"));
        assert_eq!(identity.auth_provider.as_deref(), Some("google"));
    }

    #[test]
    fn save_profile_auth_preserves_existing_same_user_profiles() {
        let dir = tempdir().unwrap();
        let profiles_dir = dir.path().join("profiles");
        let old_dir = profiles_dir.join("old");
        let auth_path = dir.path().join("auth.json");

        fs::create_dir_all(&old_dir).unwrap();
        let id = TestIdentity {
            subject: "sub-1",
            user_id: "user-1",
            account_id: "acct-1",
            email: None,
            name: None,
            auth_provider: None,
        };
        fs::write(old_dir.join("auth.json"), auth_json(&id)).unwrap();
        fs::write(&auth_path, auth_json(&id)).unwrap();

        save_profile_auth("new", &auth_path, &profiles_dir).unwrap();

        assert!(old_dir.exists());
        assert_eq!(
            fs::read_to_string(profiles_dir.join("new").join("auth.json")).unwrap(),
            auth_json(&id)
        );
    }

    #[test]
    fn save_profile_auth_overwrites_existing_profile_auth() {
        let dir = tempdir().unwrap();
        let profiles_dir = dir.path().join("profiles");
        let profile_dir = profiles_dir.join("new");
        let auth_path = dir.path().join("auth.json");
        let new_auth = auth_json(&TestIdentity {
            subject: "sub-1",
            user_id: "user-1",
            account_id: "acct-1",
            email: Some("new@example.com"),
            name: None,
            auth_provider: None,
        });

        fs::create_dir_all(&profile_dir).unwrap();
        fs::write(profile_dir.join("auth.json"), "old-auth").unwrap();
        fs::write(&auth_path, &new_auth).unwrap();

        save_profile_auth("new", &auth_path, &profiles_dir).unwrap();

        assert_eq!(
            fs::read_to_string(profiles_dir.join("new").join("auth.json")).unwrap(),
            new_auth
        );
    }

    #[test]
    fn delete_removes_profile_home_without_touching_global_auth() {
        let dir = tempdir().unwrap();
        let global_auth = dir.path().join(".codex").join("auth.json");
        let profile_home = dir.path().join(".codex").join("profiles").join("work");

        fs::create_dir_all(&profile_home).unwrap();
        fs::create_dir_all(global_auth.parent().unwrap()).unwrap();
        fs::write(&global_auth, "global-auth").unwrap();
        fs::write(profile_home.join("auth.json"), "profile-auth").unwrap();

        delete_profile_home(&profile_home).unwrap();

        assert!(!profile_home.exists());
        assert_eq!(fs::read_to_string(global_auth).unwrap(), "global-auth");
    }

    #[test]
    fn delete_errors_for_missing_profile() {
        let dir = tempdir().unwrap();
        let profile_home = dir.path().join(".codex").join("profiles").join("missing");

        let err = delete_profile_home(&profile_home).unwrap_err();

        assert!(err.to_string().contains("Profile home not found"));
    }

    #[test]
    fn replace_global_auth_with_profile_overwrites_global_auth() {
        let dir = tempdir().unwrap();
        let profile_auth = dir.path().join("profiles").join("w").join("auth.json");
        let global_auth = dir.path().join(".codex").join("auth.json");

        fs::create_dir_all(profile_auth.parent().unwrap()).unwrap();
        fs::create_dir_all(global_auth.parent().unwrap()).unwrap();
        fs::write(&profile_auth, "profile-auth").unwrap();
        fs::write(&global_auth, "global-auth").unwrap();

        replace_global_auth_with_profile(&profile_auth, &global_auth).unwrap();

        assert_eq!(fs::read_to_string(global_auth).unwrap(), "profile-auth");
    }

    #[test]
    fn format_reset_timestamp_uses_am_pm_for_same_day() {
        let captured_at = Local.with_ymd_and_hms(2026, 3, 31, 9, 15, 0).unwrap();
        let reset_at = Local.with_ymd_and_hms(2026, 3, 31, 17, 5, 0).unwrap();

        let formatted = super::format_reset_timestamp(reset_at, captured_at);

        assert_eq!(formatted, "5:05 PM");
    }

    #[test]
    fn format_reset_timestamp_uses_am_pm_for_future_day() {
        let captured_at = Local.with_ymd_and_hms(2026, 3, 31, 9, 15, 0).unwrap();
        let reset_at = Local.with_ymd_and_hms(2026, 4, 2, 0, 30, 0).unwrap();

        let formatted = super::format_reset_timestamp(reset_at, captured_at);

        assert_eq!(formatted, "12:30 AM on 2 Apr");
    }

    #[test]
    fn format_reset_timestamp_compact_uses_time_only_for_primary_window() {
        let captured_at = Local.with_ymd_and_hms(2026, 3, 31, 9, 15, 0).unwrap();
        let reset_at = Local.with_ymd_and_hms(2026, 4, 2, 0, 30, 0).unwrap();

        let formatted = super::format_reset_timestamp_compact(
            reset_at,
            captured_at,
            super::UsageWindowKind::Primary,
        );

        assert_eq!(formatted, "12:30 AM");
    }

    #[test]
    fn format_reset_timestamp_compact_uses_calendar_day_for_future_secondary_window() {
        let captured_at = Local.with_ymd_and_hms(2026, 3, 31, 9, 15, 0).unwrap();
        let reset_at = Local.with_ymd_and_hms(2026, 4, 2, 0, 30, 0).unwrap();

        let formatted = super::format_reset_timestamp_compact(
            reset_at,
            captured_at,
            super::UsageWindowKind::Secondary,
        );

        assert_eq!(formatted, "Thu 2 Apr 12:30 AM");
    }

    #[test]
    fn current_usage_reset_timestamp_uses_weekday_without_date_for_future_secondary_window() {
        let captured_at = Local.with_ymd_and_hms(2026, 3, 31, 9, 15, 0).unwrap();
        let reset_at = Local.with_ymd_and_hms(2026, 4, 2, 0, 30, 0).unwrap();

        let formatted = super::format_current_usage_reset_timestamp(
            reset_at,
            captured_at,
            super::UsageWindowKind::Secondary,
        );

        assert_eq!(formatted, "Thu 12:30 AM");
    }

    #[test]
    fn current_usage_window_compact_includes_pace_and_weekday_reset() {
        let current_local = Local.with_ymd_and_hms(2026, 3, 31, 9, 15, 0).unwrap();
        let current_utc = current_local.with_timezone(&Utc);
        let usage = ProfileUsageState::Available(ProfileUsageSnapshot {
            user_id: Some("user-1".into()),
            account_id: Some("acct-1".into()),
            email: Some("praveen@example.com".into()),
            plan_type: Some("plus".into()),
            primary: Some(UsageWindowSnapshot {
                used_percent: 42.0,
                reset_at: Some(reset_at_for_elapsed(
                    current_utc,
                    super::UsageWindowKind::Primary,
                    0.5,
                )),
                limit_multiplier: 1.0,
            }),
            secondary: Some(UsageWindowSnapshot {
                used_percent: 73.0,
                reset_at: Some(
                    Local
                        .with_ymd_and_hms(2026, 4, 2, 0, 30, 0)
                        .unwrap()
                        .timestamp(),
                ),
                limit_multiplier: 1.0,
            }),
        });

        let primary = super::current_usage_window_compact(
            &usage,
            super::UsageWindowKind::Primary,
            current_local,
            current_utc,
        );
        let weekly = super::current_usage_window_compact(
            &usage,
            super::UsageWindowKind::Secondary,
            current_local,
            current_utc,
        );

        assert_eq!(primary, " 42% (-8%) (11:45 AM)");
        assert_eq!(weekly, " 73% (-4%) (Thu 12:30 AM)");
    }

    #[test]
    fn usage_window_compact_wraps_reset_in_parentheses() {
        let usage =
            ProfileUsageState::Available(usage_snapshot("plus", "user-1", "acct-1", 42.0, 73.0));

        let formatted = super::usage_window_compact(&usage, super::UsageWindowKind::Primary);

        assert!(formatted.starts_with(" 42% ("));
        assert!(formatted.ends_with(')'));
    }

    #[test]
    fn usage_window_reset_hides_primary_reset_for_zero_percent_window() {
        let usage =
            ProfileUsageState::Available(usage_snapshot("plus", "user-1", "acct-1", 0.0, 73.0));

        let formatted = super::usage_window_reset(&usage, super::UsageWindowKind::Primary);

        assert_eq!(formatted, "-");
    }

    #[test]
    fn usage_window_compact_hides_primary_reset_for_zero_percent_window() {
        let usage =
            ProfileUsageState::Available(usage_snapshot("plus", "user-1", "acct-1", 0.0, 73.0));

        let formatted = super::usage_window_compact(&usage, super::UsageWindowKind::Primary);

        assert_eq!(formatted, "  0%");
    }

    #[test]
    fn usage_window_reset_hides_weekly_reset_for_zero_percent_window() {
        let usage =
            ProfileUsageState::Available(usage_snapshot("plus", "user-1", "acct-1", 42.0, 0.0));

        let formatted = super::usage_window_reset(&usage, super::UsageWindowKind::Secondary);

        assert_eq!(formatted, "-");
    }

    #[test]
    fn usage_window_compact_hides_weekly_reset_for_zero_percent_window() {
        let usage =
            ProfileUsageState::Available(usage_snapshot("plus", "user-1", "acct-1", 42.0, 0.0));

        let formatted = super::usage_window_compact(&usage, super::UsageWindowKind::Secondary);

        assert_eq!(formatted, "  0%");
    }

    #[test]
    fn format_compact_percent_right_aligns_numeric_part() {
        assert_eq!(super::format_compact_percent("0%"), "  0%");
        assert_eq!(super::format_compact_percent("42%"), " 42%");
        assert_eq!(super::format_compact_percent("100%"), "100%");
    }

    #[test]
    fn parse_jwt_expiration_reads_exp_claim() {
        let expires_at = Utc.with_ymd_and_hms(2026, 4, 3, 12, 0, 0).unwrap();
        let jwt = access_jwt(Some(expires_at.timestamp()));

        let parsed = parse_jwt_expiration(&jwt).unwrap();

        assert_eq!(parsed, Some(expires_at));
    }

    #[test]
    fn proactive_refresh_uses_access_token_expiration_when_available() {
        let now = Utc.with_ymd_and_hms(2026, 4, 1, 12, 0, 0).unwrap();
        let auth = stored_auth_with_access_token_and_last_refresh(
            Some(access_jwt(Some(now.timestamp() - 60))),
            Some(Utc.with_ymd_and_hms(2026, 3, 31, 12, 0, 0).unwrap()),
        );

        assert!(needs_proactive_refresh(&auth, now).unwrap());
    }

    #[test]
    fn proactive_refresh_falls_back_to_last_refresh_when_access_token_is_not_jwt() {
        let now = Utc.with_ymd_and_hms(2026, 4, 1, 12, 0, 0).unwrap();
        let auth = stored_auth_with_access_token_and_last_refresh(
            Some("opaque-access-token".into()),
            Some(Utc.with_ymd_and_hms(2026, 3, 24, 11, 59, 59).unwrap()),
        );

        assert!(needs_proactive_refresh(&auth, now).unwrap());
    }

    #[test]
    fn proactive_refresh_treats_missing_signals_as_stale() {
        let now = Utc.with_ymd_and_hms(2026, 4, 1, 12, 0, 0).unwrap();
        let auth = stored_auth_with_access_token_and_last_refresh(None, None);

        assert!(needs_proactive_refresh(&auth, now).unwrap());
    }

    #[test]
    fn active_session_markers_prunes_stale_or_non_codex_processes() {
        let dir = tempdir().unwrap();
        let profile_home = dir.path().join("profiles").join("a");
        let marker_path =
            write_session_marker(&profile_home, std::process::id(), dir.path()).unwrap();

        let active = active_session_markers(&profile_home).unwrap();

        assert!(active.is_empty());
        assert!(!marker_path.exists());
    }

    #[test]
    fn active_session_markers_does_not_touch_rollout_dirs() {
        let dir = tempdir().unwrap();
        let profile_home = dir.path().join("profiles").join("a");
        let shared_sessions = dir.path().join("shared-sessions");
        let rollout_dir = shared_sessions.join("2026").join("03").join("31");
        let rollout_path = rollout_dir.join("rollout-2026-03-31T12-00-00-thread.jsonl");
        let markers_dir = profile_home.join(".session-markers");
        let stale_marker = markers_dir.join("stale.json");

        fs::create_dir_all(&profile_home).unwrap();
        fs::create_dir_all(&rollout_dir).unwrap();
        fs::write(&rollout_path, "{\"dummy\":true}\n").unwrap();
        symlink(&shared_sessions, profile_home.join("sessions")).unwrap();
        fs::create_dir_all(&markers_dir).unwrap();
        fs::write(&stale_marker, "not-json").unwrap();

        let active = active_session_markers(&profile_home).unwrap();

        assert!(active.is_empty());
        assert!(rollout_dir.exists());
        assert!(rollout_path.exists());
        assert!(!stale_marker.exists());
    }

    #[test]
    fn create_launch_home_creates_unique_dirs_under_profile_launch_root() {
        let dir = tempdir().unwrap();
        let profile_home = dir.path().join("profiles").join("a");
        fs::create_dir_all(&profile_home).unwrap();

        let first = create_launch_home(&profile_home).unwrap();
        let second = create_launch_home(&profile_home).unwrap();

        assert_ne!(first, second);
        assert!(first.starts_with(profile_home.join(".launch")));
        assert!(second.starts_with(profile_home.join(".launch")));
        assert!(first.is_dir());
        assert!(second.is_dir());
    }

    #[test]
    fn usage_window_style_uses_run_rate_bands() {
        let now = Utc.timestamp_opt(9_000, 0).single().unwrap();
        let usage = ProfileUsageState::Available(ProfileUsageSnapshot {
            user_id: None,
            account_id: None,
            email: None,
            plan_type: Some("prolite".into()),
            primary: Some(UsageWindowSnapshot {
                used_percent: 51.0,
                reset_at: Some(reset_at_for_elapsed(
                    now,
                    super::UsageWindowKind::Primary,
                    0.5,
                )),
                limit_multiplier: 10.0,
            }),
            secondary: Some(UsageWindowSnapshot {
                used_percent: 100.0,
                reset_at: Some(reset_at_for_elapsed(
                    now,
                    super::UsageWindowKind::Secondary,
                    1.0,
                )),
                limit_multiplier: 10.0,
            }),
        });

        assert_eq!(
            super::usage_window_style_at(&usage, super::UsageWindowKind::Primary, now),
            LimitStyleKind::Success
        );
        assert_eq!(
            super::usage_window_style_at(&usage, super::UsageWindowKind::Secondary, now),
            LimitStyleKind::Success
        );
    }

    #[test]
    fn usage_window_style_is_neutral_without_run_rate() {
        let usage = ProfileUsageState::Available(ProfileUsageSnapshot {
            user_id: None,
            account_id: None,
            email: None,
            plan_type: Some("prolite".into()),
            primary: Some(UsageWindowSnapshot {
                used_percent: 42.0,
                reset_at: None,
                limit_multiplier: 10.0,
            }),
            secondary: Some(UsageWindowSnapshot {
                used_percent: 100.0,
                reset_at: None,
                limit_multiplier: 10.0,
            }),
        });

        assert_eq!(super::five_hour_limit_style(&usage), LimitStyleKind::Normal);
        assert_eq!(
            super::usage_window_style(&usage, super::UsageWindowKind::Secondary),
            LimitStyleKind::Normal
        );
    }

    #[test]
    fn list_rows_mark_active_duplicates_shared_accounts_and_invalid_auth() {
        let active = identity("sub-1", "user-1", "acct-1", Some("praveen@example.com"));
        let profiles = vec![
            saved_profile(
                "a",
                active.clone(),
                ProfileUsageState::Available(usage_snapshot(
                    "plus", "user-1", "acct-1", 42.0, 73.0,
                )),
            ),
            saved_profile(
                "b",
                identity("sub-1", "user-1", "acct-1", Some("alias@example.com")),
                ProfileUsageState::Unavailable,
            ),
            saved_profile(
                "c",
                identity("sub-2", "user-2", "acct-1", Some("team@example.com")),
                ProfileUsageState::ReauthNeeded,
            ),
            SavedProfile {
                name: "d".into(),
                auth_path: Path::new("/tmp/d/auth.json").into(),
                identity: None,
                invalid_auth: true,
                usage: ProfileUsageState::Unchecked,
            },
        ];

        let rows = build_profile_rows(&profiles, Some(&active));

        assert_eq!(
            row_status_text(&rows, "a"),
            "active same-user-as:b shared-account-with:c"
        );
        assert_eq!(
            row_status_text(&rows, "b"),
            "active same-user-as:a shared-account-with:c usage-unavailable"
        );
        assert_eq!(
            row_status_text(&rows, "c"),
            "shared-account-with:a,b reauth-needed"
        );
        assert_eq!(row_status_text(&rows, "d"), "invalid-auth");
        assert_eq!(row_field(&rows, "a", |row| row.plan.clone()), "Plus");
        assert_eq!(row_field(&rows, "a", |row| row.five_hour.clone()), "42%");
        assert_eq!(row_field(&rows, "a", |row| row.weekly.clone()), "73%");
        assert_eq!(
            row_limit_style(&rows, "a", |row| row.five_hour_style),
            LimitStyleKind::Success
        );
        assert_eq!(
            row_limit_style(&rows, "a", |row| row.weekly_style),
            LimitStyleKind::Success
        );
        assert_eq!(row_style(&rows, "a"), ProfileStyleKind::Error);
        assert_eq!(row_style(&rows, "c"), ProfileStyleKind::Normal);
    }

    #[test]
    fn list_rows_style_limits_by_run_rate_when_weekly_is_exhausted() {
        let active = identity("sub-1", "user-1", "acct-1", Some("praveen@example.com"));
        let profiles = vec![saved_profile(
            "a",
            active.clone(),
            ProfileUsageState::Available(usage_snapshot("plus", "user-1", "acct-1", 42.0, 100.0)),
        )];

        let rows = build_profile_rows(&profiles, Some(&active));

        assert_eq!(
            row_limit_style(&rows, "a", |row| row.five_hour_style),
            LimitStyleKind::Success
        );
        assert_eq!(
            row_limit_style(&rows, "a", |row| row.weekly_style),
            LimitStyleKind::Success
        );
    }

    #[test]
    fn list_rows_hide_weekly_reset_before_weekly_window_starts() {
        let active = identity("sub-1", "user-1", "acct-1", Some("praveen@example.com"));
        let profiles = vec![saved_profile(
            "a",
            active.clone(),
            ProfileUsageState::Available(usage_snapshot("plus", "user-1", "acct-1", 42.0, 0.0)),
        )];

        let rows = build_profile_rows(&profiles, Some(&active));

        assert_eq!(row_field(&rows, "a", |row| row.weekly_reset.clone()), "-");
        assert_eq!(
            row_field(&rows, "a", |row| row.weekly_compact.clone()),
            "  0%"
        );
    }

    #[test]
    fn colorize_limit_cell_uses_limit_style_when_row_has_no_error() {
        let _guard = COLOR_TEST_LOCK.lock().unwrap();
        colored::control::set_override(true);

        let row = super::ProfileRow {
            profile: "a".into(),
            label: "-".into(),
            total_dedupe_key: None,
            provider: "-".into(),
            user: "-".into(),
            account: "-".into(),
            plan: "-".into(),
            five_hour: "96%".into(),
            five_hour_reset: "-".into(),
            five_hour_compact: "96%".into(),
            five_hour_style: LimitStyleKind::Critical,
            five_hour_usage: None,
            weekly: "82%".into(),
            weekly_reset: "-".into(),
            weekly_compact: "82%".into(),
            weekly_style: LimitStyleKind::Caution,
            weekly_usage: None,
            status: Default::default(),
        };

        let critical = super::colorize_limit_cell("96%", 3, row.five_hour_style, &row);
        let caution = super::colorize_limit_cell("82%", 3, row.weekly_style, &row);

        assert!(critical.contains("\u{1b}[1;31m96%\u{1b}[0m"));
        assert!(caution.contains("82%"));
        assert!(caution == "82%" || caution.contains("\u{1b}["));

        colored::control::unset_override();
    }

    #[test]
    fn colorize_limit_cell_keeps_whole_row_error_precedence() {
        let _guard = COLOR_TEST_LOCK.lock().unwrap();
        colored::control::set_override(true);

        let row = super::ProfileRow {
            profile: "a".into(),
            label: "-".into(),
            total_dedupe_key: None,
            provider: "-".into(),
            user: "-".into(),
            account: "-".into(),
            plan: "-".into(),
            five_hour: "42%".into(),
            five_hour_reset: "-".into(),
            five_hour_compact: "42%".into(),
            five_hour_style: LimitStyleKind::Success,
            five_hour_usage: None,
            weekly: "73%".into(),
            weekly_reset: "-".into(),
            weekly_compact: "73%".into(),
            weekly_style: LimitStyleKind::Warning,
            weekly_usage: None,
            status: super::ProfileStatus {
                items: vec![super::ProfileStatusItem::SameUser(vec!["b".into()])],
            },
        };

        let rendered = super::colorize_limit_cell("42%", 3, row.five_hour_style, &row);

        assert!(
            rendered.contains("\u{1b}[1;31m42%\u{1b}[0m")
                || rendered.contains("\u{1b}[31;1m42%\u{1b}[0m")
        );

        colored::control::unset_override();
    }

    #[test]
    fn colorize_profile_cell_bolds_active_profile() {
        let _guard = COLOR_TEST_LOCK.lock().unwrap();
        colored::control::set_override(true);

        let row = super::ProfileRow {
            profile: "a".into(),
            label: "-".into(),
            total_dedupe_key: None,
            provider: "-".into(),
            user: "-".into(),
            account: "-".into(),
            plan: "-".into(),
            five_hour: "42%".into(),
            five_hour_reset: "-".into(),
            five_hour_compact: "42%".into(),
            five_hour_style: LimitStyleKind::Normal,
            five_hour_usage: None,
            weekly: "73%".into(),
            weekly_reset: "-".into(),
            weekly_compact: "73%".into(),
            weekly_style: LimitStyleKind::Normal,
            weekly_usage: None,
            status: super::ProfileStatus {
                items: vec![super::ProfileStatusItem::Active],
            },
        };

        let rendered = super::colorize_profile_cell("a", 1, &row);

        assert!(
            rendered.contains("\u{1b}[1;97ma\u{1b}[0m")
                || rendered.contains("\u{1b}[97;1ma\u{1b}[0m")
                || rendered.contains("\u{1b}[1;97;49ma\u{1b}[0m")
                || rendered.contains("\u{1b}[97;1;49ma\u{1b}[0m")
        );

        colored::control::unset_override();
    }

    #[test]
    fn usage_matches_identity_allows_personal_account_usage_shape() {
        let usage = ProfileUsageSnapshot {
            user_id: Some("user-1".into()),
            account_id: Some("user-1".into()),
            email: Some("praveen@example.com".into()),
            plan_type: Some("plus".into()),
            primary: None,
            secondary: None,
        };
        let identity = identity("sub-1", "user-1", "acct-1", Some("praveen@example.com"));

        assert!(super::usage_matches_identity(&usage, &identity));
    }

    #[tokio::test]
    async fn usage_loader_marks_unauthorized_usage_unavailable_without_refreshing() {
        let server = MockServer::start().await;
        let usage_path = "/backend-api/wham/usage";
        let loader =
            ProfileUsageLoader::with_urls(format!("{}{}", server.uri(), usage_path)).unwrap();
        let auth = read_auth(&ID_1, "old-access", "old-refresh");

        Mock::given(method("GET"))
            .and(path(usage_path))
            .and(header("authorization", "Bearer old-access"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "error": { "code": "token_invalid" }
            })))
            .mount(&server)
            .await;

        let result = loader
            .fetch_profile_usage(
                &auth,
                Some(&identity(
                    "sub-1",
                    "user-1",
                    "acct-1",
                    Some("old@example.com"),
                )),
            )
            .await
            .unwrap();

        let UsageFetchResult::Unavailable { .. } = result else {
            panic!("expected unavailable usage");
        };

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method.as_str(), "GET");
        assert_eq!(requests[0].url.path(), usage_path);
    }

    #[tokio::test]
    async fn usage_loader_marks_identity_mismatches_as_reauth_needed() {
        let server = MockServer::start().await;
        let usage_path = "/backend-api/wham/usage";
        let loader =
            ProfileUsageLoader::with_urls(format!("{}{}", server.uri(), usage_path)).unwrap();
        let auth = read_auth(&ID_1, "valid-access", "old-refresh");

        Mock::given(method("GET"))
            .and(path(usage_path))
            .and(header("authorization", "Bearer valid-access"))
            .respond_with(ResponseTemplate::new(200).set_body_json(usage_response(
                "other@example.com",
                "user-2",
                "acct-2",
                "plus",
                10.0,
                59.0,
            )))
            .mount(&server)
            .await;

        let result = loader
            .fetch_profile_usage(
                &auth,
                Some(&identity(
                    "sub-1",
                    "user-1",
                    "acct-1",
                    Some("old@example.com"),
                )),
            )
            .await
            .unwrap();

        assert!(matches!(result, UsageFetchResult::ReauthNeeded));
    }

    #[test]
    fn sync_login_codex_home_links_non_local_entries() {
        let dir = tempdir().unwrap();
        let global_codex = dir.path().join(".codex");
        let login_home = dir.path().join("login");

        fs::create_dir_all(global_codex.join("profiles")).unwrap();
        fs::create_dir_all(global_codex.join("sessions")).unwrap();
        fs::create_dir_all(global_codex.join("skills")).unwrap();
        fs::write(global_codex.join("config.toml"), "model = \"gpt-5.4\"").unwrap();
        fs::write(global_codex.join("AGENTS.md"), "shared").unwrap();
        fs::write(global_codex.join("auth.json"), "shared-auth").unwrap();

        sync_login_codex_home(&login_home, &global_codex).unwrap();

        assert_eq!(
            fs::read_link(login_home.join("config.toml")).unwrap(),
            global_codex.join("config.toml")
        );
        assert_eq!(
            fs::read_link(login_home.join("skills")).unwrap(),
            global_codex.join("skills")
        );
        assert_eq!(
            fs::read_link(login_home.join("sessions")).unwrap(),
            global_codex.join("sessions")
        );
        assert_eq!(
            fs::read_link(login_home.join("AGENTS.md")).unwrap(),
            global_codex.join("AGENTS.md")
        );
        assert!(!login_home.join("auth.json").exists());
        assert!(!login_home.join("profiles").exists());
    }

    #[test]
    fn prepare_config_group_home_seeds_only_config_entries() {
        let dir = tempdir().unwrap();
        let global_codex = dir.path().join(".codex");
        let config_groups = global_codex.join("config-groups");

        fs::create_dir_all(global_codex.join("profiles")).unwrap();
        fs::create_dir_all(global_codex.join("sessions")).unwrap();
        fs::write(global_codex.join("config.toml"), "model = \"gpt-5.4\"").unwrap();
        fs::write(global_codex.join("history.jsonl"), "history").unwrap();
        fs::write(global_codex.join("AGENTS.md"), "shared").unwrap();

        let config_home = prepare_config_group_home(&global_codex, "work").unwrap();

        assert_eq!(
            fs::read_to_string(config_home.join("config.toml")).unwrap(),
            "model = \"gpt-5.4\""
        );
        assert_eq!(
            fs::read_to_string(config_home.join("history.jsonl")).unwrap(),
            "history"
        );
        assert!(config_groups.join("work").exists());
        assert!(!config_home.join("sessions").exists());
        assert!(!config_home.join("AGENTS.md").exists());
        assert!(!fs::symlink_metadata(config_home.join("config.toml"))
            .unwrap()
            .file_type()
            .is_symlink());
    }

    #[test]
    fn prepare_resume_group_home_seeds_only_resume_entries() {
        let dir = tempdir().unwrap();
        let global_codex = dir.path().join(".codex");
        let resume_groups = global_codex.join("resume-groups");

        fs::create_dir_all(global_codex.join("profiles")).unwrap();
        fs::create_dir_all(global_codex.join("sessions")).unwrap();
        fs::write(global_codex.join("session_index.jsonl"), "index").unwrap();
        fs::write(global_codex.join("config.toml"), "model = \"gpt-5.4\"").unwrap();

        let resume_home = prepare_resume_group_home(&global_codex, "shared-work").unwrap();

        assert_eq!(
            fs::read_to_string(resume_home.join("session_index.jsonl")).unwrap(),
            "index"
        );
        assert_eq!(
            fs::read_dir(resume_home.join("sessions")).unwrap().count(),
            0
        );
        assert!(resume_groups.join("shared-work").exists());
        assert!(!resume_home.join("config.toml").exists());
    }

    #[test]
    fn sync_launch_codex_home_copies_profile_auth_and_links_selected_group_entries() {
        let dir = tempdir().unwrap();
        let global_codex = dir.path().join(".codex");
        let launch_home = dir.path().join("launch");
        let profile_auth = global_codex.join("profiles").join("a").join("auth.json");
        let config_home = dir.path().join("config-group");
        let resume_home = dir.path().join("resume-group");

        fs::create_dir_all(global_codex.join("profiles").join("a")).unwrap();
        fs::create_dir_all(global_codex.join("skills")).unwrap();
        fs::create_dir_all(&config_home).unwrap();
        fs::create_dir_all(resume_home.join("sessions")).unwrap();
        fs::write(global_codex.join("AGENTS.md"), "shared").unwrap();
        fs::write(global_codex.join("skills").join("skill.txt"), "skill").unwrap();
        fs::write(config_home.join("config.toml"), "model = \"gpt-5.4\"").unwrap();
        fs::write(
            config_home.join(".codex-global-state.json"),
            "{\"default-service-tier\":\"fast\"}",
        )
        .unwrap();
        fs::write(resume_home.join("session_index.jsonl"), "index").unwrap();
        fs::write(resume_home.join("state_5.sqlite"), "state").unwrap();
        fs::write(&profile_auth, auth_json(&ID_1)).unwrap();
        let launch_auth_mode = LaunchAuthMode::ProfileCopy {
            profile_auth: profile_auth.clone(),
            launch_auth: read_auth_snapshot(&profile_auth).unwrap(),
        };

        sync_launch_codex_home(
            &launch_home,
            &global_codex,
            &launch_auth_mode,
            &config_home,
            &resume_home,
        )
        .unwrap();

        assert_eq!(
            fs::read_to_string(launch_home.join("auth.json")).unwrap(),
            auth_json(&ID_1)
        );
        assert_eq!(
            fs::read_link(launch_home.join("config.toml")).unwrap(),
            config_home.join("config.toml")
        );
        assert_eq!(
            fs::read_link(launch_home.join(".codex-global-state.json")).unwrap(),
            config_home.join(".codex-global-state.json")
        );
        assert_eq!(
            fs::read_link(launch_home.join("session_index.jsonl")).unwrap(),
            resume_home.join("session_index.jsonl")
        );
        assert_eq!(
            fs::read_link(launch_home.join("state_5.sqlite")).unwrap(),
            resume_home.join("state_5.sqlite")
        );
        assert_eq!(
            fs::read_link(launch_home.join("skills")).unwrap(),
            global_codex.join("skills")
        );
        assert!(!launch_home.join("profiles").exists());
    }

    #[test]
    fn sync_launch_codex_home_links_global_auth_for_active_launches() {
        let dir = tempdir().unwrap();
        let global_codex = dir.path().join(".codex");
        let launch_home = dir.path().join("launch");
        let global_auth = global_codex.join("auth.json");
        let config_home = dir.path().join("config-group");
        let resume_home = dir.path().join("resume-group");

        fs::create_dir_all(&global_codex).unwrap();
        fs::create_dir_all(global_codex.join("skills")).unwrap();
        fs::create_dir_all(&config_home).unwrap();
        fs::create_dir_all(resume_home.join("sessions")).unwrap();
        fs::write(&global_auth, auth_json(&ID_1)).unwrap();
        fs::write(global_codex.join("AGENTS.md"), "shared").unwrap();
        fs::write(global_codex.join("skills").join("skill.txt"), "skill").unwrap();
        fs::write(config_home.join("config.toml"), "model = \"gpt-5.4\"").unwrap();
        fs::write(resume_home.join("session_index.jsonl"), "index").unwrap();
        let launch_auth_mode = LaunchAuthMode::GlobalShared {
            global_auth: global_auth.clone(),
        };

        sync_launch_codex_home(
            &launch_home,
            &global_codex,
            &launch_auth_mode,
            &config_home,
            &resume_home,
        )
        .unwrap();

        assert_eq!(
            fs::read_link(launch_home.join("auth.json")).unwrap(),
            global_auth
        );
        assert_eq!(
            fs::read_link(launch_home.join("config.toml")).unwrap(),
            config_home.join("config.toml")
        );
        assert_eq!(
            fs::read_link(launch_home.join("session_index.jsonl")).unwrap(),
            resume_home.join("session_index.jsonl")
        );
    }

    #[test]
    fn resolve_launch_auth_mode_reuses_global_auth_for_same_user() {
        let dir = tempdir().unwrap();
        let profile_auth = dir.path().join("profile-auth.json");
        let global_auth = dir.path().join("global-auth.json");
        fs::write(
            &profile_auth,
            auth_json_with_tokens(&ID_1, "profile-access", "profile-refresh"),
        )
        .unwrap();
        fs::write(
            &global_auth,
            auth_json_with_tokens(&ID_1_ALIAS, "global-access", "global-refresh"),
        )
        .unwrap();

        let launch_auth_mode = resolve_launch_auth_mode(&profile_auth, &global_auth).unwrap();

        assert!(matches!(
            launch_auth_mode,
            LaunchAuthMode::GlobalShared { global_auth: ref path } if *path == global_auth
        ));
    }

    #[test]
    fn resolve_launch_auth_mode_falls_back_to_profile_copy_when_global_auth_missing() {
        let dir = tempdir().unwrap();
        let profile_auth = dir.path().join("profile-auth.json");
        let global_auth = dir.path().join("global-auth.json");
        let profile_raw = auth_json_with_tokens(&ID_1, "profile-access", "profile-refresh");
        fs::write(&profile_auth, &profile_raw).unwrap();

        let launch_auth_mode = resolve_launch_auth_mode(&profile_auth, &global_auth).unwrap();

        match launch_auth_mode {
            LaunchAuthMode::ProfileCopy {
                profile_auth: path,
                launch_auth,
            } => {
                assert_eq!(path, profile_auth);
                assert_eq!(launch_auth.raw, profile_raw.into_bytes());
            }
            LaunchAuthMode::GlobalShared { .. } => panic!("expected profile copy"),
        }
    }

    #[test]
    fn resolve_launch_auth_mode_falls_back_to_profile_copy_for_different_users() {
        let dir = tempdir().unwrap();
        let profile_auth = dir.path().join("profile-auth.json");
        let global_auth = dir.path().join("global-auth.json");
        fs::write(
            &profile_auth,
            auth_json_with_tokens(&ID_1, "profile-access", "profile-refresh"),
        )
        .unwrap();
        fs::write(
            &global_auth,
            auth_json_with_tokens(&ID_2, "global-access", "global-refresh"),
        )
        .unwrap();

        let launch_auth_mode = resolve_launch_auth_mode(&profile_auth, &global_auth).unwrap();

        assert!(matches!(
            launch_auth_mode,
            LaunchAuthMode::ProfileCopy { profile_auth: ref path, .. } if *path == profile_auth
        ));
    }

    #[test]
    fn resolve_launch_auth_mode_falls_back_to_profile_copy_when_global_auth_is_invalid() {
        let dir = tempdir().unwrap();
        let profile_auth = dir.path().join("profile-auth.json");
        let global_auth = dir.path().join("global-auth.json");
        fs::write(
            &profile_auth,
            auth_json_with_tokens(&ID_1, "profile-access", "profile-refresh"),
        )
        .unwrap();
        fs::write(&global_auth, "{invalid").unwrap();

        let launch_auth_mode = resolve_launch_auth_mode(&profile_auth, &global_auth).unwrap();

        assert!(matches!(
            launch_auth_mode,
            LaunchAuthMode::ProfileCopy { profile_auth: ref path, .. } if *path == profile_auth
        ));
    }

    #[test]
    fn promote_launch_auth_if_unchanged_updates_profile_auth() {
        let dir = tempdir().unwrap();
        let profile_auth = dir.path().join("profile-auth.json");
        let launch_auth_path = dir.path().join("launch-auth.json");
        let original_auth = auth_json(&ID_1);
        let refreshed_auth = auth_json_with_tokens(
            &TestIdentity {
                email: Some("new@example.com"),
                ..ID_1
            },
            "new-access",
            "new-refresh",
        );

        fs::write(&profile_auth, &original_auth).unwrap();
        fs::write(&launch_auth_path, &refreshed_auth).unwrap();
        let launch_auth = read_auth_snapshot(&profile_auth).unwrap();

        promote_launch_auth_if_unchanged(&profile_auth, &launch_auth, &launch_auth_path).unwrap();

        assert_eq!(fs::read_to_string(&profile_auth).unwrap(), refreshed_auth);
    }

    #[test]
    fn promote_launch_auth_if_unchanged_skips_when_profile_changed() {
        let dir = tempdir().unwrap();
        let profile_auth = dir.path().join("profile-auth.json");
        let launch_auth_path = dir.path().join("launch-auth.json");
        let original_auth = auth_json(&ID_1);
        let competing_auth = auth_json_with_tokens(
            &TestIdentity {
                email: Some("other@example.com"),
                ..ID_1
            },
            "other-access",
            "other-refresh",
        );
        let refreshed_auth = auth_json_with_tokens(
            &TestIdentity {
                email: Some("new@example.com"),
                ..ID_1
            },
            "new-access",
            "new-refresh",
        );

        fs::write(&profile_auth, &original_auth).unwrap();
        let launch_auth = read_auth_snapshot(&profile_auth).unwrap();
        fs::write(&profile_auth, &competing_auth).unwrap();
        fs::write(&launch_auth_path, &refreshed_auth).unwrap();

        promote_launch_auth_if_unchanged(&profile_auth, &launch_auth, &launch_auth_path).unwrap();

        assert_eq!(fs::read_to_string(&profile_auth).unwrap(), competing_auth);
    }

    #[test]
    fn promote_launch_auth_if_unchanged_skips_account_mismatch() {
        let dir = tempdir().unwrap();
        let profile_auth = dir.path().join("profile-auth.json");
        let launch_auth_path = dir.path().join("launch-auth.json");
        let original_auth = auth_json(&ID_1);
        let switched_account_auth = auth_json_with_tokens(
            &TestIdentity {
                account_id: "acct-2",
                email: Some("new@example.com"),
                ..ID_1
            },
            "new-access",
            "new-refresh",
        );

        fs::write(&profile_auth, &original_auth).unwrap();
        fs::write(&launch_auth_path, &switched_account_auth).unwrap();
        let launch_auth = read_auth_snapshot(&profile_auth).unwrap();

        promote_launch_auth_if_unchanged(&profile_auth, &launch_auth, &launch_auth_path).unwrap();

        assert_eq!(fs::read_to_string(&profile_auth).unwrap(), original_auth);
    }

    #[test]
    fn write_auth_raw_if_unchanged_skips_when_profile_changed() {
        let dir = tempdir().unwrap();
        let auth_path = dir.path().join("auth.json");

        fs::write(&auth_path, "current").unwrap();

        let wrote = write_auth_raw_if_unchanged(&auth_path, b"expected", b"replacement").unwrap();

        assert!(!wrote);
        assert_eq!(fs::read_to_string(&auth_path).unwrap(), "current");
    }

    #[tokio::test]
    async fn refresh_profile_auth_updates_tokens() {
        let server = MockServer::start().await;
        let refresher =
            ProfileAuthRefresher::with_url(format!("{}/oauth/token", server.uri())).unwrap();
        let auth = read_auth(&ID_1, "old-access", "old-refresh");

        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id_token": jwt(&TestIdentity { email: Some("new@example.com"), ..ID_1 }),
                "access_token": "new-access",
                "refresh_token": "new-refresh",
            })))
            .mount(&server)
            .await;

        let refreshed = refresher
            .refresh_profile_auth(
                &auth,
                Some(&identity(
                    "sub-1",
                    "user-1",
                    "acct-1",
                    Some("old@example.com"),
                )),
            )
            .await
            .unwrap();

        assert_eq!(
            refreshed
                .tokens
                .as_ref()
                .and_then(|tokens| tokens.access_token.as_deref()),
            Some("new-access")
        );
        assert_eq!(
            refreshed
                .tokens
                .as_ref()
                .and_then(|tokens| tokens.refresh_token.as_deref()),
            Some("new-refresh")
        );
        assert!(refreshed.last_refresh.is_some());
    }

    #[tokio::test]
    async fn refresh_profile_auth_rejects_identity_mismatch() {
        let server = MockServer::start().await;
        let refresher =
            ProfileAuthRefresher::with_url(format!("{}/oauth/token", server.uri())).unwrap();
        let auth = read_auth(&ID_1, "old-access", "old-refresh");

        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id_token": jwt(&TestIdentity { subject: "sub-2", user_id: "user-2", account_id: "acct-2", email: Some("other@example.com"), name: None, auth_provider: Some("google") }),
                "access_token": "new-access",
                "refresh_token": "new-refresh",
            })))
            .mount(&server)
            .await;

        let err = refresher
            .refresh_profile_auth(
                &auth,
                Some(&identity(
                    "sub-1",
                    "user-1",
                    "acct-1",
                    Some("old@example.com"),
                )),
            )
            .await
            .unwrap_err();

        assert!(err
            .to_string()
            .contains("Refreshed auth does not match saved profile identity"));
    }

    #[test]
    fn cmd_parses_launch_with_explicit_profile_and_separator() {
        let cmd = parse_raw_args(&[
            OsString::from("launch"),
            OsString::from("a"),
            OsString::from("--"),
            OsString::from("--model"),
            OsString::from("gpt-5.4"),
        ])
        .unwrap();

        let subcommand = cmd.subcommand;
        let CodexCmd::Launch {
            profile_or_arg,
            resume_group,
            config_group,
            other,
            args,
        } = subcommand
        else {
            panic!("expected launch subcommand");
        };

        assert_eq!(profile_or_arg, Some(OsString::from("a")));
        assert_eq!(resume_group, None);
        assert_eq!(config_group, None);
        assert!(!other);
        assert_eq!(
            args,
            vec![OsString::from("--model"), OsString::from("gpt-5.4")]
        );
    }

    #[test]
    fn cmd_parses_launch_with_forced_auto_selection() {
        let cmd = parse_raw_args(&[
            OsString::from("launch"),
            OsString::from("-r"),
            OsString::from("shared-work"),
            OsString::from("-c"),
            OsString::from("cfg-a"),
            OsString::from("--"),
            OsString::from("a"),
        ])
        .unwrap();

        let subcommand = cmd.subcommand;
        let CodexCmd::Launch {
            profile_or_arg,
            resume_group,
            config_group,
            other,
            args,
        } = subcommand
        else {
            panic!("expected launch subcommand");
        };

        assert_eq!(profile_or_arg, None);
        assert_eq!(resume_group.as_deref(), Some("shared-work"));
        assert_eq!(config_group.as_deref(), Some("cfg-a"));
        assert!(!other);
        assert_eq!(args, vec![OsString::from("a")]);
    }

    #[test]
    fn cmd_parses_launch_with_short_group_flags_and_other_without_separator() {
        let cmd = parse_raw_args(&[
            OsString::from("launch"),
            OsString::from("-c"),
            OsString::from("cfg-a"),
            OsString::from("-r"),
            OsString::from("shared-work"),
            OsString::from("--other"),
            OsString::from("s"),
        ])
        .unwrap();

        let subcommand = cmd.subcommand;
        let CodexCmd::Launch {
            profile_or_arg,
            resume_group,
            config_group,
            other,
            args,
        } = subcommand
        else {
            panic!("expected launch subcommand");
        };

        assert_eq!(profile_or_arg, Some(OsString::from("s")));
        assert_eq!(resume_group.as_deref(), Some("shared-work"));
        assert_eq!(config_group.as_deref(), Some("cfg-a"));
        assert!(other);
        assert!(args.is_empty());
    }

    #[test]
    fn cmd_parses_usage_subcommand() {
        let cmd = parse_raw_args(&[OsString::from("usage")]).unwrap();

        assert!(matches!(cmd.subcommand, CodexCmd::Usage));
    }

    #[test]
    fn resolve_launch_target_treats_matching_profile_as_explicit() {
        let profiles = vec![available_saved_profile("a", 10.0, 20.0)];
        let profile_or_arg = OsString::from("a");
        let args = vec![OsString::from("--model"), OsString::from("gpt-5.4")];

        let target = resolve_launch_target(
            Some(&profile_or_arg),
            Some("shared-work"),
            Some("cfg-a"),
            false,
            &args,
            &profiles,
        )
        .unwrap();

        assert_eq!(
            target,
            LaunchTarget::Explicit {
                profile: "a".into(),
                resume_group: Some("shared-work".into()),
                config_group: Some("cfg-a".into()),
                args,
            }
        );
    }

    #[test]
    fn resolve_launch_target_treats_non_matching_first_value_as_codex_arg() {
        let profiles = vec![available_saved_profile("a", 10.0, 20.0)];
        let profile_or_arg = OsString::from("--model");
        let args = vec![OsString::from("gpt-5.4")];

        let target = resolve_launch_target(
            Some(&profile_or_arg),
            None,
            Some("cfg-a"),
            true,
            &args,
            &profiles,
        )
        .unwrap();

        assert_eq!(
            target,
            LaunchTarget::Auto {
                resume_group: None,
                config_group: Some("cfg-a".into()),
                other: true,
                args: vec![OsString::from("--model"), OsString::from("gpt-5.4")],
            }
        );
    }

    #[test]
    fn resolve_launch_target_rejects_other_with_explicit_profile() {
        let profiles = vec![available_saved_profile("a", 10.0, 20.0)];
        let profile_or_arg = OsString::from("a");
        let args = vec![OsString::from("--model"), OsString::from("gpt-5.4")];

        let err = resolve_launch_target(Some(&profile_or_arg), None, None, true, &args, &profiles)
            .unwrap_err();

        assert!(err
            .to_string()
            .contains("--other cannot be combined with an explicit profile"));
    }

    #[test]
    fn validate_group_name_rejects_invalid_values() {
        assert!(validate_group_name("", "config").is_err());
        assert!(validate_group_name(".", "config").is_err());
        assert!(validate_group_name("a/b", "resume").is_err());
    }

    #[test]
    fn select_auto_launch_profile_prefers_lowest_weighted_score() {
        let profiles = vec![
            available_saved_profile("a", 20.0, 30.0),
            available_saved_profile("b", 15.0, 20.0),
            available_saved_profile("c", 10.0, 40.0),
        ];

        let selected = select_auto_launch_profile(&profiles).unwrap();

        assert_eq!(selected.name, "b");
    }

    #[test]
    fn select_auto_launch_profile_prefers_most_under_pace_weekly_account() {
        let now = Utc::now();
        let profiles = vec![
            available_saved_profile_with_resets(
                "a",
                20.0,
                now.timestamp() + 3600,
                10.0,
                reset_at_for_elapsed(now, super::UsageWindowKind::Secondary, 0.05),
            ),
            available_saved_profile_with_resets(
                "b",
                20.0,
                now.timestamp() + 3600,
                40.0,
                reset_at_for_elapsed(now, super::UsageWindowKind::Secondary, 0.8),
            ),
        ];

        let selected = select_auto_launch_profile(&profiles).unwrap();

        assert_eq!(selected.name, "b");
    }

    #[test]
    fn select_auto_launch_profile_uses_three_x_weekly_pace_weight() {
        let now = Utc::now();
        let weekly_reset = reset_at_for_elapsed(now, super::UsageWindowKind::Secondary, 0.5);
        let profiles = vec![
            available_saved_profile_with_resets(
                "a",
                50.0,
                now.timestamp() + 3600,
                20.0,
                weekly_reset,
            ),
            available_saved_profile_with_resets(
                "b",
                10.0,
                now.timestamp() + 3600,
                35.0,
                weekly_reset,
            ),
        ];

        let selected = select_auto_launch_profile(&profiles).unwrap();

        assert_eq!(selected.name, "a");
    }

    #[test]
    fn select_auto_launch_profile_prefers_earlier_five_hour_reset_when_pace_ties() {
        let now = Utc::now();
        let weekly_reset = reset_at_for_elapsed(now, super::UsageWindowKind::Secondary, 0.5);
        let profiles = vec![
            available_saved_profile_with_resets(
                "later-reset",
                10.0,
                reset_at_for_elapsed(now, super::UsageWindowKind::Primary, 0.1),
                50.0,
                weekly_reset,
            ),
            available_saved_profile_with_resets(
                "earlier-reset",
                70.0,
                reset_at_for_elapsed(now, super::UsageWindowKind::Primary, 0.7),
                50.0,
                weekly_reset,
            ),
        ];

        let selected = select_auto_launch_profile(&profiles).unwrap();

        assert_eq!(selected.name, "earlier-reset");
    }

    #[test]
    fn select_auto_launch_profile_prefers_better_five_hour_pace_over_lower_raw_usage() {
        let now = Utc::now();
        let weekly_reset = reset_at_for_elapsed(now, super::UsageWindowKind::Secondary, 0.5);
        let profiles = vec![
            available_saved_profile_with_resets(
                "on-pace",
                60.0,
                reset_at_for_elapsed(now, super::UsageWindowKind::Primary, 0.8),
                50.0,
                weekly_reset,
            ),
            available_saved_profile_with_resets(
                "behind-pace",
                30.0,
                reset_at_for_elapsed(now, super::UsageWindowKind::Primary, 0.2),
                50.0,
                weekly_reset,
            ),
        ];

        let selected = select_auto_launch_profile(&profiles).unwrap();

        assert_eq!(selected.name, "on-pace");
    }

    #[test]
    fn select_auto_launch_profile_skips_hot_candidate_when_cool_one_exists() {
        let profiles = vec![
            available_saved_profile("a", 85.0, 5.0),
            available_saved_profile("b", 30.0, 25.0),
        ];

        let selected = select_auto_launch_profile(&profiles).unwrap();

        assert_eq!(selected.name, "b");
    }

    #[test]
    fn select_auto_launch_profile_falls_back_to_best_hot_candidate() {
        let profiles = vec![
            available_saved_profile("a", 90.0, 10.0),
            available_saved_profile("b", 85.0, 20.0),
            available_saved_profile("c", 80.0, 30.0),
        ];

        let selected = select_auto_launch_profile(&profiles).unwrap();

        assert_eq!(selected.name, "a");
    }

    #[test]
    fn select_auto_launch_profile_ignores_invalid_and_incomplete_usage() {
        let mut invalid = available_saved_profile("invalid", 10.0, 10.0);
        invalid.invalid_auth = true;

        let incomplete = profile_with_snapshot(
            "incomplete",
            ProfileUsageSnapshot {
                user_id: Some("user-incomplete".into()),
                account_id: Some("acct-incomplete".into()),
                email: Some("incomplete@example.com".into()),
                plan_type: Some("plus".into()),
                primary: Some(UsageWindowSnapshot {
                    used_percent: 5.0,
                    reset_at: Some(Local::now().timestamp() + 3600),
                    limit_multiplier: 1.0,
                }),
                secondary: None,
            },
        );

        let profiles = vec![
            invalid,
            profile_with_state("reauth", ProfileUsageState::ReauthNeeded),
            profile_with_state("unavailable", ProfileUsageState::Unavailable),
            incomplete,
            available_saved_profile("winner", 12.0, 18.0),
        ];

        let selected = select_auto_launch_profile(&profiles).unwrap();

        assert_eq!(selected.name, "winner");
    }

    #[test]
    fn select_auto_launch_profile_keeps_candidate_when_primary_reset_is_missing() {
        let now = Utc::now();
        let weekly_reset = reset_at_for_elapsed(now, super::UsageWindowKind::Secondary, 0.2);
        let profiles = vec![
            profile_with_snapshot(
                "missing-reset",
                ProfileUsageSnapshot {
                    user_id: Some("user-missing-reset".into()),
                    account_id: Some("acct-missing-reset".into()),
                    email: Some("missing-reset@example.com".into()),
                    plan_type: Some("plus".into()),
                    primary: Some(UsageWindowSnapshot {
                        used_percent: 10.0,
                        reset_at: None,
                        limit_multiplier: 1.0,
                    }),
                    secondary: Some(UsageWindowSnapshot {
                        used_percent: 20.0,
                        reset_at: Some(weekly_reset),
                        limit_multiplier: 1.0,
                    }),
                },
            ),
            available_saved_profile_with_resets(
                "fully-timed",
                40.0,
                reset_at_for_elapsed(now, super::UsageWindowKind::Primary, 0.2),
                20.0,
                weekly_reset,
            ),
        ];

        let selected = select_auto_launch_profile(&profiles).unwrap();

        assert_eq!(selected.name, "missing-reset");
    }

    #[test]
    fn select_auto_launch_profile_breaks_exact_ties_by_profile_name() {
        let now = Utc::now();
        let profiles = vec![
            available_saved_profile_with_resets(
                "b",
                40.0,
                reset_at_for_elapsed(now, super::UsageWindowKind::Primary, 0.4),
                30.0,
                reset_at_for_elapsed(now, super::UsageWindowKind::Secondary, 0.3),
            ),
            available_saved_profile_with_resets(
                "a",
                40.0,
                reset_at_for_elapsed(now, super::UsageWindowKind::Primary, 0.4),
                30.0,
                reset_at_for_elapsed(now, super::UsageWindowKind::Secondary, 0.3),
            ),
        ];

        let selected = select_auto_launch_profile(&profiles).unwrap();

        assert_eq!(selected.name, "a");
    }

    #[test]
    fn select_auto_launch_profile_errors_when_no_eligible_profile_exists() {
        let profiles = vec![
            profile_with_state("reauth", ProfileUsageState::ReauthNeeded),
            profile_with_state("unavailable", ProfileUsageState::Unavailable),
        ];

        let err = select_auto_launch_profile(&profiles).unwrap_err();

        assert!(err
            .to_string()
            .contains("No profiles with usable usage data found"));
    }

    #[test]
    fn select_auto_launch_profile_skips_profile_with_100_percent_weekly_usage() {
        let profiles = vec![
            available_saved_profile("maxed-weekly", 30.0, 100.0),
            available_saved_profile("usable", 20.0, 40.0),
        ];

        let selected = select_auto_launch_profile(&profiles).unwrap();

        assert_eq!(selected.name, "usable");
    }

    #[test]
    fn select_auto_launch_profile_skips_profile_with_100_percent_five_hour_usage() {
        let profiles = vec![
            available_saved_profile("maxed-five-hour", 100.0, 30.0),
            available_saved_profile("usable", 20.0, 40.0),
        ];

        let selected = select_auto_launch_profile(&profiles).unwrap();

        assert_eq!(selected.name, "usable");
    }

    #[test]
    fn select_auto_launch_profile_errors_when_all_profiles_at_100_percent() {
        let profiles = vec![
            available_saved_profile("a", 100.0, 50.0),
            available_saved_profile("b", 50.0, 100.0),
            available_saved_profile("c", 100.0, 100.0),
        ];

        let err = select_auto_launch_profile(&profiles).unwrap_err();

        assert!(err
            .to_string()
            .contains("No profiles with usable usage data found"));
    }

    #[test]
    fn select_auto_launch_profile_except_skips_excluded_profile() {
        let profiles = vec![
            available_saved_profile("a", 10.0, 20.0),
            available_saved_profile("b", 15.0, 30.0),
        ];

        let selected = select_auto_launch_profile_except(&profiles, Some("a")).unwrap();

        assert_eq!(selected.name, "b");
    }

    #[test]
    fn select_auto_launch_profile_except_errors_when_only_excluded_profile_is_usable() {
        let profiles = vec![
            available_saved_profile("a", 10.0, 20.0),
            available_saved_profile("b", 100.0, 100.0),
        ];

        let err = select_auto_launch_profile_except(&profiles, Some("a")).unwrap_err();

        assert!(err
            .to_string()
            .contains("No other profiles with usable usage data found"));
    }

    #[test]
    fn format_launch_banner_includes_profile_usage_and_reset_times() {
        let profile = available_saved_profile("a", 42.0, 73.0);
        let groups = resolve_launch_groups("a", None, None).unwrap();
        let details = launch_banner_details(&profile);
        let banner = format_launch_banner("a", &groups, &details);

        assert!(banner.starts_with("a@example.com\n"));
        assert!(banner.contains("launching"));
        assert!(banner.contains("profile"));
        assert!(banner.contains("a"));
        assert!(banner.contains("config"));
        assert!(banner.contains("resume"));
        assert!(banner.contains("shared"));
        assert!(banner.contains("\n5H:"));
        assert!(banner.contains("Weekly:"));
        assert!(banner.contains("  | "));
        assert!(banner.contains("73%"));
        assert!(!banner.contains("\n 42%    "));
        assert!(!banner.contains("reset"));
    }

    #[test]
    fn launch_banner_details_use_compact_weekly_reset_format() {
        let weekly_reset = (Local::now() + chrono::Duration::days(1)).timestamp();
        let profile = profile_with_snapshot(
            "a",
            ProfileUsageSnapshot {
                user_id: Some("user-a".into()),
                account_id: Some("acct-a".into()),
                email: Some("a@example.com".into()),
                plan_type: Some("plus".into()),
                primary: Some(UsageWindowSnapshot {
                    used_percent: 42.0,
                    reset_at: Some(Local::now().timestamp() + 3600),
                    limit_multiplier: 1.0,
                }),
                secondary: Some(UsageWindowSnapshot {
                    used_percent: 3.0,
                    reset_at: Some(weekly_reset),
                    limit_multiplier: 1.0,
                }),
            },
        );

        let groups = resolve_launch_groups("a", None, None).unwrap();
        let details = launch_banner_details(&profile);
        let banner = format_launch_banner("a", &groups, &details);
        let reset = Local.timestamp_opt(weekly_reset, 0).single().unwrap();
        let expected_reset = reset.format("%a %-I:%M %p").to_string();

        assert!(banner.contains(&expected_reset));
        assert!(!banner.contains(" on "));
        assert!(!banner.contains(&reset.format("%a %-d %b").to_string()));
    }

    #[test]
    fn resolve_launch_groups_uses_profile_and_shared_defaults() {
        let groups = resolve_launch_groups("work", None, None).unwrap();

        assert_eq!(
            groups,
            LaunchGroups {
                config: "work".into(),
                resume: "shared".into(),
            }
        );
    }

    #[test]
    fn resolve_launch_groups_preserves_explicit_group_names() {
        let groups = resolve_launch_groups("work", Some("resume-work"), Some("cfg-work")).unwrap();

        assert_eq!(
            groups,
            LaunchGroups {
                config: "cfg-work".into(),
                resume: "resume-work".into(),
            }
        );
    }

    fn saved_profile(name: &str, identity: AuthIdentity, usage: ProfileUsageState) -> SavedProfile {
        SavedProfile {
            name: name.into(),
            auth_path: Path::new("/tmp").join(name).join("auth.json"),
            identity: Some(identity),
            invalid_auth: false,
            usage,
        }
    }

    fn available_saved_profile(
        name: &str,
        primary_used_percent: f64,
        secondary_used_percent: f64,
    ) -> SavedProfile {
        let subject = format!("sub-{name}");
        let user_id = format!("user-{name}");
        let account_id = format!("acct-{name}");
        let email = format!("{name}@example.com");

        saved_profile(
            name,
            identity(&subject, &user_id, &account_id, Some(&email)),
            ProfileUsageState::Available(usage_snapshot(
                "plus",
                &user_id,
                &account_id,
                primary_used_percent,
                secondary_used_percent,
            )),
        )
    }

    fn profile_with_state(name: &str, usage: ProfileUsageState) -> SavedProfile {
        let subject = format!("sub-{name}");
        let user_id = format!("user-{name}");
        let account_id = format!("acct-{name}");
        let email = format!("{name}@example.com");

        saved_profile(
            name,
            identity(&subject, &user_id, &account_id, Some(&email)),
            usage,
        )
    }

    fn profile_with_snapshot(name: &str, usage: ProfileUsageSnapshot) -> SavedProfile {
        let subject = format!("sub-{name}");
        let user_id = format!("user-{name}");
        let account_id = format!("acct-{name}");
        let email = format!("{name}@example.com");

        saved_profile(
            name,
            identity(&subject, &user_id, &account_id, Some(&email)),
            ProfileUsageState::Available(usage),
        )
    }

    fn available_saved_profile_with_resets(
        name: &str,
        primary_used_percent: f64,
        primary_reset_at: i64,
        secondary_used_percent: f64,
        secondary_reset_at: i64,
    ) -> SavedProfile {
        let subject = format!("sub-{name}");
        let user_id = format!("user-{name}");
        let account_id = format!("acct-{name}");
        let email = format!("{name}@example.com");

        saved_profile(
            name,
            identity(&subject, &user_id, &account_id, Some(&email)),
            ProfileUsageState::Available(ProfileUsageSnapshot {
                user_id: Some(user_id),
                account_id: Some(account_id),
                email: Some(email),
                plan_type: Some("plus".into()),
                primary: Some(UsageWindowSnapshot {
                    used_percent: primary_used_percent,
                    reset_at: Some(primary_reset_at),
                    limit_multiplier: 1.0,
                }),
                secondary: Some(UsageWindowSnapshot {
                    used_percent: secondary_used_percent,
                    reset_at: Some(secondary_reset_at),
                    limit_multiplier: 1.0,
                }),
            }),
        )
    }

    fn identity(
        subject: &str,
        user_id: &str,
        account_id: &str,
        email: Option<&str>,
    ) -> AuthIdentity {
        AuthIdentity {
            auth_mode: Some("chatgpt".into()),
            subject: Some(subject.into()),
            user_id: Some(user_id.into()),
            chatgpt_account_id: Some(account_id.into()),
            email: email.map(str::to_owned),
            name: None,
            auth_provider: Some("google".into()),
        }
    }

    fn auth_json(id: &TestIdentity) -> String {
        serde_json::to_string(&json!({
            "OPENAI_API_KEY": null,
            "auth_mode": "chatgpt",
            "last_refresh": "2026-03-30T00:00:00Z",
            "tokens": {
                "access_token": "access-token",
                "account_id": id.account_id,
                "id_token": jwt(id),
                "refresh_token": "refresh-token",
            }
        }))
        .unwrap()
    }

    fn row_status_text(rows: &[super::ProfileRow], profile: &str) -> String {
        rows.iter()
            .find(|row| row.profile == profile)
            .unwrap()
            .status
            .text()
    }

    fn row_field(
        rows: &[super::ProfileRow],
        profile: &str,
        value: impl Fn(&super::ProfileRow) -> String,
    ) -> String {
        value(rows.iter().find(|row| row.profile == profile).unwrap())
    }

    fn row_style(rows: &[super::ProfileRow], profile: &str) -> ProfileStyleKind {
        rows.iter()
            .find(|row| row.profile == profile)
            .unwrap()
            .status
            .whole_row_style()
    }

    fn row_limit_style(
        rows: &[super::ProfileRow],
        profile: &str,
        value: impl Fn(&super::ProfileRow) -> LimitStyleKind,
    ) -> LimitStyleKind {
        value(rows.iter().find(|row| row.profile == profile).unwrap())
    }

    #[test]
    fn current_usage_table_only_shows_email_and_limits() {
        let mut output = Vec::new();

        print_current_usage_table(
            &mut output,
            "praveen@example.com",
            &ProfileUsageState::Available(usage_snapshot("plus", "user-1", "acct-1", 42.0, 73.0)),
        )
        .unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("EMAIL"));
        assert!(output.contains("5 HOUR LIMIT"));
        assert!(output.contains("WEEKLY LIMIT"));
        assert!(output.contains("praveen@example.com"));
        assert!(output.contains(" 42% ("));
        assert!(output.contains(" 73% ("));
        assert!(!output.contains("PROFILE"));
        assert!(!output.contains("TOTAL"));
        assert!(!output.contains("PLAN"));
        assert!(!output.contains("STATUS"));
    }

    #[test]
    fn current_usage_view_uses_usage_email_and_limits() {
        let dir = tempdir().unwrap();
        let auth_path = dir.path().join(".codex").join("auth.json");
        fs::create_dir_all(auth_path.parent().unwrap()).unwrap();
        fs::write(&auth_path, auth_json(&ID_1)).unwrap();

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let server = runtime.block_on(async {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/backend-api/wham/usage"))
                .and(header("authorization", "Bearer access-token"))
                .respond_with(ResponseTemplate::new(200).set_body_json(usage_response(
                    "fresh@example.com",
                    "user-1",
                    "acct-1",
                    "plus",
                    42.0,
                    73.0,
                )))
                .mount(&server)
                .await;
            server
        });

        let loader =
            ProfileUsageLoader::with_urls(format!("{}/backend-api/wham/usage", server.uri()))
                .unwrap();
        let (label, usage) = current_usage_view(&loader, &auth_path).unwrap();

        assert_eq!(label, "fresh@example.com");
        let ProfileUsageState::Available(usage) = usage else {
            panic!("expected available usage");
        };
        assert_eq!(usage.email.as_deref(), Some("fresh@example.com"));
        assert_eq!(usage.plan_type.as_deref(), Some("plus"));
    }

    #[test]
    fn current_usage_view_uses_global_auth_even_when_saved_profile_matches() {
        let dir = tempdir().unwrap();
        let global_auth_path = dir.path().join(".codex").join("auth.json");
        let matching_profile_auth_path = dir
            .path()
            .join(".codex")
            .join("profiles")
            .join("a")
            .join("auth.json");
        let other_auth_path = dir
            .path()
            .join(".codex")
            .join("profiles")
            .join("b")
            .join("auth.json");
        fs::create_dir_all(global_auth_path.parent().unwrap()).unwrap();
        fs::create_dir_all(matching_profile_auth_path.parent().unwrap()).unwrap();
        fs::create_dir_all(other_auth_path.parent().unwrap()).unwrap();
        fs::write(
            &global_auth_path,
            auth_json_with_tokens(&ID_1, "old-access", "old-refresh"),
        )
        .unwrap();
        fs::write(
            &matching_profile_auth_path,
            auth_json_with_tokens(&ID_1, "fresh-access", "fresh-refresh"),
        )
        .unwrap();
        fs::write(
            &other_auth_path,
            auth_json_with_tokens(&ID_2, "other-access", "other-refresh"),
        )
        .unwrap();

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let server = runtime.block_on(async {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/backend-api/wham/usage"))
                .and(header("authorization", "Bearer old-access"))
                .respond_with(ResponseTemplate::new(503))
                .mount(&server)
                .await;
            Mock::given(method("GET"))
                .and(path("/backend-api/wham/usage"))
                .and(header("authorization", "Bearer fresh-access"))
                .respond_with(ResponseTemplate::new(200).set_body_json(usage_response(
                    "fresh@example.com",
                    "user-1",
                    "acct-1",
                    "plus",
                    42.0,
                    73.0,
                )))
                .mount(&server)
                .await;
            server
        });

        let loader =
            ProfileUsageLoader::with_urls(format!("{}/backend-api/wham/usage", server.uri()))
                .unwrap();
        let (label, usage) = current_usage_view(&loader, &global_auth_path).unwrap();

        assert_eq!(label, "old@example.com");
        assert!(matches!(usage, ProfileUsageState::Unavailable));
    }

    #[test]
    fn current_usage_view_uses_global_auth_when_no_saved_profile_matches() {
        let dir = tempdir().unwrap();
        let global_auth_path = dir.path().join(".codex").join("auth.json");
        let profile_auth_path = dir
            .path()
            .join(".codex")
            .join("profiles")
            .join("a")
            .join("auth.json");
        fs::create_dir_all(global_auth_path.parent().unwrap()).unwrap();
        fs::create_dir_all(profile_auth_path.parent().unwrap()).unwrap();
        fs::write(
            &global_auth_path,
            auth_json_with_tokens(&ID_1, "old-access", "old-refresh"),
        )
        .unwrap();
        fs::write(
            &profile_auth_path,
            auth_json_with_tokens(&ID_2, "other-access", "other-refresh"),
        )
        .unwrap();

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let server = runtime.block_on(async {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/backend-api/wham/usage"))
                .and(header("authorization", "Bearer old-access"))
                .respond_with(ResponseTemplate::new(200).set_body_json(usage_response(
                    "fresh@example.com",
                    "user-1",
                    "acct-1",
                    "plus",
                    42.0,
                    73.0,
                )))
                .mount(&server)
                .await;
            server
        });

        let loader =
            ProfileUsageLoader::with_urls(format!("{}/backend-api/wham/usage", server.uri()))
                .unwrap();
        let (label, usage) = current_usage_view(&loader, &global_auth_path).unwrap();

        assert_eq!(label, "fresh@example.com");
        assert!(matches!(usage, ProfileUsageState::Available(_)));
    }

    #[test]
    fn current_usage_view_uses_global_auth_when_multiple_saved_profiles_match() {
        let dir = tempdir().unwrap();
        let global_auth_path = dir.path().join(".codex").join("auth.json");
        let profile_a_auth_path = dir
            .path()
            .join(".codex")
            .join("profiles")
            .join("a")
            .join("auth.json");
        let profile_b_auth_path = dir
            .path()
            .join(".codex")
            .join("profiles")
            .join("b")
            .join("auth.json");
        fs::create_dir_all(global_auth_path.parent().unwrap()).unwrap();
        fs::create_dir_all(profile_a_auth_path.parent().unwrap()).unwrap();
        fs::create_dir_all(profile_b_auth_path.parent().unwrap()).unwrap();
        fs::write(
            &global_auth_path,
            auth_json_with_tokens(&ID_1, "old-access", "old-refresh"),
        )
        .unwrap();
        fs::write(
            &profile_a_auth_path,
            auth_json_with_tokens(&ID_1, "fresh-access", "fresh-refresh"),
        )
        .unwrap();
        fs::write(
            &profile_b_auth_path,
            auth_json_with_tokens(&ID_1_ALIAS, "alias-access", "alias-refresh"),
        )
        .unwrap();

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let server = runtime.block_on(async {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/backend-api/wham/usage"))
                .and(header("authorization", "Bearer old-access"))
                .respond_with(ResponseTemplate::new(200).set_body_json(usage_response(
                    "fresh@example.com",
                    "user-1",
                    "acct-1",
                    "plus",
                    42.0,
                    73.0,
                )))
                .mount(&server)
                .await;
            server
        });

        let loader =
            ProfileUsageLoader::with_urls(format!("{}/backend-api/wham/usage", server.uri()))
                .unwrap();
        let (label, usage) = current_usage_view(&loader, &global_auth_path).unwrap();

        assert_eq!(label, "fresh@example.com");
        assert!(matches!(usage, ProfileUsageState::Available(_)));
    }

    #[test]
    fn active_list_profile_uses_global_auth_usage() {
        let active_auth = read_auth(&ID_1, "global-access", "global-refresh");
        let active_identity = identity("sub-1", "user-1", "acct-1", Some("old@example.com"));
        let mut profiles = vec![
            saved_profile(
                "a",
                identity("sub-1", "user-1", "acct-1", Some("profile@example.com")),
                ProfileUsageState::Available(usage_snapshot(
                    "plus", "user-1", "acct-1", 80.0, 90.0,
                )),
            ),
            saved_profile(
                "b",
                identity("sub-2", "user-2", "acct-2", Some("other@example.com")),
                ProfileUsageState::Available(usage_snapshot(
                    "plus", "user-2", "acct-2", 30.0, 40.0,
                )),
            ),
        ];

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let server = runtime.block_on(async {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/backend-api/wham/usage"))
                .and(header("authorization", "Bearer global-access"))
                .respond_with(ResponseTemplate::new(200).set_body_json(usage_response(
                    "global@example.com",
                    "user-1",
                    "acct-1",
                    "pro",
                    12.0,
                    34.0,
                )))
                .mount(&server)
                .await;
            server
        });

        let loader =
            ProfileUsageLoader::with_urls(format!("{}/backend-api/wham/usage", server.uri()))
                .unwrap();
        enrich_active_profiles_with_global_auth(
            &mut profiles,
            &loader,
            &active_auth,
            &active_identity,
        )
        .unwrap();

        let rows = build_profile_rows(&profiles, Some(&active_identity));

        assert_eq!(row_field(&rows, "a", |row| row.five_hour.clone()), "12%");
        assert_eq!(row_field(&rows, "a", |row| row.weekly.clone()), "34%");
        assert_eq!(row_field(&rows, "a", |row| row.plan.clone()), "Pro");
        assert_eq!(row_status_text(&rows, "a"), "active");
        assert_eq!(row_field(&rows, "b", |row| row.five_hour.clone()), "30%");
    }

    #[test]
    fn current_usage_view_marks_unavailable_when_usage_request_fails() {
        let dir = tempdir().unwrap();
        let auth_path = dir.path().join(".codex").join("auth.json");
        fs::create_dir_all(auth_path.parent().unwrap()).unwrap();
        fs::write(&auth_path, auth_json(&ID_1)).unwrap();

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let server = runtime.block_on(async {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/backend-api/wham/usage"))
                .respond_with(ResponseTemplate::new(503))
                .mount(&server)
                .await;
            server
        });

        let loader =
            ProfileUsageLoader::with_urls(format!("{}/backend-api/wham/usage", server.uri()))
                .unwrap();
        let (label, usage) = current_usage_view(&loader, &auth_path).unwrap();

        assert_eq!(label, "old@example.com");
        assert!(matches!(usage, ProfileUsageState::Unavailable));
    }

    #[test]
    fn current_usage_view_errors_when_usage_identity_mismatches() {
        let dir = tempdir().unwrap();
        let auth_path = dir.path().join(".codex").join("auth.json");
        fs::create_dir_all(auth_path.parent().unwrap()).unwrap();
        fs::write(&auth_path, auth_json(&ID_1)).unwrap();

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let server = runtime.block_on(async {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/backend-api/wham/usage"))
                .respond_with(ResponseTemplate::new(200).set_body_json(usage_response(
                    "fresh@example.com",
                    "user-2",
                    "acct-2",
                    "plus",
                    42.0,
                    73.0,
                )))
                .mount(&server)
                .await;
            server
        });

        let loader =
            ProfileUsageLoader::with_urls(format!("{}/backend-api/wham/usage", server.uri()))
                .unwrap();
        let err = current_usage_view(&loader, &auth_path).unwrap_err();

        assert!(err.to_string().contains("does not match usage identity"));
    }

    #[test]
    fn current_usage_view_errors_when_current_auth_is_missing() {
        let dir = tempdir().unwrap();
        let auth_path = dir.path().join(".codex").join("auth.json");
        let loader = ProfileUsageLoader::with_urls("http://localhost/unused").unwrap();

        let err = current_usage_view(&loader, &auth_path).unwrap_err();

        assert!(err.to_string().contains("No current codex auth found"));
    }

    fn usage_snapshot(
        plan_type: &str,
        user_id: &str,
        account_id: &str,
        primary_used_percent: f64,
        secondary_used_percent: f64,
    ) -> ProfileUsageSnapshot {
        let now = Local::now().timestamp();
        ProfileUsageSnapshot {
            user_id: Some(user_id.into()),
            account_id: Some(account_id.into()),
            email: None,
            plan_type: Some(plan_type.into()),
            primary: Some(UsageWindowSnapshot {
                used_percent: primary_used_percent,
                reset_at: Some(now + 3600),
                limit_multiplier: 1.0,
            }),
            secondary: Some(UsageWindowSnapshot {
                used_percent: secondary_used_percent,
                reset_at: Some(now + 7200),
                limit_multiplier: 1.0,
            }),
        }
    }

    fn reset_at_for_elapsed(
        now: chrono::DateTime<Utc>,
        kind: super::UsageWindowKind,
        elapsed_fraction: f64,
    ) -> i64 {
        let duration = match kind {
            super::UsageWindowKind::Primary => chrono::Duration::hours(5),
            super::UsageWindowKind::Secondary => chrono::Duration::days(7),
        };
        let remaining_seconds =
            ((1.0 - elapsed_fraction) * duration.num_seconds() as f64).round() as i64;

        (now + chrono::Duration::seconds(remaining_seconds)).timestamp()
    }

    fn read_auth(id: &TestIdentity, access_token: &str, refresh_token: &str) -> StoredAuth {
        let dir = tempdir().unwrap();
        let auth_path = dir.path().join("auth.json");
        fs::write(
            &auth_path,
            auth_json_with_tokens(id, access_token, refresh_token),
        )
        .unwrap();
        read_stored_auth(&auth_path).unwrap()
    }

    fn auth_json_with_tokens(id: &TestIdentity, access_token: &str, refresh_token: &str) -> String {
        serde_json::to_string(&json!({
            "OPENAI_API_KEY": null,
            "auth_mode": "chatgpt",
            "last_refresh": "2026-03-30T00:00:00Z",
            "tokens": {
                "access_token": access_token,
                "account_id": id.account_id,
                "id_token": jwt(id),
                "refresh_token": refresh_token,
            }
        }))
        .unwrap()
    }

    fn stored_auth_with_access_token_and_last_refresh(
        access_token: Option<String>,
        last_refresh: Option<chrono::DateTime<Utc>>,
    ) -> StoredAuth {
        StoredAuth {
            openai_api_key: None,
            auth_mode: Some("chatgpt".into()),
            last_refresh: last_refresh.map(|timestamp| timestamp.to_rfc3339()),
            tokens: Some(super::StoredTokens {
                account_id: Some("acct-1".into()),
                id_token: Some(jwt(&TestIdentity {
                    subject: "sub-1",
                    user_id: "user-1",
                    account_id: "acct-1",
                    email: Some("praveen@example.com"),
                    name: None,
                    auth_provider: Some("google"),
                })),
                access_token,
                refresh_token: Some("refresh-token".into()),
                extra: Default::default(),
            }),
            extra: Default::default(),
        }
    }

    fn access_jwt(exp: Option<i64>) -> String {
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&json!({
                "exp": exp,
            }))
            .unwrap(),
        );

        format!("{header}.{payload}.sig")
    }

    struct TestIdentity {
        subject: &'static str,
        user_id: &'static str,
        account_id: &'static str,
        email: Option<&'static str>,
        name: Option<&'static str>,
        auth_provider: Option<&'static str>,
    }

    const ID_1: TestIdentity = TestIdentity {
        subject: "sub-1",
        user_id: "user-1",
        account_id: "acct-1",
        email: Some("old@example.com"),
        name: None,
        auth_provider: Some("google"),
    };

    const ID_1_ALIAS: TestIdentity = TestIdentity {
        subject: "sub-1",
        user_id: "user-1",
        account_id: "acct-1",
        email: Some("alias@example.com"),
        name: None,
        auth_provider: Some("google"),
    };

    const ID_2: TestIdentity = TestIdentity {
        subject: "sub-2",
        user_id: "user-2",
        account_id: "acct-2",
        email: Some("other@example.com"),
        name: None,
        auth_provider: Some("google"),
    };

    fn jwt(id: &TestIdentity) -> String {
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&json!({
                "sub": id.subject,
                "email": id.email,
                "name": id.name,
                "auth_provider": id.auth_provider,
                "https://api.openai.com/auth": {
                    "user_id": id.user_id,
                    "chatgpt_account_id": id.account_id,
                }
            }))
            .unwrap(),
        );

        format!("{header}.{payload}.sig")
    }

    fn usage_response(
        email: &str,
        user_id: &str,
        account_id: &str,
        plan_type: &str,
        primary_used_percent: f64,
        secondary_used_percent: f64,
    ) -> serde_json::Value {
        let now = Local::now().timestamp();
        json!({
            "email": email,
            "user_id": user_id,
            "account_id": account_id,
            "plan_type": plan_type,
            "rate_limit": {
                "allowed": true,
                "limit_reached": false,
                "primary_window": {
                    "used_percent": primary_used_percent,
                    "limit_window_seconds": 18000,
                    "reset_after_seconds": 18000,
                    "reset_at": now + 3600,
                },
                "secondary_window": {
                    "used_percent": secondary_used_percent,
                    "limit_window_seconds": 604800,
                    "reset_after_seconds": 604800,
                    "reset_at": now + 7200,
                }
            }
        })
    }
}
