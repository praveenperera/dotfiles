use base64::{
    engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD},
    Engine as _,
};
use chrono::{Local, TimeZone, Utc};
use clap::{Args, Subcommand};
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
use std::fs;
use std::io::{self, ErrorKind, Write};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::time::Duration;
use xshell::Shell;

#[derive(Debug, Clone, Args)]
pub struct Codex {
    #[command(subcommand)]
    pub subcommand: CodexCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum CodexCmd {
    /// Launch codex with a specific profile
    Launch {
        /// Profile name
        profile: String,

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

    /// Refresh a saved profile's auth
    #[command(visible_alias = "rp")]
    RefreshProfile {
        /// Profile name to refresh
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
    provider: String,
    user: String,
    account: String,
    plan: String,
    five_hour: String,
    five_hour_reset: String,
    five_hour_compact: String,
    five_hour_style: LimitStyleKind,
    weekly: String,
    weekly_reset: String,
    weekly_compact: String,
    weekly_style: LimitStyleKind,
    status: ProfileStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SaveProfileOutcome {
    Saved,
    SkippedConflict,
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

fn codex_dir() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME must be set");
    PathBuf::from(home).join(".codex")
}

fn profiles_dir() -> PathBuf {
    codex_dir().join("profiles")
}

fn profile_codex_home(profile: &str) -> PathBuf {
    profiles_dir().join(profile)
}

fn auth_path() -> PathBuf {
    codex_dir().join("auth.json")
}

const CHATGPT_USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
const CHATGPT_REFRESH_URL: &str = "https://auth.openai.com/oauth/token";
const CHATGPT_REFRESH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const USAGE_FETCH_CONCURRENCY: usize = 4;
const USAGE_FETCH_TIMEOUT: Duration = Duration::from_secs(5);

pub fn run_with_flags(_sh: &Shell, flags: Codex) -> Result<()> {
    match flags.subcommand {
        CodexCmd::Launch { profile, args } => launch(&profile, &args),
        CodexCmd::Login {
            profile,
            device_auth,
        } => login(&profile, device_auth),
        CodexCmd::List { verbose } => list(verbose),
        CodexCmd::RefreshProfile { profile } => refresh_profile(&profile),
        CodexCmd::Delete { profile, yes } => delete(&profile, yes),
    }
}

fn launch(profile: &str, args: &[OsString]) -> Result<()> {
    let shared_codex_home = codex_dir();
    let profile_home = profile_codex_home(profile);
    let profile_auth = profile_home.join("auth.json");
    if !profile_auth.exists() {
        return Err(eyre!(
            "Profile '{profile}' not found. Run: cmd codex login {profile}"
        ));
    }

    sync_profile_codex_home(&profile_home, &shared_codex_home)?;
    let launch_home = tempfile::tempdir()?;
    let launch_auth = read_auth_snapshot(&profile_auth)?;
    sync_launch_codex_home(launch_home.path(), &shared_codex_home, &profile_auth)?;

    let status = codex_command(launch_home.path()).args(args).status()?;
    promote_launch_auth_if_unchanged(
        &profile_auth,
        &launch_auth,
        &launch_home.path().join("auth.json"),
    )?;

    std::process::exit(status.code().unwrap_or(1));
}

fn login(profile: &str, device_auth: bool) -> Result<()> {
    let shared_codex_home = codex_dir();
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
    let profiles = load_saved_profiles(&profiles_dir())?;
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
        let profile_home = profile_codex_home(profile);
        sync_profile_codex_home(&profile_home, &shared_codex_home)?;
        save_profile_auth(
            profile,
            &auth,
            &profiles_dir(),
            &conflicts,
            replace_conflicts,
        )?
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

fn list(verbose: bool) -> Result<()> {
    let mut profiles = load_saved_profiles(&profiles_dir())?;
    if profiles.is_empty() {
        println!("No profiles. Run: cmd codex login <name>");
        return Ok(());
    }

    enrich_profile_usage(&mut profiles)?;

    let active_identity = read_auth_identity(&auth_path()).ok();
    let rows = build_profile_rows(&profiles, active_identity.as_ref());
    if verbose {
        print_verbose_profile_table(&rows);
    } else {
        print_compact_profile_table(&rows);
    }

    Ok(())
}

fn print_compact_profile_table(rows: &[ProfileRow]) {
    let widths = compact_profile_table_widths(rows);

    println!(
        "{}   {}   {}   {}",
        format!(
            "{:<profile_width$}",
            "PROFILE",
            profile_width = widths.profile
        )
        .blue()
        .bold(),
        format!("{:<label_width$}", "EMAIL", label_width = widths.label)
            .blue()
            .bold(),
        format!(
            "{:<five_hour_width$}",
            "5 HOUR LIMIT",
            five_hour_width = widths.five_hour
        )
        .blue()
        .bold(),
        format!(
            "{:<weekly_width$}",
            "WEEKLY LIMIT",
            weekly_width = widths.weekly
        )
        .blue()
        .bold(),
    );

    for row in rows {
        println!(
            "{}   {}   {}   {}",
            colorize_row_cell(&row.profile, widths.profile, row),
            colorize_row_cell(&row.label, widths.label, row),
            colorize_limit_cell(
                &row.five_hour_compact,
                widths.five_hour,
                row.five_hour_style,
                row,
            ),
            colorize_limit_cell(&row.weekly_compact, widths.weekly, row.weekly_style, row),
        );
    }
}

fn print_verbose_profile_table(rows: &[ProfileRow]) {
    let widths = profile_table_widths(rows);

    println!(
        "{}  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}",
        format!(
            "{:<profile_width$}",
            "PROFILE",
            profile_width = widths.profile
        )
        .blue()
        .bold(),
        format!("{:<label_width$}", "EMAIL", label_width = widths.label)
            .blue()
            .bold(),
        format!(
            "{:<provider_width$}",
            "PROVIDER",
            provider_width = widths.provider
        )
        .blue()
        .bold(),
        format!("{:<user_width$}", "USER", user_width = widths.user)
            .blue()
            .bold(),
        format!(
            "{:<account_width$}",
            "ACCOUNT",
            account_width = widths.account
        )
        .blue()
        .bold(),
        format!("{:<plan_width$}", "PLAN", plan_width = widths.plan)
            .blue()
            .bold(),
        format!(
            "{:<five_hour_width$}",
            "5H",
            five_hour_width = widths.five_hour
        )
        .blue()
        .bold(),
        format!(
            "{:<five_hour_reset_width$}",
            "5H RESET",
            five_hour_reset_width = widths.five_hour_reset
        )
        .blue()
        .bold(),
        format!("{:<weekly_width$}", "WEEK", weekly_width = widths.weekly)
            .blue()
            .bold(),
        format!(
            "{:<weekly_reset_width$}",
            "WEEK RESET",
            weekly_reset_width = widths.weekly_reset
        )
        .blue()
        .bold(),
        "STATUS".blue().bold(),
    );

    for row in rows {
        println!(
            "{}  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}",
            colorize_row_cell(&row.profile, widths.profile, row),
            colorize_row_cell(&row.label, widths.label, row),
            colorize_row_cell(&row.provider, widths.provider, row),
            colorize_row_cell(&row.user, widths.user, row),
            colorize_row_cell(&row.account, widths.account, row),
            colorize_row_cell(&row.plan, widths.plan, row),
            colorize_limit_cell(&row.five_hour, widths.five_hour, row.five_hour_style, row),
            colorize_row_cell(&row.five_hour_reset, widths.five_hour_reset, row),
            colorize_limit_cell(&row.weekly, widths.weekly, row.weekly_style, row),
            colorize_row_cell(&row.weekly_reset, widths.weekly_reset, row),
            colorize_status(row),
        );
    }
}

fn refresh_profile(profile: &str) -> Result<()> {
    let profile_auth = profile_codex_home(profile).join("auth.json");
    if !profile_auth.exists() {
        return Err(eyre!("Profile '{profile}' not found. Run: cmd codex list"));
    }

    let launch_auth = read_auth_snapshot(&profile_auth)?;
    let auth = read_stored_auth(&profile_auth)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let refresher = ProfileAuthRefresher::new()?;
    let refreshed_auth =
        runtime.block_on(refresher.refresh_profile_auth(&auth, Some(&launch_auth.identity)))?;
    let refreshed_raw = serde_json::to_vec_pretty(&refreshed_auth)?;

    if write_auth_raw_if_unchanged(&profile_auth, &launch_auth.raw, &refreshed_raw)? {
        println!("Refreshed codex profile: {profile}");
    } else {
        println!("Skipped refreshing codex profile: {profile} (profile auth changed)");
    }

    Ok(())
}

fn delete(profile: &str, yes: bool) -> Result<()> {
    let profile_home = profile_codex_home(profile);
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

fn delete_profile_home(profile_home: &Path) -> Result<()> {
    if !profile_home.exists() {
        return Err(eyre!("Profile home not found: {}", profile_home.display()));
    }

    remove_existing_path(profile_home)
}

fn colorize_row_cell(value: &str, width: usize, row: &ProfileRow) -> String {
    let padded = format!("{value:<width$}");
    match row.status.whole_row_style() {
        ProfileStyleKind::Error => padded.red().bold().to_string(),
        _ => padded,
    }
}

fn colorize_limit_cell(
    value: &str,
    width: usize,
    style: LimitStyleKind,
    row: &ProfileRow,
) -> String {
    let padded = format!("{value:<width$}");
    if row.status.whole_row_style() == ProfileStyleKind::Error {
        return padded.red().bold().to_string();
    }

    match style {
        LimitStyleKind::Normal => padded,
        LimitStyleKind::Success => padded.green().to_string(),
        LimitStyleKind::Warning => padded.yellow().to_string(),
        LimitStyleKind::Caution => padded.truecolor(255, 165, 0).to_string(),
        LimitStyleKind::Error => padded.red().to_string(),
        LimitStyleKind::Critical => padded.red().bold().to_string(),
    }
}

fn colorize_status(row: &ProfileRow) -> String {
    row.status.render(row.status.whole_row_style())
}

impl ProfileStatus {
    fn push(&mut self, item: ProfileStatusItem) {
        self.items.push(item);
    }

    fn text(&self) -> String {
        if self.items.is_empty() {
            return "-".into();
        }

        self.items
            .iter()
            .map(ProfileStatusItem::text)
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn whole_row_style(&self) -> ProfileStyleKind {
        if self
            .items
            .iter()
            .any(|item| matches!(item, ProfileStatusItem::SameUser(_)))
        {
            ProfileStyleKind::Error
        } else {
            ProfileStyleKind::Normal
        }
    }

    fn render(&self, whole_row_style: ProfileStyleKind) -> String {
        if self.items.is_empty() {
            return "-".into();
        }

        if whole_row_style == ProfileStyleKind::Error {
            return self.text().red().bold().to_string();
        }

        self.items
            .iter()
            .map(ProfileStatusItem::render)
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl ProfileStatusItem {
    fn text(&self) -> String {
        match self {
            Self::Active => "active".into(),
            Self::SameUser(profiles) => format!("same-user-as:{}", profiles.join(",")),
            Self::SharedAccount(profiles) => {
                format!("shared-account-with:{}", profiles.join(","))
            }
            Self::InvalidAuth => "invalid-auth".into(),
            Self::ReauthNeeded => "reauth-needed".into(),
            Self::UsageUnavailable => "usage-unavailable".into(),
        }
    }

    fn style_kind(&self) -> ProfileStyleKind {
        match self {
            Self::Active => ProfileStyleKind::Success,
            Self::SameUser(_) => ProfileStyleKind::Error,
            Self::SharedAccount(_)
            | Self::InvalidAuth
            | Self::ReauthNeeded
            | Self::UsageUnavailable => ProfileStyleKind::Warning,
        }
    }

    fn render(&self) -> String {
        let text = self.text();
        match self.style_kind() {
            ProfileStyleKind::Success => text.green().to_string(),
            ProfileStyleKind::Warning => text.yellow().to_string(),
            ProfileStyleKind::Error => text.red().bold().to_string(),
            ProfileStyleKind::Normal => text,
        }
    }
}

impl ProfileUsageLoader {
    fn new() -> Result<Self> {
        let http = HttpClient::builder().timeout(USAGE_FETCH_TIMEOUT).build()?;
        Ok(Self {
            http,
            usage_url: CHATGPT_USAGE_URL.into(),
        })
    }

    #[cfg(test)]
    fn with_urls(usage_url: impl Into<String>) -> Result<Self> {
        let http = HttpClient::builder().timeout(USAGE_FETCH_TIMEOUT).build()?;
        Ok(Self {
            http,
            usage_url: usage_url.into(),
        })
    }

    async fn load_updates(&self, profiles: &[SavedProfile]) -> Vec<ProfileUsageUpdate> {
        stream::iter(profiles.iter().cloned())
            .map(|profile| {
                let loader = self.clone();
                async move { loader.load_profile(profile).await }
            })
            .buffer_unordered(USAGE_FETCH_CONCURRENCY)
            .collect()
            .await
    }

    async fn load_profile(&self, profile: SavedProfile) -> ProfileUsageUpdate {
        if profile.invalid_auth {
            return ProfileUsageUpdate {
                profile: profile.name,
                identity: None,
                invalid_auth: true,
                usage: ProfileUsageState::Unchecked,
            };
        }

        let auth = match read_stored_auth(&profile.auth_path) {
            Ok(auth) => auth,
            Err(_) => {
                return ProfileUsageUpdate {
                    profile: profile.name,
                    identity: None,
                    invalid_auth: true,
                    usage: ProfileUsageState::Unchecked,
                };
            }
        };

        let result = match self
            .fetch_profile_usage(&auth, profile.identity.as_ref())
            .await
        {
            Ok(result) => result,
            Err(_) => UsageFetchResult::Unavailable {
                identity: profile.identity.clone(),
            },
        };

        match result {
            UsageFetchResult::Available { identity, usage } => ProfileUsageUpdate {
                profile: profile.name,
                identity: identity.or(profile.identity),
                invalid_auth: false,
                usage: ProfileUsageState::Available(usage),
            },
            UsageFetchResult::ReauthNeeded => ProfileUsageUpdate {
                profile: profile.name,
                identity: profile.identity,
                invalid_auth: false,
                usage: ProfileUsageState::ReauthNeeded,
            },
            UsageFetchResult::Unavailable { identity } => ProfileUsageUpdate {
                profile: profile.name,
                identity: identity.or(profile.identity),
                invalid_auth: false,
                usage: ProfileUsageState::Unavailable,
            },
        }
    }

    async fn fetch_profile_usage(
        &self,
        auth: &StoredAuth,
        expected_identity: Option<&AuthIdentity>,
    ) -> Result<UsageFetchResult> {
        let usage = match self.fetch_usage(auth).await {
            Ok(usage) => usage,
            Err(_) => {
                return Ok(UsageFetchResult::Unavailable {
                    identity: expected_identity.cloned(),
                })
            }
        };

        if let Some(expected_identity) = expected_identity {
            if !usage_matches_identity(&usage, expected_identity) {
                return Ok(UsageFetchResult::ReauthNeeded);
            }
        }

        Ok(UsageFetchResult::Available {
            identity: expected_identity.cloned(),
            usage,
        })
    }

    async fn fetch_usage(
        &self,
        auth: &StoredAuth,
    ) -> std::result::Result<ProfileUsageSnapshot, UsageHttpError> {
        let token = auth
            .tokens
            .as_ref()
            .and_then(|tokens| tokens.access_token.as_deref())
            .filter(|token| !token.is_empty())
            .ok_or_else(|| UsageHttpError::other("Missing access_token"))?;

        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("codex-cli"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|err| UsageHttpError::other(err.to_string()))?,
        );

        if let Some(account_id) = auth
            .tokens
            .as_ref()
            .and_then(|tokens| tokens.account_id.as_deref())
            .filter(|account_id| !account_id.is_empty())
        {
            if let Ok(name) = HeaderName::from_bytes(b"ChatGPT-Account-Id") {
                if let Ok(value) = HeaderValue::from_str(account_id) {
                    headers.insert(name, value);
                }
            }
        }

        let response = self
            .http
            .get(&self.usage_url)
            .headers(headers)
            .send()
            .await
            .map_err(UsageHttpError::from_reqwest)?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(UsageHttpError::with_status(
                status,
                if body.is_empty() {
                    status.to_string()
                } else {
                    body
                },
            ));
        }

        let payload = response
            .json::<UsageResponse>()
            .await
            .map_err(UsageHttpError::from_reqwest)?;
        Ok(ProfileUsageSnapshot {
            user_id: payload.user_id,
            account_id: payload.account_id,
            email: payload.email,
            plan_type: payload.plan_type,
            primary: payload.rate_limit.as_ref().and_then(|rate_limit| {
                rate_limit
                    .primary_window
                    .as_ref()
                    .map(|window| UsageWindowSnapshot {
                        used_percent: window.used_percent,
                        reset_at: Some(window.reset_at),
                    })
            }),
            secondary: payload.rate_limit.as_ref().and_then(|rate_limit| {
                rate_limit
                    .secondary_window
                    .as_ref()
                    .map(|window| UsageWindowSnapshot {
                        used_percent: window.used_percent,
                        reset_at: Some(window.reset_at),
                    })
            }),
        })
    }
}

impl ProfileAuthRefresher {
    fn new() -> Result<Self> {
        let http = HttpClient::builder().timeout(USAGE_FETCH_TIMEOUT).build()?;
        Ok(Self {
            http,
            refresh_url: CHATGPT_REFRESH_URL.into(),
        })
    }

    #[cfg(test)]
    fn with_url(refresh_url: impl Into<String>) -> Result<Self> {
        let http = HttpClient::builder().timeout(USAGE_FETCH_TIMEOUT).build()?;
        Ok(Self {
            http,
            refresh_url: refresh_url.into(),
        })
    }

    async fn refresh_profile_auth(
        &self,
        auth: &StoredAuth,
        expected_identity: Option<&AuthIdentity>,
    ) -> Result<StoredAuth> {
        let refresh_token = auth
            .tokens
            .as_ref()
            .and_then(|tokens| tokens.refresh_token.as_deref())
            .filter(|token| !token.is_empty())
            .ok_or_else(|| eyre!("Missing refresh_token in auth.json"))?;

        let request = RefreshRequest {
            client_id: CHATGPT_REFRESH_CLIENT_ID,
            grant_type: "refresh_token",
            refresh_token,
            scope: "openid profile email",
        };
        let response = self
            .http
            .post(&self.refresh_url)
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
            .json(&request)
            .send()
            .await?;

        if response.status() == StatusCode::UNAUTHORIZED {
            return Err(eyre!("Refresh token rejected"));
        }
        if !response.status().is_success() {
            return Err(eyre!("Refresh token request failed: {}", response.status()));
        }

        let payload = response.json::<RefreshResponse>().await?;
        let mut refreshed = auth.clone();
        let tokens = refreshed.tokens.get_or_insert_with(StoredTokens::default);

        if let Some(id_token) = payload.id_token {
            tokens.id_token = Some(id_token);
        }
        if let Some(access_token) = payload.access_token {
            tokens.access_token = Some(access_token);
        }
        if let Some(refresh_token) = payload.refresh_token {
            tokens.refresh_token = Some(refresh_token);
        }
        refreshed.last_refresh = Some(Utc::now().to_rfc3339());

        if let Some(expected_identity) = expected_identity {
            let refreshed_identity = stored_auth_identity(&refreshed)?;
            if !matches_launch_identity(expected_identity, &refreshed_identity) {
                return Err(eyre!(
                    "Refreshed auth does not match saved profile identity"
                ));
            }
        }

        Ok(refreshed)
    }
}

#[derive(Debug)]
struct UsageHttpError {
    message: String,
}

impl UsageHttpError {
    fn with_status(_status: StatusCode, message: String) -> Self {
        Self { message }
    }

    fn other(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    fn from_reqwest(err: reqwest::Error) -> Self {
        Self {
            message: err.to_string(),
        }
    }
}

impl std::fmt::Display for UsageHttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for UsageHttpError {}

fn enrich_profile_usage(profiles: &mut [SavedProfile]) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let loader = ProfileUsageLoader::new()?;
    let updates = runtime.block_on(loader.load_updates(profiles));

    for update in updates {
        if let Some(profile) = profiles
            .iter_mut()
            .find(|profile| profile.name == update.profile)
        {
            profile.identity = update.identity;
            profile.invalid_auth = update.invalid_auth;
            profile.usage = update.usage;
        }
    }

    Ok(())
}

fn codex_command(codex_home: &Path) -> std::process::Command {
    let mut command = std::process::Command::new("codex");
    command.env("CODEX_HOME", codex_home);
    command
}

fn sync_profile_codex_home(profile_home: &Path, shared_codex_home: &Path) -> Result<()> {
    fs::create_dir_all(profile_home)?;

    for entry in fs::read_dir(shared_codex_home)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };

        if is_profile_local_entry(name) {
            continue;
        }

        let source = entry.path();
        let target = profile_home.join(name);
        sync_shared_entry(&source, &target)?;
    }

    Ok(())
}

fn sync_launch_codex_home(
    launch_home: &Path,
    shared_codex_home: &Path,
    profile_auth: &Path,
) -> Result<()> {
    sync_profile_codex_home(launch_home, shared_codex_home)?;
    copy_auth_file(profile_auth, &launch_home.join("auth.json"))
}

fn is_profile_local_entry(name: &str) -> bool {
    matches!(name, "auth.json" | "profiles")
}

fn copy_auth_file(source: &Path, target: &Path) -> Result<()> {
    remove_existing_path(target)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, target)?;
    Ok(())
}

fn sync_shared_entry(source: &Path, target: &Path) -> Result<()> {
    if symlink_points_to(target, source)? {
        return Ok(());
    }

    remove_existing_path(target)?;

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    symlink(source, target)?;
    Ok(())
}

fn symlink_points_to(target: &Path, source: &Path) -> Result<bool> {
    match fs::read_link(target) {
        Ok(existing) => Ok(existing == source),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(false),
        Err(err) if err.kind() == ErrorKind::InvalidInput => Ok(false),
        Err(err) => Err(err.into()),
    }
}

fn remove_existing_path(path: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err.into()),
    };

    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }

    Ok(())
}

fn read_auth_identity(path: &Path) -> Result<AuthIdentity> {
    let auth = read_stored_auth(path)?;
    stored_auth_identity(&auth)
}

fn read_stored_auth(path: &Path) -> Result<StoredAuth> {
    let auth = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&auth)?)
}

