use clap::Parser;
use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use xshell::Shell;

#[derive(Parser, Debug)]
#[command(name = "pr-context")]
#[command(about = "Fetches PR comments and their code references from GitHub")]
#[command(visible_alias = "prc")]
pub struct Args {
    /// GitHub PR URL or repository in format "owner/repo"
    pub repo_or_url: String,

    /// Pull request number (optional if URL is provided)
    pub pr_number: Option<u64>,

    /// GitHub token (optional, for higher rate limits)
    #[arg(short, long, env = "GITHUB_TOKEN")]
    pub token: Option<String>,

    /// Only include comments with code references
    #[arg(short = 'c', long)]
    pub code_only: bool,

    /// Compact output (only author, body, and code_reference)
    #[arg(short = 'C', long)]
    pub compact: bool,
}

// data structure for GitHub API PR review comment response
#[derive(Debug, Serialize, Deserialize)]
struct ReviewComment {
    id: u64,
    body: String,
    user: User,
    created_at: String,
    updated_at: String,
    path: Option<String>,
    position: Option<u64>,
    original_position: Option<u64>,
    commit_id: Option<String>,
    original_commit_id: Option<String>,
    diff_hunk: Option<String>,
    line: Option<u64>,
    original_line: Option<u64>,
    start_line: Option<u64>,
    original_start_line: Option<u64>,
    start_side: Option<String>,
    side: Option<String>,
}

