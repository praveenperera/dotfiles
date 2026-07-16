use clap::{Parser, ValueEnum};
use color_eyre::eyre::{Context, Result};
use serde::Serialize;
use std::ffi::OsString;
use xshell::Shell;

use crate::{github, runtime};

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Markdown,
    Json,
}

#[derive(Parser, Debug)]
#[command(name = "pr-context")]
#[command(about = "Fetches thread-aware PR review context from GitHub")]
#[command(visible_alias = "prc")]
pub struct Args {
    /// GitHub PR URL, repository in format "owner/repo", or just PR number (auto-detects repo from git remote)
    pub repo_or_url: String,

    /// Pull request number (optional if URL or PR number is provided as first argument)
    pub pr_number: Option<u64>,

    /// GitHub token (defaults to GITHUB_TOKEN, GH_TOKEN, or gh auth token)
    #[arg(short, long)]
    pub token: Option<String>,

    /// Only include code review threads
    #[arg(short = 'c', long)]
    pub code_only: bool,

    /// Only include unresolved review threads
    #[arg(short = 'u', long)]
    pub unresolved_only: bool,

    /// Omit nonessential IDs, timestamps, and diff hunks
    #[arg(short = 'C', long)]
    pub compact: bool,

    /// Output format
    #[arg(short = 'f', long, default_value = "markdown")]
    pub format: OutputFormat,
}

#[derive(Debug, Serialize)]
struct PrContext {
    pull_request: PullRequest,
    conversation_comments: Vec<Comment>,
    reviews: Vec<Review>,
    review_threads: Vec<ReviewThread>,
}

impl PrContext {
    fn apply_filters(&mut self, code_only: bool, unresolved_only: bool) {
        if code_only {
            self.conversation_comments.clear();
            self.reviews.clear();
        }
        if unresolved_only {
            self.review_threads
                .retain(|thread| matches!(thread.resolution, ThreadResolution::Unresolved));
        }
    }
}

#[derive(Debug, Serialize)]
struct PullRequest {
    repository: String,
    number: u64,
    url: String,
    title: String,
    state: PullRequestState,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum PullRequestState {
    Open,
    Closed,
    Merged,
}

#[derive(Debug, Serialize)]
struct Comment {
    id: String,
    author: Option<String>,
    body: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct Review {
    id: String,
    state: ReviewState,
    author: Option<String>,
    body: String,
    submitted_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum ReviewState {
    Pending,
    Commented,
    Approved,
    ChangesRequested,
    Dismissed,
}

#[derive(Debug, Serialize)]
struct ReviewThread {
    id: String,
    resolution: ThreadResolution,
    is_outdated: bool,
    location: ThreadLocation,
    comments: Vec<ReviewThreadComment>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum ThreadResolution {
    Unresolved,
    Resolved { resolved_by: Option<String> },
}

#[derive(Debug, Serialize)]
#[serde(tag = "subject", rename_all = "snake_case")]
enum ThreadLocation {
    File {
        path: String,
    },
    Line {
        path: String,
        current: Option<DiffRange>,
        original: Option<OriginalRange>,
    },
}

#[derive(Debug, Serialize)]
struct DiffRange {
    start: Option<DiffPosition>,
    end: DiffPosition,
}

#[derive(Debug, Serialize)]
struct DiffPosition {
    line: u64,
    side: DiffSide,
}

#[derive(Debug, Serialize)]
struct OriginalRange {
    start_line: Option<u64>,
    line: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum DiffSide {
    Left,
    Right,
}

#[derive(Debug, Serialize)]
struct ReviewThreadComment {
    id: String,
    author: Option<String>,
    body: String,
    created_at: String,
    updated_at: String,
    diff_hunk: String,
}

#[derive(Serialize)]
struct CompactPrContext<'a> {
    pull_request: &'a PullRequest,
    conversation_comments: Vec<CompactComment<'a>>,
    reviews: Vec<CompactReview<'a>>,
    review_threads: Vec<CompactReviewThread<'a>>,
}

impl<'a> From<&'a PrContext> for CompactPrContext<'a> {
    fn from(context: &'a PrContext) -> Self {
        Self {
            pull_request: &context.pull_request,
            conversation_comments: context
                .conversation_comments
                .iter()
                .map(CompactComment::from)
                .collect(),
            reviews: context.reviews.iter().map(CompactReview::from).collect(),
            review_threads: context
                .review_threads
                .iter()
                .map(CompactReviewThread::from)
                .collect(),
        }
    }
}

#[derive(Serialize)]
struct CompactComment<'a> {
    author: &'a Option<String>,
    body: &'a str,
}

impl<'a> From<&'a Comment> for CompactComment<'a> {
    fn from(comment: &'a Comment) -> Self {
        Self {
            author: &comment.author,
            body: &comment.body,
        }
    }
}

#[derive(Serialize)]
struct CompactReview<'a> {
    state: &'a ReviewState,
    author: &'a Option<String>,
    body: &'a str,
}