#[derive(Debug, Clone)]
struct AuthSnapshot {
    raw: Vec<u8>,
    identity: AuthIdentity,
}

fn read_auth_snapshot(path: &Path) -> Result<AuthSnapshot> {
    let raw = fs::read(path)?;
    let identity = parse_auth_identity_bytes(&raw)?;
    Ok(AuthSnapshot { raw, identity })
}

fn parse_auth_identity_bytes(raw: &[u8]) -> Result<AuthIdentity> {
    let auth = std::str::from_utf8(raw).wrap_err("auth.json is not valid UTF-8")?;
    parse_auth_identity(auth)
}

// only promote auth back to the saved profile when nothing else changed it mid-run
fn promote_launch_auth_if_unchanged(
    profile_auth: &Path,
    launch_auth: &AuthSnapshot,
    final_launch_auth_path: &Path,
) -> Result<()> {
    let Ok(final_launch_raw) = fs::read(final_launch_auth_path) else {
        return Ok(());
    };
    if final_launch_raw == launch_auth.raw {
        return Ok(());
    }

    let Ok(final_launch_identity) = parse_auth_identity_bytes(&final_launch_raw) else {
        return Ok(());
    };
    if !matches_launch_identity(&launch_auth.identity, &final_launch_identity) {
        return Ok(());
    }

    write_auth_raw_if_unchanged(profile_auth, &launch_auth.raw, &final_launch_raw)?;
    Ok(())
}