// data structure for GitHub API issue comment response (general PR comments)
#[derive(Debug, Serialize, Deserialize)]
struct IssueComment {
    id: u64,
    body: String,
    user: User,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct User {
    login: String,
    id: u64,
}

// our unified comment structure with code context
#[derive(Debug, Serialize)]
struct CommentWithContext {
    comment_id: u64,
    comment_type: String,
    author: String,
    body: String,
    created_at: String,
    updated_at: String,
    code_reference: Option<CodeReference>,
}

#[derive(Debug, Serialize)]
struct CodeReference {
    file_path: String,
    diff_hunk: String,
    line: Option<u64>,
    start_line: Option<u64>,
    side: Option<String>,
    commit_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct PrContext {
    repo: String,
    pr_number: u64,
    comments: Vec<CommentWithContext>,
}

// compact version with only author, body, and code_reference
#[derive(Debug, Serialize)]
struct CompactComment {
    author: String,
    body: String,
    code_reference: Option<CodeReference>,
}

#[derive(Debug, Serialize)]
struct CompactPrContext {
    repo: String,
    pr_number: u64,
    comments: Vec<CompactComment>,
}

pub fn run(_sh: &Shell, args: &[OsString]) -> Result<()> {
    let args = Args::parse_from(args);
    run_with_args(_sh, args)
}

pub fn run_with_args(_sh: &Shell, args: Args) -> Result<()> {
    // parse the input to extract owner, repo, and pr_number
    let (owner, repo, pr_number) = parse_input(&args.repo_or_url, args.pr_number)?;

    let runtime = tokio::runtime::Runtime::new()?;
    let mut pr_context = runtime.block_on(fetch_pr_context(&owner, &repo, pr_number, &args.token))?;

    // filter to only comments with code references if requested
    if args.code_only {
        pr_context.comments.retain(|c| c.code_reference.is_some());
    }

    // output compact or full format
    if args.compact {
        let compact_context = CompactPrContext {
            repo: pr_context.repo,
            pr_number: pr_context.pr_number,
            comments: pr_context
                .comments
                .into_iter()
                .map(|c| CompactComment {
                    author: c.author,
                    body: c.body,
                    code_reference: c.code_reference,
                })
                .collect(),
        };
        let json = serde_json::to_string_pretty(&compact_context)?;
        println!("{}", json);
    } else {
        let json = serde_json::to_string_pretty(&pr_context)?;
        println!("{}", json);
    }

    Ok(())
}

async fn fetch_pr_context(
    owner: &str,
    repo: &str,
    pr_number: u64,
    token: &Option<String>,
) -> Result<PrContext> {
    let client = reqwest::Client::builder()
        .user_agent("pr-context-cli")
        .build()?;

    // fetch review comments (comments on code)
    let review_comments = fetch_review_comments(&client, owner, repo, pr_number, token)
        .await
        .context("Failed to fetch review comments")?;

    // fetch issue comments (general comments on the PR)
    let issue_comments = fetch_issue_comments(&client, owner, repo, pr_number, token)
        .await
        .context("Failed to fetch issue comments")?;

    let mut comments = Vec::new();

    // convert review comments to our unified structure
    for rc in review_comments {
        let code_reference = if rc.path.is_some() {
            Some(CodeReference {
                file_path: rc.path.unwrap_or_default(),
                diff_hunk: rc.diff_hunk.unwrap_or_default(),
                line: rc.line,
                start_line: rc.start_line,
                side: rc.side,
                commit_id: rc.commit_id,
            })
        } else {
            None
        };

        comments.push(CommentWithContext {
            comment_id: rc.id,
            comment_type: "review".to_string(),
            author: rc.user.login,
            body: rc.body,
            created_at: rc.created_at,
            updated_at: rc.updated_at,
            code_reference,
        });
    }

    // convert issue comments to our unified structure
    for ic in issue_comments {
        comments.push(CommentWithContext {
            comment_id: ic.id,
            comment_type: "issue".to_string(),
            author: ic.user.login,
            body: ic.body,
            created_at: ic.created_at,
            updated_at: ic.updated_at,
            code_reference: None,
        });
    }

    Ok(PrContext {
        repo: format!("{}/{}", owner, repo),
        pr_number,
        comments,
    })
}

// parse input to extract owner, repo, and pr number
// supports both "owner/repo" + pr_number and full GitHub URLs
fn parse_input(input: &str, pr_number: Option<u64>) -> Result<(String, String, u64)> {
    // check if input is a URL
    if input.starts_with("http://") || input.starts_with("https://") {
        return parse_github_url(input);
    }

    // otherwise treat as owner/repo format
    let parts: Vec<&str> = input.split('/').collect();
    if parts.len() != 2 {
        eyre::bail!("Repository must be in format 'owner/repo' or a valid GitHub PR URL");
    }

    let pr_num = pr_number.ok_or_else(|| {
        eyre::eyre!("PR number is required when using 'owner/repo' format")
    })?;

    Ok((parts[0].to_string(), parts[1].to_string(), pr_num))
}

// parse GitHub PR URL to extract owner, repo, and pr number
// supports formats like:
// - https://github.com/owner/repo/pull/123
// - https://github.com/owner/repo/pull/123/files
fn parse_github_url(url: &str) -> Result<(String, String, u64)> {
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
        eyre::bail!("Invalid GitHub PR URL format. Expected: https://github.com/owner/repo/pull/123");
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

async fn fetch_review_comments(
    client: &reqwest::Client,
    owner: &str,
    repo: &str,
    pr_number: u64,
    token: &Option<String>,
) -> Result<Vec<ReviewComment>> {
    let mut all_comments = Vec::new();
    let mut url = Some(format!(
        "https://api.github.com/repos/{}/{}/pulls/{}/comments?per_page=100",
        owner, repo, pr_number
    ));

    while let Some(current_url) = url {
        let mut request = client.get(&current_url);

        if let Some(token) = token {
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

async fn fetch_issue_comments(
    client: &reqwest::Client,
    owner: &str,
    repo: &str,
    pr_number: u64,
    token: &Option<String>,
) -> Result<Vec<IssueComment>> {
    let mut all_comments = Vec::new();
    let mut url = Some(format!(
        "https://api.github.com/repos/{}/{}/issues/{}/comments?per_page=100",
        owner, repo, pr_number
    ));

    while let Some(current_url) = url {
        let mut request = client.get(&current_url);

        if let Some(token) = token {
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
