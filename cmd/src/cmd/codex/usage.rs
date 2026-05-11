use super::*;
use crate::fsutil;
use crate::runtime;
use std::collections::{BTreeMap, HashSet};
use std::str::FromStr;

const DOUBLED_PLAN_TYPES: &[KnownPlanType] = &[KnownPlanType::Prolite];
pub(super) const USAGE_HISTORY_RETENTION_DAYS: i64 = 14;
const USAGE_HISTORY_DISPLAY_HOURS: i64 = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KnownPlanType {
    Plus,
    Team,
    Prolite,
    Pro,
}

impl KnownPlanType {
    fn default_limit_config(self) -> PlanLimitConfig {
        let multiplier = match self {
            Self::Plus | Self::Team => 1.0,
            Self::Prolite => 5.0,
            Self::Pro => 20.0,
        };

        PlanLimitConfig {
            multiplier,
            is_doubled: DOUBLED_PLAN_TYPES.contains(&self),
        }
    }
}

impl FromStr for KnownPlanType {
    type Err = ();

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "plus" => Ok(Self::Plus),
            "team" => Ok(Self::Team),
            "prolite" => Ok(Self::Prolite),
            "pro" => Ok(Self::Pro),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PlanLimitConfig {
    multiplier: f64,
    is_doubled: bool,
}

impl PlanLimitConfig {
    fn effective_multiplier(self) -> f64 {
        let doubled_multiplier = if self.is_doubled { 2.0 } else { 1.0 };
        self.multiplier * doubled_multiplier
    }
}

#[derive(Debug, Default, Deserialize)]
struct CmdConfigFile {
    #[serde(default)]
    cmd: CmdConfigSection,
}

#[derive(Debug, Default, Deserialize)]
struct CmdConfigSection {
    #[serde(default)]
    codex_usage: CodexUsageConfig,
}

#[derive(Debug, Default, Deserialize)]
struct CodexUsageConfig {
    #[serde(default)]
    plans: BTreeMap<String, PlanLimitOverride>,
}

#[derive(Debug, Default, Deserialize)]
struct PlanLimitOverride {
    multiplier: Option<f64>,
    is_doubled: Option<bool>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub(super) struct UsageHistory {
    samples: Vec<UsageHistorySample>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct UsageHistorySample {
    captured_at: chrono::DateTime<Utc>,
    label: String,
    user_id: Option<String>,
    account_id: Option<String>,
    email: Option<String>,
    plan_type: Option<String>,
    primary: Option<UsageHistoryWindow>,
    secondary: Option<UsageHistoryWindow>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct UsageHistoryWindow {
    used_percent: f64,
    reset_at: Option<i64>,
    limit_multiplier: f64,
}

fn effective_limit_multiplier(plan_type: Option<&str>) -> f64 {
    let Some(plan_type) = plan_type else {
        return 1.0;
    };
    let plan_type = plan_type.to_ascii_lowercase();
    let mut limit_config = default_limit_config(&plan_type);

    let config_path = codex_dir().ok().map(|dir| dir.join("config.toml"));
    if let Some(config_path) = config_path {
        if let Ok(raw) = stdfs::read_to_string(config_path) {
            if let Ok(config) = toml::from_str::<CmdConfigFile>(&raw) {
                apply_plan_limit_override(
                    &mut limit_config,
                    config.cmd.codex_usage.plans.get(&plan_type),
                );
            }
        }
    }

    limit_config.effective_multiplier()
}

fn default_limit_config(plan_type: &str) -> PlanLimitConfig {
    KnownPlanType::from_str(plan_type)
        .map(KnownPlanType::default_limit_config)
        .unwrap_or(PlanLimitConfig {
            multiplier: 1.0,
            is_doubled: false,
        })
}

fn apply_plan_limit_override(
    limit_config: &mut PlanLimitConfig,
    override_config: Option<&PlanLimitOverride>,
) {
    if let Some(override_config) = override_config {
        if let Some(multiplier) = override_config.multiplier {
            limit_config.multiplier = multiplier;
        }
        if let Some(is_doubled) = override_config.is_doubled {
            limit_config.is_doubled = is_doubled;
        }
    }
}

pub(super) fn usage_history_cache_path() -> Result<PathBuf> {
    Ok(usage_history_cache_path_from_env(
        std::env::var_os("XDG_CACHE_HOME"),
        fsutil::home_dir()?,
    ))
}

fn usage_history_cache_path_from_env(
    xdg_cache_home: Option<std::ffi::OsString>,
    home: PathBuf,
) -> PathBuf {
    let base = xdg_cache_home
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or(home.join(".cache"));

    base.join("cmd").join("codex-usage-history.json")
}

pub(super) fn load_usage_history(path: &Path) -> UsageHistory {
    stdfs::read(path)
        .ok()
        .and_then(|raw| serde_json::from_slice::<UsageHistory>(&raw).ok())
        .unwrap_or_default()
}

pub(super) fn save_usage_history(path: &Path, history: &UsageHistory) -> Result<()> {
    fsutil::ensure_parent_dir(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| eyre!("usage history path has no parent"))?;
    let temp = tempfile::NamedTempFile::new_in(parent)?;
    stdfs::write(temp.path(), serde_json::to_vec_pretty(history)?)?;
    temp.persist(path)?;

    Ok(())
}

pub(super) fn record_usage_sample(path: &Path, sample: UsageHistorySample) -> Result<()> {
    let mut history = load_usage_history(path);
    push_usage_sample(&mut history, sample);
    save_usage_history(path, &history)
}

fn push_usage_sample(history: &mut UsageHistory, sample: UsageHistorySample) {
    history.samples.push(sample);
}

pub(super) fn prune_usage_history(history: &mut UsageHistory, cutoff: chrono::DateTime<Utc>) {
    history
        .samples
        .retain(|sample| sample.captured_at >= cutoff);
}

pub(super) fn spawn_usage_history_prune() {
    let Ok(path) = usage_history_cache_path() else {
        return;
    };

    std::thread::spawn(move || {
        let _ = prune_usage_history_file(&path, Utc::now());
    });
}

fn prune_usage_history_file(path: &Path, now: chrono::DateTime<Utc>) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let mut history = load_usage_history(path);
    let original_len = history.samples.len();
    prune_usage_history(
        &mut history,
        now - chrono::Duration::days(USAGE_HISTORY_RETENTION_DAYS),
    );

    if history.samples.len() != original_len {
        save_usage_history(path, &history)?;
    }

    Ok(())
}

pub(super) fn usage_history_sample(
    label: &str,
    usage: &ProfileUsageState,
    captured_at: chrono::DateTime<Utc>,
) -> Option<UsageHistorySample> {
    let ProfileUsageState::Available(snapshot) = usage else {
        return None;
    };

    Some(UsageHistorySample {
        captured_at,
        label: label.into(),
        user_id: snapshot.user_id.clone(),
        account_id: snapshot.account_id.clone(),
        email: snapshot.email.clone(),
        plan_type: snapshot.plan_type.clone(),
        primary: snapshot.primary.as_ref().map(UsageHistoryWindow::from),
        secondary: snapshot.secondary.as_ref().map(UsageHistoryWindow::from),
    })
}

pub(super) fn usage_run_rates(
    history: &UsageHistory,
    current: &UsageHistorySample,
) -> UsageRunRates {
    let Some(previous) = history
        .samples
        .iter()
        .rev()
        .find(|sample| usage_samples_match(sample, current))
    else {
        return UsageRunRates::default();
    };

    UsageRunRates {
        primary: window_run_rate(
            previous.primary.as_ref(),
            current.primary.as_ref(),
            previous,
            current,
        ),
        secondary: window_run_rate(
            previous.secondary.as_ref(),
            current.secondary.as_ref(),
            previous,
            current,
        ),
    }
}

fn usage_samples_match(previous: &UsageHistorySample, current: &UsageHistorySample) -> bool {
    if let (Some(previous), Some(current)) = (&previous.account_id, &current.account_id) {
        return previous == current;
    }

    if let (Some(previous), Some(current)) = (&previous.user_id, &current.user_id) {
        return previous == current;
    }

    if let (Some(previous), Some(current)) = (&previous.email, &current.email) {
        return previous.eq_ignore_ascii_case(current);
    }

    false
}

fn window_run_rate(
    previous_window: Option<&UsageHistoryWindow>,
    current_window: Option<&UsageHistoryWindow>,
    previous: &UsageHistorySample,
    current: &UsageHistorySample,
) -> Option<f64> {
    let previous_window = previous_window?;
    let current_window = current_window?;
    if previous_window.reset_at != current_window.reset_at {
        return None;
    }

    let elapsed_hours = current
        .captured_at
        .signed_duration_since(previous.captured_at)
        .num_seconds() as f64
        / 3600.0;
    if elapsed_hours <= 0.0 {
        return None;
    }

    Some((current_window.used_percent - previous_window.used_percent) / elapsed_hours)
}

impl From<&UsageWindowSnapshot> for UsageHistoryWindow {
    fn from(window: &UsageWindowSnapshot) -> Self {
        Self {
            used_percent: window.used_percent,
            reset_at: window.reset_at,
            limit_multiplier: window.limit_multiplier,
        }
    }
}

pub(super) fn print_usage_history(
    writer: &mut impl Write,
    history: &UsageHistory,
    now: chrono::DateTime<Utc>,
) -> io::Result<()> {
    let rows = usage_history_rows(history, now);
    if rows.is_empty() {
        writeln!(writer, "No usage history from the last 24 hours")?;
        return Ok(());
    }

    let widths = UsageHistoryWidths {
        captured_at: "TIME".len().max(
            rows.iter()
                .map(|row| row.captured_at.len())
                .max()
                .unwrap_or_default(),
        ),
        label: "EMAIL".len().max(
            rows.iter()
                .map(|row| row.label.len())
                .max()
                .unwrap_or_default(),
        ),
        primary: "5 HOUR LIMIT".len().max(
            rows.iter()
                .map(|row| row.primary.len())
                .max()
                .unwrap_or_default(),
        ),
        secondary: "WEEKLY LIMIT".len().max(
            rows.iter()
                .map(|row| row.secondary.len())
                .max()
                .unwrap_or_default(),
        ),
    };

    writeln!(
        writer,
        "{}   {}   {}   {}",
        format!("{:<width$}", "TIME", width = widths.captured_at)
            .blue()
            .bold(),
        format!("{:<width$}", "EMAIL", width = widths.label)
            .blue()
            .bold(),
        format!("{:<width$}", "5 HOUR LIMIT", width = widths.primary)
            .blue()
            .bold(),
        format!("{:<width$}", "WEEKLY LIMIT", width = widths.secondary)
            .blue()
            .bold(),
    )?;

    for row in rows {
        writeln!(
            writer,
            "{:<captured_at_width$}   {:<label_width$}   {:<primary_width$}   {}",
            row.captured_at,
            row.label,
            row.primary,
            row.secondary,
            captured_at_width = widths.captured_at,
            label_width = widths.label,
            primary_width = widths.primary,
        )?;
    }

    Ok(())
}

fn usage_history_rows(history: &UsageHistory, now: chrono::DateTime<Utc>) -> Vec<UsageHistoryRow> {
    let cutoff = now - chrono::Duration::hours(USAGE_HISTORY_DISPLAY_HOURS);
    let mut seen = HashSet::new();
    let mut samples = history
        .samples
        .iter()
        .filter(|sample| sample.captured_at >= cutoff)
        .collect::<Vec<_>>();
    samples.sort_by_key(|sample| sample.captured_at);

    samples
        .into_iter()
        .filter(|sample| seen.insert(sample.dedupe_key()))
        .map(UsageHistoryRow::from)
        .collect()
}

#[derive(Debug)]
struct UsageHistoryWidths {
    captured_at: usize,
    label: usize,
    primary: usize,
    secondary: usize,
}

#[derive(Debug, PartialEq, Eq)]
struct UsageHistoryRow {
    captured_at: String,
    label: String,
    primary: String,
    secondary: String,
}

impl From<&UsageHistorySample> for UsageHistoryRow {
    fn from(sample: &UsageHistorySample) -> Self {
        Self {
            captured_at: format_history_timestamp(sample.captured_at.with_timezone(&Local)),
            label: sample.email.clone().unwrap_or_else(|| sample.label.clone()),
            primary: sample
                .primary
                .as_ref()
                .map(format_history_window)
                .unwrap_or_else(|| "-".into()),
            secondary: sample
                .secondary
                .as_ref()
                .map(format_history_window)
                .unwrap_or_else(|| "-".into()),
        }
    }
}

fn format_history_timestamp(captured_at: chrono::DateTime<Local>) -> String {
    captured_at.format("%a %-I:%M %p").to_string()
}

fn format_history_window(window: &UsageHistoryWindow) -> String {
    match window.reset_at.and_then(|reset_at| {
        Local
            .timestamp_opt(reset_at, 0)
            .single()
            .map(|reset_at| reset_at.format("%a %-I:%M %p").to_string())
    }) {
        Some(reset_at) => format!("{:>3}% ({reset_at})", window.used_percent.round() as i64),
        None => format!("{:>3}%", window.used_percent.round() as i64),
    }
}

impl UsageHistorySample {
    fn dedupe_key(&self) -> String {
        format!(
            "{}\n{}\n{}\n{}\n{}",
            self.account_id
                .as_deref()
                .or(self.user_id.as_deref())
                .or(self.email.as_deref())
                .unwrap_or(&self.label),
            self.plan_type.as_deref().unwrap_or_default(),
            self.primary
                .as_ref()
                .map(UsageHistoryWindow::dedupe_key)
                .unwrap_or_default(),
            self.secondary
                .as_ref()
                .map(UsageHistoryWindow::dedupe_key)
                .unwrap_or_default(),
            self.label,
        )
    }
}

impl UsageHistoryWindow {
    fn dedupe_key(&self) -> String {
        format!(
            "{:.3}:{}:{:.3}",
            self.used_percent,
            self.reset_at
                .map(|reset_at| reset_at.to_string())
                .unwrap_or_default(),
            self.limit_multiplier,
        )
    }
}

pub(super) fn needs_proactive_refresh(
    auth: &StoredAuth,
    now: chrono::DateTime<Utc>,
) -> Result<bool> {
    let access_token = auth
        .tokens
        .as_ref()
        .and_then(|tokens| tokens.access_token.as_deref())
        .filter(|token| !token.is_empty());
    if let Some(access_token) = access_token {
        if let Ok(Some(expires_at)) = parse_jwt_expiration(access_token) {
            return Ok(expires_at <= now);
        }
    }

    let Some(last_refresh) = auth.last_refresh.as_deref() else {
        return Ok(true);
    };
    let last_refresh = chrono::DateTime::parse_from_rfc3339(last_refresh)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .wrap_err("last_refresh is not valid RFC3339")?;
    Ok(last_refresh < now - chrono::Duration::days(PROFILE_REFRESH_FALLBACK_DAYS))
}

pub(super) fn parse_jwt_expiration(token: &str) -> Result<Option<chrono::DateTime<Utc>>> {
    let claims: StandardJwtClaims = serde_json::from_slice(&jwt_payload(token)?)?;
    Ok(claims
        .exp
        .and_then(|exp| Utc.timestamp_opt(exp, 0).single()))
}

impl ProfileUsageLoader {
    pub(super) fn new() -> Result<Self> {
        Self::with_timeout(USAGE_FETCH_TIMEOUT)
    }

    pub(super) fn with_timeout(timeout: Duration) -> Result<Self> {
        let http = HttpClient::builder().timeout(timeout).build()?;
        Ok(Self {
            http,
            usage_url: CHATGPT_USAGE_URL.into(),
        })
    }

    #[cfg(test)]
    pub(super) fn with_urls(usage_url: impl Into<String>) -> Result<Self> {
        let http = HttpClient::builder().timeout(USAGE_FETCH_TIMEOUT).build()?;
        Ok(Self {
            http,
            usage_url: usage_url.into(),
        })
    }

    pub(super) async fn load_updates(&self, profiles: &[SavedProfile]) -> Vec<ProfileUsageUpdate> {
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

    pub(super) async fn fetch_profile_usage(
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
        let limit_multiplier = effective_limit_multiplier(payload.plan_type.as_deref());
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
                        limit_multiplier,
                    })
            }),
            secondary: payload.rate_limit.as_ref().and_then(|rate_limit| {
                rate_limit
                    .secondary_window
                    .as_ref()
                    .map(|window| UsageWindowSnapshot {
                        used_percent: window.used_percent,
                        reset_at: Some(window.reset_at),
                        limit_multiplier,
                    })
            }),
        })
    }
}

