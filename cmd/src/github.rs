use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};

pub struct Github {
    client: reqwest::Client,
    token: Option<String>,
}

impl Github {
    const BASE_URL: &'static str = "https://api.github.com";

    pub fn new(token: Option<String>) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("pr-context-cli")
            .build()?;

        Ok(Self { client, token })
    }

    pub async fn fetch_review_comments(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<Vec<ReviewComment>> {
        let mut all_comments = Vec::new();
        let mut url = Some(format!(
            "{}/repos/{}/{}/pulls/{}/comments?per_page=100",
            Self::BASE_URL,
            owner,
            repo,
            pr_number
        ));

        while let Some(current_url) = url {
            let mut request = self.client.get(&current_url);

            if let Some(token) = &self.token {
                request = request.header("Authorization", format!("Bearer {}", token));
            }

            let response = request.send().await?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await?;
                eyre::bail!("GitHub API request failed with status {}: {}", status, body);
            }

            // extract next page URL from Link header
            url = parse_next_link(response.headers());

            let comments: Vec<ReviewComment> = response.json().await?;
            all_comments.extend(comments);
        }

        Ok(all_comments)
    }

    pub async fn fetch_issue_comments(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<Vec<IssueComment>> {
        let mut all_comments = Vec::new();
        let mut url = Some(format!(
            "{}/repos/{}/{}/issues/{}/comments?per_page=100",
            Self::BASE_URL,
            owner,
            repo,
            pr_number
        ));

        while let Some(current_url) = url {
            let mut request = self.client.get(&current_url);

            if let Some(token) = &self.token {
                request = request.header("Authorization", format!("Bearer {}", token));
            }

            let response = request.send().await?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await?;
                eyre::bail!("GitHub API request failed with status {}: {}", status, body);
            }

            // extract next page URL from Link header
            url = parse_next_link(response.headers());

            let comments: Vec<IssueComment> = response.json().await?;
            all_comments.extend(comments);
        }

        Ok(all_comments)
    }
}

// data structure for GitHub API PR review comment response
#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewComment {
    pub id: u64,
    pub body: String,
    pub user: User,
    pub created_at: String,
    pub updated_at: String,
    pub path: Option<String>,
    pub position: Option<u64>,
    pub original_position: Option<u64>,
    pub commit_id: Option<String>,
    pub original_commit_id: Option<String>,
    pub diff_hunk: Option<String>,
    pub line: Option<u64>,
    pub original_line: Option<u64>,
    pub start_line: Option<u64>,
    pub original_start_line: Option<u64>,
    pub start_side: Option<String>,
    pub side: Option<String>,
}

// data structure for GitHub API issue comment response (general PR comments)
#[derive(Debug, Serialize, Deserialize)]
pub struct IssueComment {
    pub id: u64,
    pub body: String,
    pub user: User,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub login: String,
    pub id: u64,
}

// parse GitHub PR URL to extract owner, repo, and pr number
// supports formats like:
// - https://github.com/owner/repo/pull/123
// - https://github.com/owner/repo/pull/123/files
pub fn parse_url(url: &str) -> Result<(String, String, u64)> {
    let url = url.trim_end_matches('/');

    // remove protocol
    let url = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    // remove github.com prefix
    let url = url
        .strip_prefix("github.com/")
        .ok_or_else(|| eyre::eyre!("URL must be a GitHub URL"))?;

    // split remaining path
    let parts: Vec<&str> = url.split('/').collect();

    // need at least: owner/repo/pull/number
    if parts.len() < 4 {
        eyre::bail!(
            "Invalid GitHub PR URL format. Expected: https://github.com/owner/repo/pull/123"
        );
    }

    if parts[2] != "pull" {
        eyre::bail!("URL must be a pull request URL (contain '/pull/')");
    }

    let owner = parts[0].to_string();
    let repo = parts[1].to_string();
    let pr_number = parts[3]
        .parse::<u64>()
        .context("Failed to parse PR number from URL")?;

    Ok((owner, repo, pr_number))
}

// parse the Link header to extract the next page URL
// github returns Link header in format:
// <https://api.github.com/...?page=2>; rel="next", <https://api.github.com/...?page=3>; rel="last"
fn parse_next_link(headers: &reqwest::header::HeaderMap) -> Option<String> {
    let link_header = headers.get(reqwest::header::LINK)?.to_str().ok()?;

    // split by comma to get individual links
    for link_part in link_header.split(',') {
        let link_part = link_part.trim();

        // check if this is the "next" relation
        if link_part.contains("rel=\"next\"") {
            // extract URL between < and >
            if let Some(start) = link_part.find('<') {
                if let Some(end) = link_part.find('>') {
                    return Some(link_part[start + 1..end].to_string());
                }
            }
        }
    }

    None
}