impl<'a> From<&'a Review> for CompactReview<'a> {
    fn from(review: &'a Review) -> Self {
        Self {
            state: &review.state,
            author: &review.author,
            body: &review.body,
        }
    }
}

#[derive(Serialize)]
struct CompactReviewThread<'a> {
    id: &'a str,
    resolution: &'a ThreadResolution,
    is_outdated: bool,
    location: &'a ThreadLocation,
    comments: Vec<CompactComment<'a>>,
}

impl<'a> From<&'a ReviewThread> for CompactReviewThread<'a> {
    fn from(thread: &'a ReviewThread) -> Self {
        Self {
            id: &thread.id,
            resolution: &thread.resolution,
            is_outdated: thread.is_outdated,
            location: &thread.location,
            comments: thread
                .comments
                .iter()
                .map(|comment| CompactComment {
                    author: &comment.author,
                    body: &comment.body,
                })
                .collect(),
        }
    }
}

pub fn run(_sh: &Shell, args: &[OsString]) -> Result<()> {
    execute(Args::parse_from(args).into())
}

pub fn run_with_flags(_sh: &Shell, args: crate::cmd::main_cmd::PrContextArgs) -> Result<()> {
    execute(args.into())
}

#[derive(Debug, Clone)]
struct PrContextOptions {
    repo_or_url: String,
    pr_number: Option<u64>,
    token: Option<String>,
    code_only: bool,
    unresolved_only: bool,
    compact: bool,
    format: OutputFormat,
}

impl From<Args> for PrContextOptions {
    fn from(value: Args) -> Self {
        Self {
            repo_or_url: value.repo_or_url,
            pr_number: value.pr_number,
            token: value.token,
            code_only: value.code_only,
            unresolved_only: value.unresolved_only,
            compact: value.compact,
            format: value.format,
        }
    }
}

impl From<crate::cmd::main_cmd::PrContextArgs> for PrContextOptions {
    fn from(value: crate::cmd::main_cmd::PrContextArgs) -> Self {
        Self {
            repo_or_url: value.repo_or_url,
            pr_number: value.pr_number,
            token: value.token,
            code_only: value.code_only,
            unresolved_only: value.unresolved_only,
            compact: value.compact,
            format: value.format,
        }
    }
}

fn execute(options: PrContextOptions) -> Result<()> {
    let (owner, repo, number) = parse_input(&options.repo_or_url, options.pr_number)?;
    let mut context = runtime::block_on(fetch_pr_context(&owner, &repo, number, options.token))??;
    context.apply_filters(options.code_only, options.unresolved_only);

    match options.format {
        OutputFormat::Markdown => print!("{}", format_as_markdown(&context, options.compact)),
        OutputFormat::Json => print_json(&context, options.compact)?,
    }

    Ok(())
}

fn print_json(context: &PrContext, compact: bool) -> Result<()> {
    if compact {
        println!(
            "{}",
            serde_json::to_string_pretty(&CompactPrContext::from(context))?
        );
    } else {
        println!("{}", serde_json::to_string_pretty(context)?);
    }
    Ok(())
}

async fn fetch_pr_context(
    owner: &str,
    repo: &str,
    number: u64,
    token: Option<String>,
) -> Result<PrContext> {
    let data = github::Github::new(token)?
        .fetch_pr_review(owner, repo, number)
        .await
        .context("Failed to fetch pull request review context")?;
    data.try_into()
}