fn write_auth_raw_if_unchanged(
    path: &Path,
    expected_raw: &[u8],
    replacement_raw: &[u8],
) -> Result<bool> {
    let Ok(current_raw) = fs::read(path) else {
        return Ok(false);
    };
    if current_raw != expected_raw {
        return Ok(false);
    }

    fs::write(path, replacement_raw)?;
    Ok(true)
}

fn matches_launch_identity(initial: &AuthIdentity, final_auth: &AuthIdentity) -> bool {
    if !is_same_user(initial, final_auth) {
        return false;
    }

    match (
        initial.chatgpt_account_id.as_deref(),
        final_auth.chatgpt_account_id.as_deref(),
    ) {
        (Some(initial_account), Some(final_account)) => initial_account == final_account,
        _ => true,
    }
}

fn stored_auth_identity(auth: &StoredAuth) -> Result<AuthIdentity> {
    parse_auth_identity(&serde_json::to_string(auth)?)
}

fn parse_auth_identity(auth: &str) -> Result<AuthIdentity> {
    let auth: StoredAuth = serde_json::from_str(auth)?;
    let tokens = auth
        .tokens
        .ok_or_else(|| eyre!("Missing tokens in auth.json"))?;
    let id_token = tokens
        .id_token
        .as_deref()
        .ok_or_else(|| eyre!("Missing id_token in auth.json"))?;
    let claims = parse_id_token_claims(id_token)?;
    let openai_auth = claims.openai_auth.unwrap_or_default();

    Ok(AuthIdentity {
        auth_mode: auth.auth_mode,
        subject: claims.sub,
        user_id: openai_auth.user_id,
        chatgpt_account_id: openai_auth.chatgpt_account_id.or(tokens.account_id),
        email: claims.email,
        name: claims.name,
        auth_provider: claims.auth_provider,
    })
}

