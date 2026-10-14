//! Claude Code subscription usage
//!
//! Lives beside the Codex usage code because it maps Claude's limits onto the same snapshot
//! types, so pace, run rates, and history work the same way for both providers

use super::ops::CurrentUsageReport;
use super::*;
use crate::{fsutil, runtime};
use sha2::{Digest, Sha256};

const CLAUDE_USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const CLAUDE_OAUTH_BETA: &str = "oauth-2025-04-20";
const CLAUDE_KEYCHAIN_SERVICE: &str = "Claude Code-credentials";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeCredentialsFile {
    claude_ai_oauth: Option<ClaudeOauthCredentials>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeOauthCredentials {
    access_token: String,
    // milliseconds since the epoch
    expires_at: Option<i64>,
    subscription_type: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeGlobalConfig {
    oauth_account: Option<ClaudeOauthAccount>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeOauthAccount {
    account_uuid: Option<String>,
    organization_uuid: Option<String>,
    email_address: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudeUsageResponse {
    five_hour: Option<ClaudeUsageWindow>,
    seven_day: Option<ClaudeUsageWindow>,
}

#[derive(Debug, Deserialize)]
struct ClaudeUsageWindow {
    utilization: Option<f64>,
    resets_at: Option<chrono::DateTime<Utc>>,
}

impl ClaudeUsageWindow {
    fn into_snapshot(self) -> Option<UsageWindowSnapshot> {
        Some(UsageWindowSnapshot {
            used_percent: self.utilization?,
            // resets land a few hundred ms before the hour, so round to show 7:00 PM, not 6:59 PM
            reset_at: self
                .resets_at
                .map(|reset_at| (reset_at.timestamp_millis() + 500).div_euclid(1000)),
            // claude plans have no codex-style multiplier and the codex plan overrides use
            // overlapping names such as "pro", so keep claude unweighted
            limit_multiplier: 1.0,
        })
    }
}

pub(super) fn claude_usage_report(
    captured_at: chrono::DateTime<Utc>,
) -> Result<CurrentUsageReport> {
    let paths = ClaudePaths::from_env()?;
    let credentials = paths.read_credentials()?;
    if credentials
        .expires_at
        .is_some_and(|expires_at| expires_at <= captured_at.timestamp_millis())
    {
        return Err(eyre!("Claude Code auth expired. Run claude to refresh it"));
    }

    let account = paths.read_account();
    let response = runtime::block_on(fetch_claude_usage(&credentials.access_token))??;
    let label = account.email_address.clone().unwrap_or_else(|| "-".into());
    let usage = ProfileUsageState::Available(ProfileUsageSnapshot {
        user_id: account.account_uuid,
        account_id: account.organization_uuid,
        email: account.email_address,
        plan_type: credentials.subscription_type,
        five_hour: response
            .five_hour
            .and_then(ClaudeUsageWindow::into_snapshot),
        weekly: response
            .seven_day
            .and_then(ClaudeUsageWindow::into_snapshot),
    });

    Ok(CurrentUsageReport::new(
        label,
        usage,
        captured_at,
        usage_history_cache_path_for(CLAUDE_USAGE_HISTORY_FILE).ok(),
    ))
}

async fn fetch_claude_usage(access_token: &str) -> Result<ClaudeUsageResponse> {
    let http = HttpClient::builder().timeout(USAGE_FETCH_TIMEOUT).build()?;
    let response = http
        .get(CLAUDE_USAGE_URL)
        .bearer_auth(access_token)
        .header("anthropic-beta", CLAUDE_OAUTH_BETA)
        .send()
        .await
        .wrap_err("Failed to fetch Claude usage")?;

    let status = response.status();
    if status == StatusCode::UNAUTHORIZED {
        return Err(eyre!("Claude Code auth rejected. Run claude to refresh it"));
    }
    if !status.is_success() {
        return Err(eyre!("Claude usage request failed: {status}"));
    }

    response
        .json::<ClaudeUsageResponse>()
        .await
        .wrap_err("Failed to parse Claude usage")
}

struct ClaudePaths {
    credentials: PathBuf,
    global_config: PathBuf,
    keychain_service: Option<String>,
}

impl ClaudePaths {
    fn from_env() -> Result<Self> {
        match std::env::var_os("CLAUDE_CONFIG_DIR").filter(|dir| !dir.is_empty()) {
            Some(dir) => {
                // claude code suffixes the keychain entry with a hash of a custom config dir
                let suffix = hex::encode(Sha256::digest(dir.as_encoded_bytes()));
                let dir = PathBuf::from(dir);
                Ok(Self {
                    credentials: dir.join(".credentials.json"),
                    global_config: dir.join(".claude.json"),
                    keychain_service: macos_keychain_service(format!(
                        "{CLAUDE_KEYCHAIN_SERVICE}-{}",
                        &suffix[..8]
                    )),
                })
            }
            None => {
                let home = fsutil::home_dir()?;
                Ok(Self {
                    credentials: home.join(".claude").join(".credentials.json"),
                    global_config: home.join(".claude.json"),
                    keychain_service: macos_keychain_service(CLAUDE_KEYCHAIN_SERVICE.into()),
                })
            }
        }
    }

    fn read_credentials(&self) -> Result<ClaudeOauthCredentials> {
        let file = stdfs::read(&self.credentials)
            .ok()
            .and_then(|raw| parse_claude_credentials(&raw));
        let keychain = self
            .keychain_service
            .as_deref()
            .and_then(keychain_claude_credentials);

        // claude code on macOS keeps its live token in the keychain and can leave a stale copy
        // in the file, so take whichever expires last
        [keychain, file]
            .into_iter()
            .flatten()
            .max_by_key(|credentials| credentials.expires_at.unwrap_or(i64::MIN))
            .ok_or_else(|| eyre!("No Claude Code auth found. Run claude and log in"))
    }

    fn read_account(&self) -> ClaudeOauthAccount {
        stdfs::read(&self.global_config)
            .ok()
            .and_then(|raw| serde_json::from_slice::<ClaudeGlobalConfig>(&raw).ok())
            .and_then(|config| config.oauth_account)
            .unwrap_or_default()
    }
}

fn parse_claude_credentials(raw: &[u8]) -> Option<ClaudeOauthCredentials> {
    serde_json::from_slice::<ClaudeCredentialsFile>(raw)
        .ok()?
        .claude_ai_oauth
}

fn macos_keychain_service(service: String) -> Option<String> {
    cfg!(target_os = "macos").then_some(service)
}

fn keychain_claude_credentials(service: &str) -> Option<ClaudeOauthCredentials> {
    let output = std::process::Command::new("security")
        .args(["find-generic-password", "-s", service, "-w"])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    parse_claude_credentials(&output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_usage_response_maps_five_hour_and_weekly_windows() {
        let response = serde_json::from_value::<ClaudeUsageResponse>(serde_json::json!({
            "five_hour": {
                "utilization": 20.0,
                "resets_at": "2026-10-14T23:59:59.696741+00:00"
            },
            "seven_day": {
                "utilization": 21.5,
                "resets_at": "2026-10-17T04:59:59.696760+00:00"
            },
            "seven_day_opus": null
        }))
        .unwrap();

        let five_hour = response
            .five_hour
            .and_then(ClaudeUsageWindow::into_snapshot)
            .unwrap();
        let weekly = response
            .seven_day
            .and_then(ClaudeUsageWindow::into_snapshot)
            .unwrap();

        assert_eq!(five_hour.used_percent, 20.0);
        assert_eq!(
            five_hour.reset_at,
            Some(
                Utc.with_ymd_and_hms(2026, 10, 15, 0, 0, 0)
                    .unwrap()
                    .timestamp()
            )
        );
        assert_eq!(weekly.used_percent, 21.5);
        assert_eq!(
            weekly.reset_at,
            Some(
                Utc.with_ymd_and_hms(2026, 10, 17, 5, 0, 0)
                    .unwrap()
                    .timestamp()
            )
        );
    }

    #[test]
    fn claude_usage_window_without_utilization_is_skipped() {
        let window = serde_json::from_value::<ClaudeUsageWindow>(serde_json::json!({
            "utilization": null,
            "resets_at": null
        }))
        .unwrap();

        assert!(window.into_snapshot().is_none());
    }
}