impl TryFrom<github::PullRequestReviewData> for PrContext {
    type Error = color_eyre::eyre::Error;

    fn try_from(data: github::PullRequestReviewData) -> Result<Self> {
        let pull_request = PullRequest {
            repository: data.repository,
            number: data.pull_request.number,
            url: data.pull_request.url,
            title: data.pull_request.title,
            state: parse_pull_request_state(&data.pull_request.state)?,
        };
        let conversation_comments = data
            .conversation_comments
            .into_iter()
            .map(|comment| Comment {
                id: comment.id,
                author: author_login(comment.author),
                body: comment.body,
                created_at: comment.created_at,
                updated_at: comment.updated_at,
            })
            .collect();
        let reviews = data
            .reviews
            .into_iter()
            .map(|review| {
                Ok(Review {
                    id: review.id,
                    state: parse_review_state(&review.state)?,
                    author: author_login(review.author),
                    body: review.body,
                    submitted_at: review.submitted_at,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let review_threads = data
            .review_threads
            .into_iter()
            .map(convert_review_thread)
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            pull_request,
            conversation_comments,
            reviews,
            review_threads,
        })
    }
}

fn convert_review_thread(thread: github::ReviewThread) -> Result<ReviewThread> {
    let location = match thread.subject_type.as_str() {
        "FILE" => ThreadLocation::File {
            path: thread.path.clone(),
        },
        "LINE" => line_location(&thread)?,
        subject_type => eyre::bail!("Unsupported review thread subject type: {subject_type}"),
    };
    let resolution = if thread.is_resolved {
        ThreadResolution::Resolved {
            resolved_by: author_login(thread.resolved_by),
        }
    } else {
        ThreadResolution::Unresolved
    };
    let comments = thread
        .comments
        .into_iter()
        .map(|comment| ReviewThreadComment {
            id: comment.id,
            author: author_login(comment.author),
            body: comment.body,
            created_at: comment.created_at,
            updated_at: comment.updated_at,
            diff_hunk: comment.diff_hunk,
        })
        .collect();

    Ok(ReviewThread {
        id: thread.id,
        resolution,
        is_outdated: thread.is_outdated,
        location,
        comments,
    })
}

fn line_location(thread: &github::ReviewThread) -> Result<ThreadLocation> {
    let current = current_range(thread)?;
    let original = thread.original_line.map(|line| OriginalRange {
        start_line: thread.original_start_line,
        line,
    });

    if current.is_none() && original.is_none() {
        eyre::bail!("Line review thread {} has no line anchor", thread.id);
    }

    Ok(ThreadLocation::Line {
        path: thread.path.clone(),
        current,
        original,
    })
}

fn current_range(thread: &github::ReviewThread) -> Result<Option<DiffRange>> {
    let Some(line) = thread.line else {
        if thread.start_line.is_some() || thread.start_diff_side.is_some() {
            eyre::bail!(
                "Review thread {} has a partial current line anchor",
                thread.id
            );
        }
        return Ok(None);
    };
    let start = match (thread.start_line, thread.start_diff_side.as_deref()) {
        (Some(line), Some(side)) => Some(DiffPosition {
            line,
            side: parse_diff_side(side)?,
        }),
        (None, None) => None,
        _ => eyre::bail!(
            "Review thread {} has a partial start line anchor",
            thread.id
        ),
    };

    Ok(Some(DiffRange {
        start,
        end: DiffPosition {
            line,
            side: parse_diff_side(&thread.diff_side)?,
        },
    }))
}

fn parse_pull_request_state(state: &str) -> Result<PullRequestState> {
    match state {
        "OPEN" => Ok(PullRequestState::Open),
        "CLOSED" => Ok(PullRequestState::Closed),
        "MERGED" => Ok(PullRequestState::Merged),
        state => eyre::bail!("Unsupported pull request state: {state}"),
    }
}

fn parse_review_state(state: &str) -> Result<ReviewState> {
    match state {
        "PENDING" => Ok(ReviewState::Pending),
        "COMMENTED" => Ok(ReviewState::Commented),
        "APPROVED" => Ok(ReviewState::Approved),
        "CHANGES_REQUESTED" => Ok(ReviewState::ChangesRequested),
        "DISMISSED" => Ok(ReviewState::Dismissed),
        state => eyre::bail!("Unsupported review state: {state}"),
    }
}

fn parse_diff_side(side: &str) -> Result<DiffSide> {
    match side {
        "LEFT" => Ok(DiffSide::Left),
        "RIGHT" => Ok(DiffSide::Right),
        side => eyre::bail!("Unsupported review diff side: {side}"),
    }
}

fn author_login(author: Option<github::Author>) -> Option<String> {
    author.map(|author| author.login)
}

fn git_remote_repo(sh: &Shell) -> Result<(String, String)> {
    let output = sh
        .cmd("git")
        .args(&["remote", "get-url", "origin"])
        .output()
        .context("Failed to run git remote get-url origin")?;

    if !output.status.success() {
        eyre::bail!("Not in a git repository or no origin remote configured");
    }

    let remote_url = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in git remote URL")?
        .trim()
        .to_string();
    let github_pos = remote_url
        .find("github.com")
        .ok_or_else(|| eyre::eyre!("Not a GitHub repository URL: {remote_url}"))?;
    let after_github = &remote_url[github_pos + "github.com".len()..];
    let path = if let Some(stripped) = after_github.strip_prefix(':') {
        stripped
    } else if let Some(stripped) = after_github.strip_prefix('/') {
        stripped
    } else {
        eyre::bail!("Invalid GitHub URL format: {remote_url}");
    };
    let path = path.strip_suffix(".git").unwrap_or(path);
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() != 2 {
        eyre::bail!("Invalid GitHub repository path: {path}");
    }

    Ok((parts[0].to_string(), parts[1].to_string()))
}

fn parse_input(input: &str, pr_number: Option<u64>) -> Result<(String, String, u64)> {
    if input.starts_with("http://") || input.starts_with("https://") {
        return github::parse_url(input);
    }

    if let Ok(number) = input.parse::<u64>() {
        if pr_number.is_some() {
            eyre::bail!("PR number provided twice: as first argument and as second argument");
        }
        let (owner, repo) = git_remote_repo(&Shell::new()?)?;
        return Ok((owner, repo, number));
    }

    let parts: Vec<&str> = input.split('/').collect();
    if parts.len() != 2 {
        eyre::bail!(
            "Repository must be in format 'owner/repo', a PR number, or a valid GitHub PR URL"
        );
    }
    let number = pr_number
        .ok_or_else(|| eyre::eyre!("PR number is required when using 'owner/repo' format"))?;
    Ok((parts[0].to_string(), parts[1].to_string(), number))
}

fn format_as_markdown(context: &PrContext, compact: bool) -> String {
    let mut output = format!(
        "# PR #{} — {}\n\n**Repository:** `{}`\n**State:** {}\n**URL:** {}\n\n**Review threads:** {}\n**Reviews:** {}\n**Conversation comments:** {}\n\n",
        context.pull_request.number,
        context.pull_request.title,
        context.pull_request.repository,
        pull_request_state_label(&context.pull_request.state),
        context.pull_request.url,
        context.review_threads.len(),
        context.reviews.len(),
        context.conversation_comments.len(),
    );

    if !context.review_threads.is_empty() {
        output.push_str("## Review Threads\n\n");
        for (index, thread) in context.review_threads.iter().enumerate() {
            format_thread(&mut output, index + 1, thread, compact);
        }
    }
    if !context.reviews.is_empty() {
        output.push_str("## Reviews\n\n");
        for review in &context.reviews {
            format_review(&mut output, review, compact);
        }
    }
    if !context.conversation_comments.is_empty() {
        output.push_str("## Conversation Comments\n\n");
        for comment in &context.conversation_comments {
            format_comment(&mut output, comment, compact);
        }
    }

    output
}

fn format_thread(output: &mut String, index: usize, thread: &ReviewThread, compact: bool) {
    let resolution = match &thread.resolution {
        ThreadResolution::Unresolved => "unresolved".to_string(),
        ThreadResolution::Resolved { resolved_by } => {
            format!("resolved by @{}", resolved_by.as_deref().unwrap_or("ghost"))
        }
    };
    let outdated = if thread.is_outdated { ", outdated" } else { "" };
    output.push_str(&format!("### Thread {index} — {resolution}{outdated}\n\n"));
    output.push_str(&format!("**Thread ID:** `{}`\n", thread.id));
    format_location(output, &thread.location);

    let diff_hunk = thread
        .comments
        .iter()
        .map(|comment| comment.diff_hunk.as_str())
        .find(|diff_hunk| !diff_hunk.is_empty());
    if !compact {
        if let Some(diff_hunk) = diff_hunk {
            output.push_str("\n**Diff:**\n```diff\n");
            output.push_str(diff_hunk);
            output.push_str("\n```\n");
        }
    }

    for comment in &thread.comments {
        output.push_str(&format!(
            "\n**@{}{}:**\n{}\n",
            comment.author.as_deref().unwrap_or("ghost"),
            if compact {
                String::new()
            } else {
                format!(" — {}", comment.created_at)
            },
            comment.body,
        ));
    }
    output.push_str("\n---\n\n");
}

fn format_location(output: &mut String, location: &ThreadLocation) {
    match location {
        ThreadLocation::File { path } => {
            output.push_str(&format!("**File:** `{path}`\n"));
        }
        ThreadLocation::Line {
            path,
            current,
            original,
        } => {
            output.push_str(&format!("**File:** `{path}`\n"));
            if let Some(range) = current {
                output.push_str(&format!(
                    "**Line:** {}\n",
                    range_label(range.start.as_ref().map(|start| start.line), range.end.line)
                ));
            } else if let Some(range) = original {
                output.push_str(&format!(
                    "**Original line:** {}\n",
                    range_label(range.start_line, range.line)
                ));
            }
        }
    }
}

fn range_label(start_line: Option<u64>, line: u64) -> String {
    start_line
        .map(|start| format!("{start}-{line}"))
        .unwrap_or_else(|| line.to_string())
}

fn format_review(output: &mut String, review: &Review, compact: bool) {
    output.push_str(&format!(
        "### {} by @{}\n\n",
        review_state_label(&review.state),
        review.author.as_deref().unwrap_or("ghost")
    ));
    if !compact {
        output.push_str(&format!("**Review ID:** `{}`\n", review.id));
        if let Some(submitted_at) = &review.submitted_at {
            output.push_str(&format!("**Submitted:** {submitted_at}\n"));
        }
        output.push('\n');
    }
    output.push_str(&review.body);
    output.push_str("\n\n---\n\n");
}

fn format_comment(output: &mut String, comment: &Comment, compact: bool) {
    output.push_str(&format!(
        "### @{}\n\n",
        comment.author.as_deref().unwrap_or("ghost")
    ));
    if !compact {
        output.push_str(&format!(
            "**Comment ID:** `{}`\n**Created:** {}\n\n",
            comment.id, comment.created_at
        ));
    }
    output.push_str(&comment.body);
    output.push_str("\n\n---\n\n");
}

fn pull_request_state_label(state: &PullRequestState) -> &'static str {
    match state {
        PullRequestState::Open => "open",
        PullRequestState::Closed => "closed",
        PullRequestState::Merged => "merged",
    }
}

fn review_state_label(state: &ReviewState) -> &'static str {
    match state {
        ReviewState::Pending => "pending",
        ReviewState::Commented => "commented",
        ReviewState::Approved => "approved",
        ReviewState::ChangesRequested => "changes requested",
        ReviewState::Dismissed => "dismissed",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        current_range, format_as_markdown, CompactPrContext, DiffPosition, DiffRange, DiffSide,
        OriginalRange, PrContext, PullRequest, PullRequestState, ReviewThread, ReviewThreadComment,
        ThreadLocation, ThreadResolution,
    };

