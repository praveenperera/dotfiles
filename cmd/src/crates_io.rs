use eyre::{eyre, Result};
use serde::Deserialize;

const USER_AGENT: &str = "cmd-crate-versions/1.0 (github.com/praveenperera/dotfiles)";
const API_BASE: &str = "https://crates.io/api/v1/crates";

#[derive(Debug, Deserialize)]
struct CrateResponse {
    versions: Vec<Version>,
}

#[derive(Debug, Deserialize)]
struct Version {
    num: String,
    yanked: bool,
}

pub struct CratesIoClient {
    client: reqwest::Client,
}

impl CratesIoClient {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .build()?;

        Ok(Self { client })
    }

    /// Fetches the latest non-yanked version for a crate
    pub async fn get_latest_version(&self, crate_name: &str, pre: bool) -> Result<String> {
        let url = format!("{API_BASE}/{crate_name}");

        let response = self
            .client
            .get(&url)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| {
                if e.status() == Some(reqwest::StatusCode::NOT_FOUND) {
                    eyre!("crate '{}' not found on crates.io", crate_name)
                } else {
                    eyre!("failed to fetch crate '{}': {}", crate_name, e)
                }
            })?;

        let crate_data: CrateResponse = response.json().await?;

        crate_data
            .versions
            .iter()
            .find(|v| !v.yanked && (pre || !v.num.contains('-')))
            .map(|v| v.num.clone())
            .ok_or_else(|| eyre!("no non-yanked versions found for '{}'", crate_name))
    }
}
