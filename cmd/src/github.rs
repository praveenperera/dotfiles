use color_eyre::eyre::{Context, Result};
use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use std::process::Command;

pub struct Github {
    client: reqwest::Client,
    token: Option<String>,
    base_url: String,
}

impl Github {
    const BASE_URL: &'static str = "https://api.github.com";

    pub fn new(token: Option<String>) -> Result<Self> {
        Self::with_base_url(Self::BASE_URL, resolve_token(token))
    }

    fn with_base_url(base_url: impl Into<String>, token: Option<String>) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("pr-context-cli")
            .build()?;

        Ok(Self {
            client,
            token: token.and_then(non_empty_token),
            base_url: base_url.into().trim_end_matches('/').to_string(),
        })
    }

    pub async fn fetch_review_comments(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<Vec<ReviewComment>> {
        let mut all_comments = Vec::new();
        let mut url = Some(self.url(&format!(
            "/repos/{owner}/{repo}/pulls/{pr_number}/comments?per_page=100"
        )));

        while let Some(current_url) = url {
            let response = self
                .send(&current_url, "failed to fetch PR review comments")
                .await?;

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
        let mut url = Some(self.url(&format!(
            "/repos/{owner}/{repo}/issues/{pr_number}/comments?per_page=100"
        )));

        while let Some(current_url) = url {
            let response = self
                .send(&current_url, "failed to fetch PR issue comments")
                .await?;

            // extract next page URL from Link header
            url = parse_next_link(response.headers());

            let comments: Vec<IssueComment> = response.json().await?;
            all_comments.extend(comments);
        }

        Ok(all_comments)
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    fn request(&self, url: &str) -> RequestBuilder {
        let request = self.client.get(url);

        if let Some(token) = &self.token {
            request.header("Authorization", format!("Bearer {token}"))
        } else {
            request
        }
    }

    async fn send(&self, url: &str, context: &str) -> Result<reqwest::Response> {
        let response = self.request(url).send().await?;

        if response.status().is_success() {
            return Ok(response);
        }

        let status = response.status();
        let body = response.text().await?;

        if matches!(
            status,
            reqwest::StatusCode::UNAUTHORIZED
                | reqwest::StatusCode::FORBIDDEN
                | reqwest::StatusCode::NOT_FOUND
        ) {
            eyre::bail!(
                "{context}: GitHub API request failed with status {status}: {body}\nIf this is a private repository, authenticate with --token, GITHUB_TOKEN, GH_TOKEN, or `gh auth login` with repo read access"
            );
        }

        eyre::bail!("{context}: GitHub API request failed with status {status}: {body}");
    }
}

pub fn resolve_token(explicit_token: Option<String>) -> Option<String> {
    resolve_token_with(
        explicit_token,
        |name| std::env::var(name).ok(),
        gh_auth_token,
    )
}

fn resolve_token_with(
    explicit_token: Option<String>,
    env_token: impl Fn(&str) -> Option<String>,
    gh_token: impl Fn() -> Option<String>,
) -> Option<String> {
    explicit_token
        .and_then(non_empty_token)
        .or_else(|| env_token("GITHUB_TOKEN").and_then(non_empty_token))
        .or_else(|| env_token("GH_TOKEN").and_then(non_empty_token))
        .or_else(|| gh_token().and_then(non_empty_token))
}

fn gh_auth_token() -> Option<String> {
    let output = Command::new("gh")
        .args(["auth", "token", "--hostname", "github.com"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8(output.stdout)
        .ok()
        .and_then(non_empty_token)
}

fn non_empty_token(token: String) -> Option<String> {
    let token = token.trim().to_string();
    if token.is_empty() {
        None
    } else {
        Some(token)
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

#[cfg(test)]
mod tests {
    use super::{resolve_token_with, Github};
    use serde_json::json;
    use wiremock::{
        matchers::{header, method, path, query_param},
        Mock, MockServer, ResponseTemplate,
    };

    #[test]
    fn token_resolver_prefers_explicit_token() {
        let token = resolve_token_with(
            Some(" explicit ".to_string()),
            |_| Some("env-token".to_string()),
            || Some("gh-token".to_string()),
        );

        assert_eq!(token.as_deref(), Some("explicit"));
    }

    #[test]
    fn token_resolver_uses_env_before_gh() {
        let token = resolve_token_with(
            None,
            |name| match name {
                "GITHUB_TOKEN" => Some(" github ".to_string()),
                "GH_TOKEN" => Some(" gh-env ".to_string()),
                _ => None,
            },
            || Some("gh-cli".to_string()),
        );

        assert_eq!(token.as_deref(), Some("github"));
    }

    #[test]
    fn token_resolver_skips_empty_tokens_and_uses_gh_token_env() {
        let token = resolve_token_with(
            Some(" ".to_string()),
            |name| match name {
                "GITHUB_TOKEN" => Some(" ".to_string()),
                "GH_TOKEN" => Some(" gh-env ".to_string()),
                _ => None,
            },
            || Some("gh-cli".to_string()),
        );

        assert_eq!(token.as_deref(), Some("gh-env"));
    }

    #[test]
    fn token_resolver_falls_back_to_gh_auth_token() {
        let token = resolve_token_with(None, |_| None, || Some(" gh-cli ".to_string()));

        assert_eq!(token.as_deref(), Some("gh-cli"));
    }

    #[tokio::test]
    async fn sends_auth_header_for_review_and_issue_comments() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/7/comments"))
            .and(query_param("per_page", "100"))
            .and(header("authorization", "Bearer token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/issues/7/comments"))
            .and(query_param("per_page", "100"))
            .and(header("authorization", "Bearer token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(&server)
            .await;

        let github = Github::with_base_url(server.uri(), Some("token".to_string())).unwrap();

        github
            .fetch_review_comments("owner", "repo", 7)
            .await
            .unwrap();
        github
            .fetch_issue_comments("owner", "repo", 7)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn auth_like_failures_explain_private_repo_auth() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/private/pulls/7/comments"))
            .and(query_param("per_page", "100"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({
                "message": "Not Found"
            })))
            .mount(&server)
            .await;

        let github = Github::with_base_url(server.uri(), None).unwrap();
        let error = github
            .fetch_review_comments("owner", "private", 7)
            .await
            .unwrap_err()
            .to_string();

        assert!(error.contains("private repository"));
        assert!(error.contains("GITHUB_TOKEN"));
        assert!(error.contains("GH_TOKEN"));
        assert!(error.contains("gh auth login"));
    }
}