fn parse_id_token_claims(id_token: &str) -> Result<IdTokenClaims> {
    let payload = jwt_payload(id_token)?;
    Ok(serde_json::from_slice(&payload)?)
}

fn jwt_payload(token: &str) -> Result<Vec<u8>> {
    let mut parts = token.split('.');
    let _header = parts.next().ok_or_else(|| eyre!("Malformed JWT"))?;
    let payload = parts.next().ok_or_else(|| eyre!("Malformed JWT"))?;
    let _signature = parts.next().ok_or_else(|| eyre!("Malformed JWT"))?;
    let padded = pad_base64url(payload);
    URL_SAFE_NO_PAD
        .decode(payload.as_bytes())
        .or_else(|_| URL_SAFE.decode(padded.as_bytes()))
        .wrap_err("Failed to decode JWT payload")
}

fn pad_base64url(value: &str) -> String {
    let mut value = value.to_owned();
    let padding = (4 - value.len() % 4) % 4;
    for _ in 0..padding {
        value.push('=');
    }
    value
}

fn load_saved_profiles(dir: &Path) -> Result<Vec<SavedProfile>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut profiles = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let auth_path = entry.path().join("auth.json");
        if !auth_path.exists() {
            continue;
        }

        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };

        let (identity, invalid_auth) = match read_auth_identity(&auth_path) {
            Ok(identity) => (Some(identity), false),
            Err(_) => (None, true),
        };

        profiles.push(SavedProfile {
            name,
            auth_path,
            identity,
            invalid_auth,
            usage: ProfileUsageState::Unchecked,
        });
    }

    profiles.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(profiles)
}

fn conflicting_profiles(
    profiles: &[SavedProfile],
    requested_profile: &str,
    identity: &AuthIdentity,
) -> Vec<String> {
    profiles
        .iter()
        .filter(|profile| profile.name != requested_profile)
        .filter_map(|profile| {
            profile
                .identity
                .as_ref()
                .filter(|existing| is_same_user(existing, identity))
                .map(|_| profile.name.clone())
        })
        .collect()
}

fn prompt_for_replacement(conflicts: &[String], requested_profile: &str) -> Result<bool> {
    let existing = conflicts.join(", ");
    prompt_for_confirmation(&format!("Replace '{existing}' with '{requested_profile}'?"))
}