    #[test]
    fn filters_to_unresolved_code_threads_without_dropping_outdated_threads() {
        let mut context = sample_context();

        context.apply_filters(true, true);

        assert!(context.conversation_comments.is_empty());
        assert!(context.reviews.is_empty());
        assert_eq!(context.review_threads.len(), 1);
        assert!(context.review_threads[0].is_outdated);
    }

    #[test]
    fn compact_json_keeps_thread_state_and_omits_comment_metadata() {
        let context = sample_context();
        let value = serde_json::to_value(CompactPrContext::from(&context)).unwrap();

        assert_eq!(
            value["review_threads"][0]["resolution"]["state"],
            "unresolved"
        );
        assert_eq!(value["review_threads"][0]["id"], "PRRT_1");
        assert_eq!(value["review_threads"][0]["is_outdated"], true);
        assert_eq!(value["review_threads"][0]["location"]["subject"], "line");
        assert!(value["review_threads"][0]["comments"][0]
            .get("created_at")
            .is_none());
        assert!(value["review_threads"][0]["comments"][0]
            .get("diff_hunk")
            .is_none());
    }

    #[test]
    fn markdown_exposes_resolution_outdated_and_original_anchor() {
        let context = sample_context();
        let markdown = format_as_markdown(&context, true);

        assert!(markdown.contains("Thread 1 — unresolved, outdated"));
        assert!(markdown.contains("**Original line:** 12"));
        assert!(markdown.contains("Thread 2 — resolved by @maintainer"));
    }

