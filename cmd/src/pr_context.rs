use clap::{Parser, ValueEnum};
use color_eyre::eyre::{Context, Result};
use serde::Serialize;
use std::ffi::OsString;
use xshell::Shell;

use crate::github;

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Markdown,
    Json,
}

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

    /// Output format
    #[arg(short = 'f', long, default_value = "markdown")]
    pub format: OutputFormat,
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

    // parse the input to extract owner, repo, and pr_number
    let (owner, repo, pr_number) = parse_input(&args.repo_or_url, args.pr_number)?;

    let runtime = tokio::runtime::Runtime::new()?;
    let mut pr_context = runtime.block_on(fetch_pr_context(&owner, &repo, pr_number, &args.token))?;

    // filter to only comments with code references if requested
    if args.code_only {
        pr_context.comments.retain(|c| c.code_reference.is_some());
    }

    // output based on format flag
    match args.format {
        OutputFormat::Markdown => {
            let markdown = format_as_markdown(&pr_context, args.compact);
            print!("{}", markdown);
        }
        OutputFormat::Json => {
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
        }
    }

    Ok(())
}

pub fn run_with_flags(_sh: &Shell, args: crate::cmd::main_cmd::PrContextArgs) -> Result<()> {

    // parse the input to extract owner, repo, and pr_number
    let (owner, repo, pr_number) = parse_input(&args.repo_or_url, args.pr_number)?;

    let runtime = tokio::runtime::Runtime::new()?;
    let mut pr_context = runtime.block_on(fetch_pr_context(&owner, &repo, pr_number, &args.token))?;

    // filter to only comments with code references if requested
    if args.code_only {
        pr_context.comments.retain(|c| c.code_reference.is_some());
    }

    // output based on format flag
    match args.format {
        OutputFormat::Markdown => {
            let markdown = format_as_markdown(&pr_context, args.compact);
            print!("{}", markdown);
        }
        OutputFormat::Json => {
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
        }
    }

    Ok(())
}

async fn fetch_pr_context(
    owner: &str,
    repo: &str,
    pr_number: u64,
    token: &Option<String>,
) -> Result<PrContext> {
    let gh = github::Github::new(token.clone())?;

    // fetch review comments (comments on code)
    let review_comments = gh
        .fetch_review_comments(owner, repo, pr_number)
        .await
        .context("Failed to fetch review comments")?;

    // fetch issue comments (general comments on the PR)
    let issue_comments = gh
        .fetch_issue_comments(owner, repo, pr_number)
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
        return github::parse_url(input);
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

// format PR context as markdown
fn format_as_markdown(pr_context: &PrContext, compact: bool) -> String {
    let mut output = String::new();

    output.push_str(&format!("# PR #{} - {}\n\n", pr_context.pr_number, pr_context.repo));
    output.push_str(&format!("**Total comments:** {}\n\n", pr_context.comments.len()));

    for comment in &pr_context.comments {
        output.push_str("---\n\n");

        if !compact {
            output.push_str(&format!("**Comment ID:** {}\n", comment.comment_id));
            output.push_str(&format!("**Type:** {}\n", comment.comment_type));
        }

        output.push_str(&format!("**Author:** @{}\n", comment.author));

        if !compact {
            output.push_str(&format!("**Created:** {}\n", comment.created_at));
        }

        if let Some(code_ref) = &comment.code_reference {
            output.push_str(&format!("**File:** `{}`\n", code_ref.file_path));

            if let Some(line) = code_ref.line {
                output.push_str(&format!("**Line:** {}\n", line));
            } else if let Some(start_line) = code_ref.start_line {
                output.push_str(&format!("**Lines:** {}-...\n", start_line));
            }

            if !compact && !code_ref.diff_hunk.is_empty() {
                output.push_str("\n**Diff:**\n");
                output.push_str("```diff\n");
                output.push_str(&code_ref.diff_hunk);
                output.push_str("\n```\n");
            }
        }

        output.push_str("\n**Comment:**\n");
        output.push_str(&comment.body);
        output.push_str("\n\n");
    }

    output
}
