use eyre::{eyre, Result};
use serde::Deserialize;

const USER_AGENT: &str = "cmd-crate-versions/1.0 (github.com/praveenperera/dotfiles)";
const API_BASE: &str = "https://crates.io/api/v1/crates";

#[derive(Debug, Deserialize)]
struct CrateResponse {
    #[serde(rename = "crate")]
    crate_info: CrateInfo,
    versions: Vec<Version>,
}

#[derive(Debug, Deserialize)]
struct CrateInfo {
    max_stable_version: Option<String>,
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
        let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;

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

        select_version(&crate_data, pre)
            .ok_or_else(|| eyre!("no non-yanked versions found for '{}'", crate_name))
    }
}

fn select_version(crate_data: &CrateResponse, pre: bool) -> Option<String> {
    if !pre {
        if let Some(version) = crate_data.crate_info.max_stable_version.as_ref() {
            return Some(version.clone());
        }
    }

    crate_data
        .versions
        .iter()
        .find(|v| !v.yanked && (pre || !v.num.contains('-')))
        .map(|v| v.num.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(max_stable_version: Option<&str>, versions: &[(&str, bool)]) -> CrateResponse {
        CrateResponse {
            crate_info: CrateInfo {
                max_stable_version: max_stable_version.map(str::to_string),
            },
            versions: versions
                .iter()
                .map(|(num, yanked)| Version {
                    num: (*num).to_string(),
                    yanked: *yanked,
                })
                .collect(),
        }
    }

    #[test]
    fn uses_max_stable_version_for_default_selection() {
        let crate_data = response(
            Some("3.0.0"),
            &[("2.4.0", false), ("3.0.0", false), ("3.0.0-rc.2", false)],
        );

        assert_eq!(select_version(&crate_data, false).as_deref(), Some("3.0.0"));
    }

    #[test]
    fn falls_back_to_first_non_yanked_stable_version() {
        let crate_data = response(
            None,
            &[("3.0.0-rc.2", false), ("2.4.0", true), ("2.3.0", false)],
        );

        assert_eq!(select_version(&crate_data, false).as_deref(), Some("2.3.0"));
    }

    #[test]
    fn pre_selection_uses_first_non_yanked_version() {
        let crate_data = response(Some("3.0.0"), &[("3.1.0-alpha.1", false), ("3.0.0", false)]);

        assert_eq!(
            select_version(&crate_data, true).as_deref(),
            Some("3.1.0-alpha.1")
        );
    }
}