    #[test]
    fn rejects_a_multiline_anchor_without_its_diff_side() {
        let thread = github_thread_with_partial_start_anchor();

        let error = current_range(&thread).unwrap_err().to_string();

        assert!(error.contains("partial start line anchor"));
    }

    fn github_thread_with_partial_start_anchor() -> crate::github::ReviewThread {
        crate::github::ReviewThread {
            id: "PRRT_1".to_string(),
            is_resolved: false,
            is_outdated: false,
            path: "src/lib.rs".to_string(),
            line: Some(12),
            diff_side: "RIGHT".to_string(),
            start_line: Some(10),
            start_diff_side: None,
            original_line: Some(12),
            original_start_line: Some(10),
            subject_type: "LINE".to_string(),
            resolved_by: None,
            comments: Vec::new(),
        }
    }

    fn sample_context() -> PrContext {
        PrContext {
            pull_request: PullRequest {
                repository: "owner/repo".to_string(),
                number: 7,
                url: "https://github.com/owner/repo/pull/7".to_string(),
                title: "Thread-aware comments".to_string(),
                state: PullRequestState::Open,
            },
            conversation_comments: vec![super::Comment {
                id: "IC_1".to_string(),
                author: None,
                body: "conversation".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                updated_at: "2026-01-01T00:00:00Z".to_string(),
            }],
            reviews: vec![super::Review {
                id: "PRR_1".to_string(),
                state: super::ReviewState::ChangesRequested,
                author: Some("reviewer".to_string()),
                body: "changes requested".to_string(),
                submitted_at: Some("2026-01-01T00:00:00Z".to_string()),
            }],
            review_threads: vec![
                ReviewThread {
                    id: "PRRT_1".to_string(),
                    resolution: ThreadResolution::Unresolved,
                    is_outdated: true,
                    location: ThreadLocation::Line {
                        path: "src/lib.rs".to_string(),
                        current: None,
                        original: Some(OriginalRange {
                            start_line: None,
                            line: 12,
                        }),
                    },
                    comments: vec![ReviewThreadComment {
                        id: "PRRC_1".to_string(),
                        author: Some("reviewer".to_string()),
                        body: "fix this".to_string(),
                        created_at: "2026-01-01T00:00:00Z".to_string(),
                        updated_at: "2026-01-01T00:00:00Z".to_string(),
                        diff_hunk: "@@ -1 +1 @@".to_string(),
                    }],
                },
                ReviewThread {
                    id: "PRRT_2".to_string(),
                    resolution: ThreadResolution::Resolved {
                        resolved_by: Some("maintainer".to_string()),
                    },
                    is_outdated: false,
                    location: ThreadLocation::Line {
                        path: "src/main.rs".to_string(),
                        current: Some(DiffRange {
                            start: Some(DiffPosition {
                                line: 20,
                                side: DiffSide::Right,
                            }),
                            end: DiffPosition {
                                line: 21,
                                side: DiffSide::Right,
                            },
                        }),
                        original: None,
                    },
                    comments: Vec::new(),
                },
            ],
        }
    }
}
