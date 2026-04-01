use super::*;
use crate::runtime;

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
        let http = HttpClient::builder().timeout(USAGE_FETCH_TIMEOUT).build()?;
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