fn prompt_for_confirmation(prompt: &str) -> Result<bool> {
    print!("{prompt} [y/N] ");
    io::stdout().flush()?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn save_profile_auth(
    profile: &str,
    auth_path: &Path,
    profiles_dir: &Path,
    conflicts: &[String],
    replace_conflicts: bool,
) -> Result<SaveProfileOutcome> {
    if !conflicts.is_empty() && !replace_conflicts {
        return Ok(SaveProfileOutcome::SkippedConflict);
    }

    for conflict in conflicts {
        let conflict_dir = profiles_dir.join(conflict);
        if conflict_dir.exists() {
            fs::remove_dir_all(&conflict_dir)?;
        }
    }

    let profile_dir = profiles_dir.join(profile);
    fs::create_dir_all(&profile_dir)?;
    fs::copy(auth_path, profile_dir.join("auth.json"))?;

    Ok(SaveProfileOutcome::Saved)
}

fn build_profile_rows(
    profiles: &[SavedProfile],
    active_identity: Option<&AuthIdentity>,
) -> Vec<ProfileRow> {
    profiles
        .iter()
        .map(|profile| {
            let Some(identity) = profile.identity.as_ref() else {
                return ProfileRow {
                    profile: profile.name.clone(),
                    label: "-".into(),
                    provider: "-".into(),
                    user: "-".into(),
                    account: "-".into(),
                    plan: "-".into(),
                    five_hour: "-".into(),
                    five_hour_reset: "-".into(),
                    five_hour_compact: "-".into(),
                    five_hour_style: LimitStyleKind::Normal,
                    weekly: "-".into(),
                    weekly_reset: "-".into(),
                    weekly_compact: "-".into(),
                    weekly_style: LimitStyleKind::Normal,
                    status: ProfileStatus {
                        items: if profile.invalid_auth {
                            vec![ProfileStatusItem::InvalidAuth]
                        } else {
                            Vec::new()
                        },
                    },
                };
            };

            let mut status = ProfileStatus::default();

            if active_identity.is_some_and(|active| is_same_user(active, identity)) {
                status.push(ProfileStatusItem::Active);
            }

            let same_user_profiles = profiles
                .iter()
                .filter(|other| other.name != profile.name)
                .filter_map(|other| {
                    other
                        .identity
                        .as_ref()
                        .filter(|other_identity| is_same_user(identity, other_identity))
                        .map(|_| other.name.clone())
                })
                .collect::<Vec<_>>();

            if !same_user_profiles.is_empty() {
                status.push(ProfileStatusItem::SameUser(same_user_profiles));
            }

            let shared_account_profiles = profiles
                .iter()
                .filter(|other| other.name != profile.name)
                .filter_map(|other| {
                    other
                        .identity
                        .as_ref()
                        .filter(|other_identity| shares_account(identity, other_identity))
                        .map(|_| other.name.clone())
                })
                .collect::<Vec<_>>();

            if !shared_account_profiles.is_empty() {
                status.push(ProfileStatusItem::SharedAccount(shared_account_profiles));
            }

            match profile.usage {
                ProfileUsageState::ReauthNeeded => status.push(ProfileStatusItem::ReauthNeeded),
                ProfileUsageState::Unavailable => {
                    status.push(ProfileStatusItem::UsageUnavailable);
                }
                ProfileUsageState::Unchecked | ProfileUsageState::Available(_) => {}
            }

            ProfileRow {
                profile: profile.name.clone(),
                label: best_label(identity),
                provider: identity
                    .auth_provider
                    .clone()
                    .or_else(|| identity.auth_mode.clone())
                    .unwrap_or_else(|| "-".into()),
                user: identity
                    .user_id
                    .as_deref()
                    .map(shorten_id)
                    .unwrap_or_else(|| {
                        identity
                            .subject
                            .as_deref()
                            .map(shorten_id)
                            .unwrap_or_else(|| "-".into())
                    }),
                account: identity
                    .chatgpt_account_id
                    .as_deref()
                    .map(shorten_id)
                    .unwrap_or_else(|| "-".into()),
                plan: usage_plan(&profile.usage),
                five_hour: usage_window_percent(&profile.usage, UsageWindowKind::Primary),
                five_hour_reset: usage_window_reset(&profile.usage, UsageWindowKind::Primary),
                five_hour_compact: usage_window_compact(&profile.usage, UsageWindowKind::Primary),
                five_hour_style: five_hour_limit_style(&profile.usage),
                weekly: usage_window_percent(&profile.usage, UsageWindowKind::Secondary),
                weekly_reset: usage_window_reset(&profile.usage, UsageWindowKind::Secondary),
                weekly_compact: usage_window_compact(&profile.usage, UsageWindowKind::Secondary),
                weekly_style: usage_window_style(&profile.usage, UsageWindowKind::Secondary),
                status,
            }
        })
        .collect()
}

fn profile_table_widths(rows: &[ProfileRow]) -> ProfileTableWidths {
    rows.iter().fold(
        ProfileTableWidths {
            profile: "PROFILE".len(),
            label: "EMAIL".len(),
            provider: "PROVIDER".len(),
            user: "USER".len(),
            account: "ACCOUNT".len(),
            plan: "PLAN".len(),
            five_hour: "5H".len(),
            five_hour_reset: "5H RESET".len(),
            weekly: "WEEK".len(),
            weekly_reset: "WEEK RESET".len(),
        },
        |widths, row| ProfileTableWidths {
            profile: widths.profile.max(row.profile.len()),
            label: widths.label.max(row.label.len()),
            provider: widths.provider.max(row.provider.len()),
            user: widths.user.max(row.user.len()),
            account: widths.account.max(row.account.len()),
            plan: widths.plan.max(row.plan.len()),
            five_hour: widths.five_hour.max(row.five_hour.len()),
            five_hour_reset: widths.five_hour_reset.max(row.five_hour_reset.len()),
            weekly: widths.weekly.max(row.weekly.len()),
            weekly_reset: widths.weekly_reset.max(row.weekly_reset.len()),
        },
    )
}

fn compact_profile_table_widths(rows: &[ProfileRow]) -> CompactProfileTableWidths {
    rows.iter().fold(
        CompactProfileTableWidths {
            profile: "PROFILE".len(),
            label: "EMAIL".len(),
            five_hour: "5 HOUR LIMIT".len(),
            weekly: "WEEKLY LIMIT".len(),
        },
        |widths, row| CompactProfileTableWidths {
            profile: widths.profile.max(row.profile.len()),
            label: widths.label.max(row.label.len()),
            five_hour: widths.five_hour.max(row.five_hour_compact.len()),
            weekly: widths.weekly.max(row.weekly_compact.len()),
        },
    )
}

#[derive(Debug, Clone, Copy)]
enum UsageWindowKind {
    Primary,
    Secondary,
}

fn usage_plan(usage: &ProfileUsageState) -> String {
    match usage {
        ProfileUsageState::Available(snapshot) => snapshot
            .plan_type
            .as_deref()
            .map(title_case)
            .unwrap_or_else(|| "-".into()),
        _ => "-".into(),
    }
}

fn usage_window_percent(usage: &ProfileUsageState, kind: UsageWindowKind) -> String {
    usage_window(usage, kind)
        .map(|window| format!("{:.0}%", window.used_percent))
        .unwrap_or_else(|| "-".into())
}

fn usage_window_style(usage: &ProfileUsageState, kind: UsageWindowKind) -> LimitStyleKind {
    usage_window(usage, kind)
        .map(|window| limit_style(window.used_percent))
        .unwrap_or(LimitStyleKind::Normal)
}

fn five_hour_limit_style(usage: &ProfileUsageState) -> LimitStyleKind {
    let weekly_exhausted = usage_window(usage, UsageWindowKind::Secondary)
        .is_some_and(|window| format!("{:.0}", window.used_percent) == "100");

    if weekly_exhausted {
        LimitStyleKind::Critical
    } else {
        usage_window_style(usage, UsageWindowKind::Primary)
    }
}

fn usage_window_reset(usage: &ProfileUsageState, kind: UsageWindowKind) -> String {
    usage_window(usage, kind)
        .and_then(|window| window.reset_at)
        .and_then(|timestamp| Local.timestamp_opt(timestamp, 0).single())
        .map(|timestamp| format_reset_timestamp(timestamp, Local::now()))
        .unwrap_or_else(|| "-".into())
}

fn usage_window_compact(usage: &ProfileUsageState, kind: UsageWindowKind) -> String {
    let percent = usage_window_percent(usage, kind);
    let reset = usage_window_reset_compact(usage, kind);

    match (percent.as_str(), reset.as_str()) {
        ("-", _) => "-".into(),
        (_, "-") => format_compact_percent(&percent),
        _ => format!("{} ({reset})", format_compact_percent(&percent)),
    }
}

fn format_compact_percent(percent: &str) -> String {
    let Some(number) = percent.strip_suffix('%') else {
        return percent.to_string();
    };

    format!("{number:>3}%")
}

fn usage_window_reset_compact(usage: &ProfileUsageState, kind: UsageWindowKind) -> String {
    usage_window(usage, kind)
        .and_then(|window| window.reset_at)
        .and_then(|timestamp| Local.timestamp_opt(timestamp, 0).single())
        .map(|timestamp| format_reset_timestamp_compact(timestamp, Local::now(), kind))
        .unwrap_or_else(|| "-".into())
}

fn usage_window(usage: &ProfileUsageState, kind: UsageWindowKind) -> Option<&UsageWindowSnapshot> {
    match usage {
        ProfileUsageState::Available(snapshot) => match kind {
            UsageWindowKind::Primary => snapshot.primary.as_ref(),
            UsageWindowKind::Secondary => snapshot.secondary.as_ref(),
        },
        _ => None,
    }
}

fn limit_style(used_percent: f64) -> LimitStyleKind {
    if used_percent < 50.0 {
        LimitStyleKind::Success
    } else if used_percent < 80.0 {
        LimitStyleKind::Warning
    } else if used_percent <= 90.0 {
        LimitStyleKind::Caution
    } else if used_percent <= 95.0 {
        LimitStyleKind::Error
    } else {
        LimitStyleKind::Critical
    }
}

fn best_label(identity: &AuthIdentity) -> String {
    identity
        .email
        .clone()
        .or_else(|| identity.name.clone())
        .unwrap_or_else(|| "-".into())
}

fn shorten_id(value: &str) -> String {
    if value.len() <= 16 {
        return value.to_string();
    }

    format!("{}…{}", &value[..10], &value[value.len() - 4..])
}

fn is_same_user(left: &AuthIdentity, right: &AuthIdentity) -> bool {
    matches!(
        (&left.subject, &right.subject),
        (Some(left_subject), Some(right_subject)) if left_subject == right_subject
    ) || matches!(
        (&left.user_id, &right.user_id),
        (Some(left_user_id), Some(right_user_id)) if left_user_id == right_user_id
    )
}

fn shares_account(left: &AuthIdentity, right: &AuthIdentity) -> bool {
    if is_same_user(left, right) {
        return false;
    }

    matches!(
        (&left.chatgpt_account_id, &right.chatgpt_account_id),
        (Some(left_account), Some(right_account)) if left_account == right_account
    )
}

fn usage_matches_identity(usage: &ProfileUsageSnapshot, identity: &AuthIdentity) -> bool {
    if let (Some(usage_user_id), Some(identity_user_id)) =
        (usage.user_id.as_deref(), identity.user_id.as_deref())
    {
        if usage_user_id != identity_user_id {
            return false;
        }
    }

    if let (Some(usage_account_id), Some(identity_account_id)) = (
        usage.account_id.as_deref(),
        identity.chatgpt_account_id.as_deref(),
    ) {
        if usage
            .user_id
            .as_deref()
            .is_some_and(|usage_user_id| usage_account_id == usage_user_id)
        {
            return true;
        }

        if usage_account_id != identity_account_id {
            return false;
        }
    }

    if usage.user_id.is_none() && usage.account_id.is_none() {
        if let (Some(usage_email), Some(identity_email)) =
            (usage.email.as_deref(), identity.email.as_deref())
        {
            if usage_email != identity_email {
                return false;
            }
        }
    }

    true
}

fn format_reset_timestamp(
    dt: chrono::DateTime<Local>,
    captured_at: chrono::DateTime<Local>,
) -> String {
    let time = dt.format("%-I:%M %p").to_string();
    if dt.date_naive() == captured_at.date_naive() {
        time
    } else {
        format!("{time} on {}", dt.format("%-d %b"))
    }
}

fn format_reset_timestamp_compact(
    dt: chrono::DateTime<Local>,
    captured_at: chrono::DateTime<Local>,
    kind: UsageWindowKind,
) -> String {
    let time = dt.format("%-I:%M %p").to_string();

    match kind {
        UsageWindowKind::Primary => time,
        UsageWindowKind::Secondary if dt.date_naive() == captured_at.date_naive() => time,
        UsageWindowKind::Secondary => format!("{} {time}", dt.format("%a")),
    }
}

fn title_case(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }

    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let rest = chars.as_str().to_ascii_lowercase();
    first.to_uppercase().collect::<String>() + &rest
}