impl ProfileAuthRefresher {
    pub(super) fn new() -> Result<Self> {
        let http = HttpClient::builder().timeout(USAGE_FETCH_TIMEOUT).build()?;
        Ok(Self {
            http,
            refresh_url: CHATGPT_REFRESH_URL.into(),
        })
    }

    #[cfg(test)]
    pub(super) fn with_url(refresh_url: impl Into<String>) -> Result<Self> {
        let http = HttpClient::builder().timeout(USAGE_FETCH_TIMEOUT).build()?;
        Ok(Self {
            http,
            refresh_url: refresh_url.into(),
        })
    }

    pub(super) async fn refresh_profile_auth(
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
            if !is_same_user(expected_identity, &refreshed_identity)
                || match (
                    expected_identity.chatgpt_account_id.as_deref(),
                    refreshed_identity.chatgpt_account_id.as_deref(),
                ) {
                    (Some(expected), Some(actual)) => expected != actual,
                    _ => false,
                }
            {
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

pub(super) fn enrich_profile_usage(profiles: &mut [SavedProfile]) -> Result<()> {
    let loader = ProfileUsageLoader::new()?;
    let updates = runtime::block_on(loader.load_updates(profiles))?;

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

pub(super) fn usage_matches_identity(
    usage: &ProfileUsageSnapshot,
    identity: &AuthIdentity,
) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_limit_config_matches_current_codex_limits() {
        assert_eq!(default_limit_config("plus").effective_multiplier(), 1.0);
        assert_eq!(default_limit_config("team").effective_multiplier(), 1.0);
        assert_eq!(default_limit_config("prolite").effective_multiplier(), 10.0);
        assert_eq!(default_limit_config("pro").effective_multiplier(), 20.0);
    }

    #[test]
    fn plan_limit_overrides_can_disable_doubling() {
        let mut limit_config = default_limit_config("prolite");
        let overrides = toml::from_str::<CmdConfigFile>(
            r#"
            [cmd.codex_usage.plans.prolite]
            is_doubled = false
            "#,
        )
        .unwrap();

        apply_plan_limit_override(
            &mut limit_config,
            overrides.cmd.codex_usage.plans.get("prolite"),
        );

        assert_eq!(limit_config.effective_multiplier(), 5.0);
    }

    #[test]
    fn plan_limit_overrides_can_define_new_plan_weights() {
        let mut limit_config = default_limit_config("enterprise");
        let overrides = toml::from_str::<CmdConfigFile>(
            r#"
            [cmd.codex_usage.plans.enterprise]
            multiplier = 50
            is_doubled = true
            "#,
        )
        .unwrap();

        apply_plan_limit_override(
            &mut limit_config,
            overrides.cmd.codex_usage.plans.get("enterprise"),
        );

        assert_eq!(limit_config.effective_multiplier(), 100.0);
    }

    #[test]
    fn known_plan_type_parses_supported_values() {
        assert_eq!(KnownPlanType::from_str("plus"), Ok(KnownPlanType::Plus));
        assert_eq!(KnownPlanType::from_str("team"), Ok(KnownPlanType::Team));
        assert_eq!(
            KnownPlanType::from_str("prolite"),
            Ok(KnownPlanType::Prolite)
        );
        assert_eq!(KnownPlanType::from_str("pro"), Ok(KnownPlanType::Pro));
        assert!(KnownPlanType::from_str("enterprise").is_err());
    }

    #[test]
    fn usage_history_cache_path_uses_xdg_cache_home() {
        let path = usage_history_cache_path_from_env(
            Some(std::ffi::OsString::from("/tmp/cache")),
            PathBuf::from("/home/praveen"),
        );

        assert_eq!(
            path,
            PathBuf::from("/tmp/cache/cmd/codex-usage-history.json")
        );
    }

    #[test]
    fn usage_history_cache_path_falls_back_to_home_cache() {
        let path = usage_history_cache_path_from_env(None, PathBuf::from("/home/praveen"));

        assert_eq!(
            path,
            PathBuf::from("/home/praveen/.cache/cmd/codex-usage-history.json")
        );
    }

    #[test]
    fn usage_history_prunes_samples_older_than_two_weeks() {
        let now = Utc.with_ymd_and_hms(2026, 5, 8, 12, 0, 0).unwrap();
        let mut history = UsageHistory::default();

        push_usage_sample(
            &mut history,
            sample_at(now - chrono::Duration::days(15), "acct-1", 1.0),
        );
        push_usage_sample(
            &mut history,
            sample_at(now - chrono::Duration::days(14), "acct-1", 2.0),
        );
        push_usage_sample(&mut history, sample_at(now, "acct-1", 3.0));
        prune_usage_history(
            &mut history,
            now - chrono::Duration::days(USAGE_HISTORY_RETENTION_DAYS),
        );

        assert_eq!(history.samples.len(), 2);
        assert_eq!(
            history.samples[0].primary.as_ref().unwrap().used_percent,
            2.0
        );
        assert_eq!(
            history.samples[1].primary.as_ref().unwrap().used_percent,
            3.0
        );
    }

    #[test]
    fn usage_history_rows_show_last_24_hours_without_duplicates() {
        let now = Utc.with_ymd_and_hms(2026, 5, 8, 12, 0, 0).unwrap();
        let duplicate = sample_at(now - chrono::Duration::hours(2), "acct-1", 10.0);
        let changed = sample_at(now - chrono::Duration::hours(1), "acct-1", 12.0);
        let history = UsageHistory {
            samples: vec![
                sample_at(now - chrono::Duration::hours(25), "acct-1", 8.0),
                duplicate.clone(),
                duplicate,
                changed,
            ],
        };

        let rows = usage_history_rows(&history, now);

        assert_eq!(rows.len(), 2);
        assert!(rows[0].primary.starts_with(" 10% ("));
        assert!(rows[1].primary.starts_with(" 12% ("));
    }

    #[test]
    fn usage_run_rate_uses_previous_matching_sample() {
        let now = Utc.timestamp_opt(1_000, 0).single().unwrap();
        let previous = sample_at(now, "acct-1", 10.0);
        let current = sample_at(now + chrono::Duration::minutes(30), "acct-1", 13.0);
        let history = UsageHistory {
            samples: vec![previous],
        };

        let rates = usage_run_rates(&history, &current);

        assert_eq!(rates.primary, Some(6.0));
    }

    #[test]
    fn usage_run_rate_skips_different_identity() {
        let now = Utc.timestamp_opt(1_000, 0).single().unwrap();
        let previous = sample_at(now, "acct-1", 10.0);
        let current = sample_at(now + chrono::Duration::minutes(30), "acct-2", 13.0);
        let history = UsageHistory {
            samples: vec![previous],
        };

        let rates = usage_run_rates(&history, &current);

        assert_eq!(rates.primary, None);
    }

    #[test]
    fn usage_run_rate_skips_changed_reset_window() {
        let now = Utc.timestamp_opt(1_000, 0).single().unwrap();
        let previous = sample_at(now, "acct-1", 10.0);
        let mut current = sample_at(now + chrono::Duration::minutes(30), "acct-1", 13.0);
        current.primary.as_mut().unwrap().reset_at = Some(10_000);
        let history = UsageHistory {
            samples: vec![previous],
        };

        let rates = usage_run_rates(&history, &current);

        assert_eq!(rates.primary, None);
    }

    #[test]
    fn usage_run_rate_skips_non_positive_elapsed_time() {
        let now = Utc.timestamp_opt(1_000, 0).single().unwrap();
        let previous = sample_at(now, "acct-1", 10.0);
        let current = sample_at(now, "acct-1", 13.0);
        let history = UsageHistory {
            samples: vec![previous],
        };

        let rates = usage_run_rates(&history, &current);

        assert_eq!(rates.primary, None);
    }

    fn sample_at(
        captured_at: chrono::DateTime<Utc>,
        account_id: &str,
        used_percent: f64,
    ) -> UsageHistorySample {
        UsageHistorySample {
            captured_at,
            label: "praveen@example.com".into(),
            user_id: Some("user-1".into()),
            account_id: Some(account_id.into()),
            email: Some("praveen@example.com".into()),
            plan_type: Some("plus".into()),
            primary: Some(UsageHistoryWindow {
                used_percent,
                reset_at: Some(5_000),
                limit_multiplier: 1.0,
            }),
            secondary: None,
        }
    }
}