#[cfg(test)]
mod tests {
    use super::{
        build_profile_rows, conflicting_profiles, delete_profile_home, parse_auth_identity,
        promote_launch_auth_if_unchanged, read_auth_snapshot, read_stored_auth, save_profile_auth,
        sync_launch_codex_home, sync_profile_codex_home, write_auth_raw_if_unchanged, AuthIdentity,
        LimitStyleKind, ProfileAuthRefresher, ProfileStyleKind, ProfileUsageLoader,
        ProfileUsageSnapshot, ProfileUsageState, SaveProfileOutcome, SavedProfile, StoredAuth,
        UsageFetchResult, UsageWindowSnapshot,
    };
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use chrono::{Local, TimeZone};
    use serde_json::json;
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::path::Path;
    use tempfile::tempdir;
    use wiremock::{
        matchers::{header, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[test]
    fn parses_auth_identity_from_id_token() {
        let auth = auth_json(
            "google-oauth2|1234",
            "user-1234",
            "acct-1234",
            Some("praveen@example.com"),
            Some("Praveen Perera"),
            Some("google"),
        );

        let identity = parse_auth_identity(&auth).unwrap();

        assert_eq!(identity.subject.as_deref(), Some("google-oauth2|1234"));
        assert_eq!(identity.user_id.as_deref(), Some("user-1234"));
        assert_eq!(identity.chatgpt_account_id.as_deref(), Some("acct-1234"));
        assert_eq!(identity.email.as_deref(), Some("praveen@example.com"));
        assert_eq!(identity.name.as_deref(), Some("Praveen Perera"));
        assert_eq!(identity.auth_provider.as_deref(), Some("google"));
    }

    #[test]
    fn finds_conflicts_for_matching_user_identity() {
        let requested = identity("sub-1", "user-1", "acct-1", Some("a@example.com"));
        let profiles = vec![
            saved_profile(
                "a",
                identity("sub-1", "user-1", "acct-1", Some("a@example.com")),
                ProfileUsageState::Unchecked,
            ),
            saved_profile(
                "b",
                identity("sub-2", "user-2", "acct-1", Some("b@example.com")),
                ProfileUsageState::Unchecked,
            ),
        ];

        let conflicts = conflicting_profiles(&profiles, "new", &requested);

        assert_eq!(conflicts, vec!["a".to_string()]);
    }

    #[test]
    fn save_profile_auth_skips_when_replacement_is_declined() {
        let dir = tempdir().unwrap();
        let profiles_dir = dir.path().join("profiles");
        let old_dir = profiles_dir.join("old");
        let auth_path = dir.path().join("auth.json");

        fs::create_dir_all(&old_dir).unwrap();
        fs::write(
            old_dir.join("auth.json"),
            auth_json("sub-1", "user-1", "acct-1", None, None, None),
        )
        .unwrap();
        fs::write(
            &auth_path,
            auth_json("sub-1", "user-1", "acct-1", None, None, None),
        )
        .unwrap();

        let outcome = save_profile_auth(
            "new",
            &auth_path,
            &profiles_dir,
            &["old".to_string()],
            false,
        )
        .unwrap();

        assert_eq!(outcome, SaveProfileOutcome::SkippedConflict);
        assert!(old_dir.exists());
        assert!(!profiles_dir.join("new").exists());
    }

    #[test]
    fn save_profile_auth_replaces_conflicting_profiles() {
        let dir = tempdir().unwrap();
        let profiles_dir = dir.path().join("profiles");
        let old_dir = profiles_dir.join("old");
        let auth_path = dir.path().join("auth.json");
        let new_auth = auth_json(
            "sub-1",
            "user-1",
            "acct-1",
            Some("new@example.com"),
            None,
            None,
        );

        fs::create_dir_all(&old_dir).unwrap();
        fs::write(
            old_dir.join("auth.json"),
            auth_json("sub-1", "user-1", "acct-1", None, None, None),
        )
        .unwrap();
        fs::write(&auth_path, &new_auth).unwrap();

        let outcome =
            save_profile_auth("new", &auth_path, &profiles_dir, &["old".to_string()], true)
                .unwrap();

        assert_eq!(outcome, SaveProfileOutcome::Saved);
        assert!(!old_dir.exists());
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
    fn format_reset_timestamp_compact_uses_weekday_for_future_secondary_window() {
        let captured_at = Local.with_ymd_and_hms(2026, 3, 31, 9, 15, 0).unwrap();
        let reset_at = Local.with_ymd_and_hms(2026, 4, 2, 0, 30, 0).unwrap();

        let formatted = super::format_reset_timestamp_compact(
            reset_at,
            captured_at,
            super::UsageWindowKind::Secondary,
        );

        assert_eq!(formatted, "Thu 12:30 AM");
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
    fn format_compact_percent_right_aligns_numeric_part() {
        assert_eq!(super::format_compact_percent("0%"), "  0%");
        assert_eq!(super::format_compact_percent("42%"), " 42%");
        assert_eq!(super::format_compact_percent("100%"), "100%");
    }

    #[test]
    fn usage_window_style_uses_expected_bands() {
        assert_eq!(super::limit_style(49.0), LimitStyleKind::Success);
        assert_eq!(super::limit_style(50.0), LimitStyleKind::Warning);
        assert_eq!(super::limit_style(79.0), LimitStyleKind::Warning);
        assert_eq!(super::limit_style(80.0), LimitStyleKind::Caution);
        assert_eq!(super::limit_style(90.0), LimitStyleKind::Caution);
        assert_eq!(super::limit_style(91.0), LimitStyleKind::Error);
        assert_eq!(super::limit_style(95.0), LimitStyleKind::Error);
        assert_eq!(super::limit_style(96.0), LimitStyleKind::Critical);
        assert_eq!(
            super::usage_window_style(
                &ProfileUsageState::Unchecked,
                super::UsageWindowKind::Primary
            ),
            LimitStyleKind::Normal
        );
    }

    #[test]
    fn five_hour_style_turns_bold_red_when_weekly_displays_hundred() {
        let usage =
            ProfileUsageState::Available(usage_snapshot("plus", "user-1", "acct-1", 42.0, 100.0));

        assert_eq!(
            super::five_hour_limit_style(&usage),
            LimitStyleKind::Critical
        );
        assert_eq!(
            super::usage_window_style(&usage, super::UsageWindowKind::Secondary),
            LimitStyleKind::Critical
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
            LimitStyleKind::Warning
        );
        assert_eq!(row_style(&rows, "a"), ProfileStyleKind::Error);
        assert_eq!(row_style(&rows, "c"), ProfileStyleKind::Normal);
    }

    #[test]
    fn list_rows_force_five_hour_bold_red_when_weekly_is_exhausted() {
        let active = identity("sub-1", "user-1", "acct-1", Some("praveen@example.com"));
        let profiles = vec![saved_profile(
            "a",
            active.clone(),
            ProfileUsageState::Available(usage_snapshot("plus", "user-1", "acct-1", 42.0, 100.0)),
        )];

        let rows = build_profile_rows(&profiles, Some(&active));

        assert_eq!(
            row_limit_style(&rows, "a", |row| row.five_hour_style),
            LimitStyleKind::Critical
        );
        assert_eq!(
            row_limit_style(&rows, "a", |row| row.weekly_style),
            LimitStyleKind::Critical
        );
    }

    #[test]
    fn colorize_limit_cell_uses_limit_style_when_row_has_no_error() {
        colored::control::set_override(true);

        let row = super::ProfileRow {
            profile: "a".into(),
            label: "-".into(),
            provider: "-".into(),
            user: "-".into(),
            account: "-".into(),
            plan: "-".into(),
            five_hour: "96%".into(),
            five_hour_reset: "-".into(),
            five_hour_compact: "96%".into(),
            five_hour_style: LimitStyleKind::Critical,
            weekly: "82%".into(),
            weekly_reset: "-".into(),
            weekly_compact: "82%".into(),
            weekly_style: LimitStyleKind::Caution,
            status: Default::default(),
        };

        let critical = super::colorize_limit_cell("96%", 3, row.five_hour_style, &row);
        let caution = super::colorize_limit_cell("82%", 3, row.weekly_style, &row);

        assert!(critical.contains("\u{1b}[1;31m96%\u{1b}[0m"));
        assert!(caution.contains("\u{1b}[38;2;255;165;0m82%\u{1b}[0m"));

        colored::control::unset_override();
    }

    #[test]
    fn colorize_limit_cell_keeps_whole_row_error_precedence() {
        colored::control::set_override(true);

        let row = super::ProfileRow {
            profile: "a".into(),
            label: "-".into(),
            provider: "-".into(),
            user: "-".into(),
            account: "-".into(),
            plan: "-".into(),
            five_hour: "42%".into(),
            five_hour_reset: "-".into(),
            five_hour_compact: "42%".into(),
            five_hour_style: LimitStyleKind::Success,
            weekly: "73%".into(),
            weekly_reset: "-".into(),
            weekly_compact: "73%".into(),
            weekly_style: LimitStyleKind::Warning,
            status: super::ProfileStatus {
                items: vec![super::ProfileStatusItem::SameUser(vec!["b".into()])],
            },
        };

        let rendered = super::colorize_limit_cell("42%", 3, row.five_hour_style, &row);

        assert!(rendered.contains("\u{1b}[1;31m42%\u{1b}[0m"));

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
        let auth = read_auth("sub-1", "user-1", "acct-1", "old-access", "old-refresh");

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
        let auth = read_auth("sub-1", "user-1", "acct-1", "valid-access", "old-refresh");

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
    fn sync_profile_codex_home_links_shared_entries_without_touching_auth() {
        let dir = tempdir().unwrap();
        let global_codex = dir.path().join(".codex");
        let profile_home = global_codex.join("profiles").join("a");

        fs::create_dir_all(global_codex.join("profiles")).unwrap();
        fs::create_dir_all(global_codex.join("skills")).unwrap();
        fs::write(global_codex.join("config.toml"), "model = \"gpt-5.4\"").unwrap();
        fs::write(global_codex.join("AGENTS.md"), "shared").unwrap();
        fs::create_dir_all(&profile_home).unwrap();
        fs::write(profile_home.join("auth.json"), "local-auth").unwrap();

        sync_profile_codex_home(&profile_home, &global_codex).unwrap();

        assert_eq!(
            fs::read_to_string(profile_home.join("auth.json")).unwrap(),
            "local-auth"
        );
        assert_eq!(
            fs::read_link(profile_home.join("config.toml")).unwrap(),
            global_codex.join("config.toml")
        );
        assert_eq!(
            fs::read_link(profile_home.join("skills")).unwrap(),
            global_codex.join("skills")
        );
        assert_eq!(
            fs::read_link(profile_home.join("AGENTS.md")).unwrap(),
            global_codex.join("AGENTS.md")
        );
        assert!(!profile_home.join("profiles").exists());
    }

    #[test]
    fn sync_profile_codex_home_replaces_stale_targets() {
        let dir = tempdir().unwrap();
        let global_codex = dir.path().join(".codex");
        let profile_home = global_codex.join("profiles").join("a");

        fs::create_dir_all(global_codex.join("profiles")).unwrap();
        fs::create_dir_all(global_codex.join("skills")).unwrap();
        fs::write(global_codex.join("config.toml"), "model = \"gpt-5.4\"").unwrap();
        fs::create_dir_all(&profile_home).unwrap();
        fs::write(profile_home.join("config.toml"), "stale").unwrap();
        fs::create_dir_all(profile_home.join("skills")).unwrap();
        symlink(
            global_codex.join("config.toml"),
            profile_home.join("AGENTS.md"),
        )
        .unwrap();
        fs::write(profile_home.join("auth.json"), "local-auth").unwrap();

        sync_profile_codex_home(&profile_home, &global_codex).unwrap();

        assert_eq!(
            fs::read_link(profile_home.join("config.toml")).unwrap(),
            global_codex.join("config.toml")
        );
        assert_eq!(
            fs::read_link(profile_home.join("skills")).unwrap(),
            global_codex.join("skills")
        );
        assert_eq!(
            fs::read_to_string(profile_home.join("auth.json")).unwrap(),
            "local-auth"
        );
    }

    #[test]
    fn sync_launch_codex_home_copies_auth_and_links_shared_entries() {
        let dir = tempdir().unwrap();
        let global_codex = dir.path().join(".codex");
        let launch_home = dir.path().join("launch");
        let profile_auth = global_codex.join("profiles").join("a").join("auth.json");

        fs::create_dir_all(global_codex.join("profiles").join("a")).unwrap();
        fs::create_dir_all(global_codex.join("skills")).unwrap();
        fs::write(global_codex.join("config.toml"), "model = \"gpt-5.4\"").unwrap();
        fs::write(&profile_auth, "profile-auth").unwrap();

        sync_launch_codex_home(&launch_home, &global_codex, &profile_auth).unwrap();

        assert_eq!(
            fs::read_to_string(launch_home.join("auth.json")).unwrap(),
            "profile-auth"
        );
        assert_eq!(
            fs::read_link(launch_home.join("config.toml")).unwrap(),
            global_codex.join("config.toml")
        );
        assert_eq!(
            fs::read_link(launch_home.join("skills")).unwrap(),
            global_codex.join("skills")
        );
        assert!(!launch_home.join("profiles").exists());
    }

    #[test]
    fn promote_launch_auth_if_unchanged_updates_profile_auth() {
        let dir = tempdir().unwrap();
        let profile_auth = dir.path().join("profile-auth.json");
        let launch_auth_path = dir.path().join("launch-auth.json");
        let original_auth = auth_json(
            "sub-1",
            "user-1",
            "acct-1",
            Some("old@example.com"),
            None,
            Some("google"),
        );
        let refreshed_auth = auth_json_with_tokens(
            "sub-1",
            "user-1",
            "acct-1",
            Some("new@example.com"),
            None,
            Some("google"),
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
        let original_auth = auth_json(
            "sub-1",
            "user-1",
            "acct-1",
            Some("old@example.com"),
            None,
            Some("google"),
        );
        let competing_auth = auth_json_with_tokens(
            "sub-1",
            "user-1",
            "acct-1",
            Some("other@example.com"),
            None,
            Some("google"),
            "other-access",
            "other-refresh",
        );
        let refreshed_auth = auth_json_with_tokens(
            "sub-1",
            "user-1",
            "acct-1",
            Some("new@example.com"),
            None,
            Some("google"),
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
        let original_auth = auth_json(
            "sub-1",
            "user-1",
            "acct-1",
            Some("old@example.com"),
            None,
            Some("google"),
        );
        let switched_account_auth = auth_json_with_tokens(
            "sub-1",
            "user-1",
            "acct-2",
            Some("new@example.com"),
            None,
            Some("google"),
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
        let auth = read_auth("sub-1", "user-1", "acct-1", "old-access", "old-refresh");

        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id_token": jwt("sub-1", "user-1", "acct-1", Some("new@example.com"), None, Some("google")),
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
        let auth = read_auth("sub-1", "user-1", "acct-1", "old-access", "old-refresh");

        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id_token": jwt("sub-2", "user-2", "acct-2", Some("other@example.com"), None, Some("google")),
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

    fn saved_profile(name: &str, identity: AuthIdentity, usage: ProfileUsageState) -> SavedProfile {
        SavedProfile {
            name: name.into(),
            auth_path: Path::new("/tmp").join(name).join("auth.json"),
            identity: Some(identity),
            invalid_auth: false,
            usage,
        }
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

    fn auth_json(
        subject: &str,
        user_id: &str,
        account_id: &str,
        email: Option<&str>,
        name: Option<&str>,
        auth_provider: Option<&str>,
    ) -> String {
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&json!({
                "sub": subject,
                "email": email,
                "name": name,
                "auth_provider": auth_provider,
                "https://api.openai.com/auth": {
                    "user_id": user_id,
                    "chatgpt_account_id": account_id,
                }
            }))
            .unwrap(),
        );

        serde_json::to_string(&json!({
            "OPENAI_API_KEY": null,
            "auth_mode": "chatgpt",
            "last_refresh": "2026-03-30T00:00:00Z",
            "tokens": {
                "access_token": "access-token",
                "account_id": account_id,
                "id_token": format!("{header}.{payload}.sig"),
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
            }),
            secondary: Some(UsageWindowSnapshot {
                used_percent: secondary_used_percent,
                reset_at: Some(now + 7200),
            }),
        }
    }

    fn read_auth(
        subject: &str,
        user_id: &str,
        account_id: &str,
        access_token: &str,
        refresh_token: &str,
    ) -> StoredAuth {
        let dir = tempdir().unwrap();
        let auth_path = dir.path().join("auth.json");
        fs::write(
            &auth_path,
            auth_json_with_tokens(
                subject,
                user_id,
                account_id,
                Some("old@example.com"),
                None,
                Some("google"),
                access_token,
                refresh_token,
            ),
        )
        .unwrap();
        read_stored_auth(&auth_path).unwrap()
    }

    fn auth_json_with_tokens(
        subject: &str,
        user_id: &str,
        account_id: &str,
        email: Option<&str>,
        name: Option<&str>,
        auth_provider: Option<&str>,
        access_token: &str,
        refresh_token: &str,
    ) -> String {
        serde_json::to_string(&json!({
            "OPENAI_API_KEY": null,
            "auth_mode": "chatgpt",
            "last_refresh": "2026-03-30T00:00:00Z",
            "tokens": {
                "access_token": access_token,
                "account_id": account_id,
                "id_token": jwt(subject, user_id, account_id, email, name, auth_provider),
                "refresh_token": refresh_token,
            }
        }))
        .unwrap()
    }

    fn jwt(
        subject: &str,
        user_id: &str,
        account_id: &str,
        email: Option<&str>,
        name: Option<&str>,
        auth_provider: Option<&str>,
    ) -> String {
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&json!({
                "sub": subject,
                "email": email,
                "name": name,
                "auth_provider": auth_provider,
                "https://api.openai.com/auth": {
                    "user_id": user_id,
                    "chatgpt_account_id": account_id,
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
