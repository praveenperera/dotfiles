use chrono::{DateTime, Utc};
use clap::Parser;
use eyre::{Context as _, Result};
use log::{info, warn};
use serde::Serialize;
use std::ffi::OsString;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use xshell::{cmd, Shell};

#[derive(Debug, Clone, Parser)]
#[command(
    name = "better-context",
    about = "Clone/update a repo for agent exploration"
)]
pub struct BetterContext {
    /// Repository: owner/repo, URL, or local path
    pub repo: String,

    /// Force fresh clone
    #[arg(short, long)]
    pub fresh: bool,

    /// Checkout specific git ref (branch, tag, or SHA)
    #[arg(short, long)]
    pub r#ref: Option<String>,

    /// Clone complete history instead of single-branch
    #[arg(long)]
    pub full: bool,

    /// Suppress progress logs
    #[arg(short, long)]
    pub quiet: bool,
}

#[derive(Debug, Serialize)]
pub struct Output {
    pub path: String,
    pub url: Option<String>,
    pub branch: String,
    #[serde(rename = "updated_at")]
    pub updated_at: DateTime<Utc>,
    pub stale: bool,
}

enum RepoSource {
    GitHub {
        owner: String,
        repo: String,
    },
    Url {
        url: String,
        host: String,
        owner: String,
        repo: String,
    },
    Local {
        path: PathBuf,
    },
}

impl RepoSource {
    fn clone_url(&self) -> String {
        match self {
            RepoSource::GitHub { owner, repo } => format!("https://github.com/{owner}/{repo}"),
            RepoSource::Url { url, .. } => url.clone(),
            RepoSource::Local { path } => path.display().to_string(),
        }
    }
}

pub fn run(sh: &Shell, args: &[OsString]) -> Result<()> {
    let flags = BetterContext::parse_from(args);
    run_with_flags(sh, flags)
}

pub fn run_with_flags(sh: &Shell, flags: BetterContext) -> Result<()> {
    let source = parse_repo_source(&flags.repo)?;
    let mut stale = false;

    let (path, url, branch) = match &source {
        RepoSource::Local { path } => {
            if !path.join(".git").exists() {
                eyre::bail!("{} is not a git repository", path.display());
            }
            let branch = get_current_branch(sh, path)?;
            (path.clone(), None, branch)
        }
        _ => {
            let cache_path = repo_cache_path(&source)?;
            let clone_url = source.clone_url();

            if flags.fresh && cache_path.exists() {
                if !flags.quiet {
                    info!("Removing existing cache for fresh clone");
                }
                std::fs::remove_dir_all(&cache_path)?;
            }

            if cache_path.exists() {
                match update_repo_with_retry(sh, &cache_path, flags.quiet) {
                    Ok(()) => {}
                    Err(e) => {
                        warn!("Failed to update repo: {}. Using cached version.", e);
                        stale = true;
                    }
                }
            } else {
                clone_repo_with_retry(sh, &clone_url, &cache_path, flags.full, flags.quiet)?;
            }

            let branch = if let Some(ref git_ref) = flags.r#ref {
                match checkout_ref(sh, &cache_path, git_ref) {
                    Ok(()) => git_ref.clone(),
                    Err(e) => {
                        warn!(
                            "Could not checkout {}: {}. Using default branch.",
                            git_ref, e
                        );
                        get_current_branch(sh, &cache_path)?
                    }
                }
            } else {
                get_current_branch(sh, &cache_path)?
            };

            (cache_path, Some(clone_url), branch)
        }
    };

    let output = Output {
        path: path.to_string_lossy().to_string(),
        url,
        branch,
        updated_at: Utc::now(),
        stale,
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn update_repo_with_retry(sh: &Shell, path: &PathBuf, quiet: bool) -> Result<()> {
    let delays = [1, 2, 4];
    let mut last_err = None;

    for (i, delay) in delays.iter().enumerate() {
        if !quiet {
            info!(
                "Updating cached repo at {} (attempt {})",
                path.display(),
                i + 1
            );
        }

        let _guard = sh.push_dir(path);
        let fetch = cmd!(sh, "git fetch --all --prune");
        let fetch = if quiet { fetch.quiet() } else { fetch };

        match fetch.run() {
            Ok(()) => {
                let reset = if quiet {
                    cmd!(sh, "git reset --hard --quiet origin/HEAD").quiet()
                } else {
                    cmd!(sh, "git reset --hard origin/HEAD")
                };
                reset.run()?;
                return Ok(());
            }
            Err(e) => {
                last_err = Some(e);
                if i < delays.len() - 1 {
                    if !quiet {
                        warn!("Fetch failed, retrying in {}s...", delay);
                    }
                    thread::sleep(Duration::from_secs(*delay));
                }
            }
        }
    }

    Err(last_err.unwrap().into())
}

fn clone_repo_with_retry(
    sh: &Shell,
    url: &str,
    path: &PathBuf,
    full: bool,
    quiet: bool,
) -> Result<()> {
    let delays = [1, 2, 4];
    let mut last_err = None;

    std::fs::create_dir_all(path.parent().unwrap())?;

    for (i, delay) in delays.iter().enumerate() {
        if !quiet {
            info!("Cloning {} to {} (attempt {})", url, path.display(), i + 1);
        }

        let clone_cmd = match (full, quiet) {
            (true, true) => cmd!(sh, "git clone --quiet {url} {path}").quiet(),
            (true, false) => cmd!(sh, "git clone {url} {path}"),
            (false, true) => cmd!(sh, "git clone --quiet --single-branch {url} {path}").quiet(),
            (false, false) => cmd!(sh, "git clone --single-branch {url} {path}"),
        };

        match clone_cmd.run() {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = Some(e);
                let _ = std::fs::remove_dir_all(path);
                if i < delays.len() - 1 {
                    if !quiet {
                        warn!("Clone failed, retrying in {}s...", delay);
                    }
                    thread::sleep(Duration::from_secs(*delay));
                }
            }
        }
    }

    Err(eyre::Report::from(last_err.unwrap())
        .wrap_err("Failed to clone repository after 3 attempts"))
}

fn parse_repo_source(input: &str) -> Result<RepoSource> {
    let path = PathBuf::from(input);
    if path.exists() || input.starts_with('/') || input.starts_with('.') {
        let canonical = path.canonicalize().wrap_err("Local path does not exist")?;
        return Ok(RepoSource::Local { path: canonical });
    }

    if input.starts_with("https://") {
        return parse_https_url(input);
    }

    if input.starts_with("git@") {
        return parse_ssh_url(input);
    }

    let parts: Vec<&str> = input.split('/').collect();
    if parts.len() == 2 {
        return Ok(RepoSource::GitHub {
            owner: parts[0].to_string(),
            repo: parts[1].trim_end_matches(".git").to_string(),
        });
    }

    eyre::bail!(
        "Invalid repo format. Expected: owner/repo, URL, or local path. Got: {}",
        input
    )
}

fn parse_https_url(input: &str) -> Result<RepoSource> {
    let without_scheme = input.strip_prefix("https://").unwrap();
    let parts: Vec<&str> = without_scheme.splitn(2, '/').collect();
    if parts.len() != 2 {
        eyre::bail!("Invalid HTTPS URL: {}", input);
    }

    let host = parts[0].to_string();
    let path_parts: Vec<&str> = parts[1].split('/').collect();
    if path_parts.len() < 2 {
        eyre::bail!("Invalid repository path in URL: {}", input);
    }

    let owner = path_parts[0].to_string();
    let repo = path_parts[1].trim_end_matches(".git").to_string();

    Ok(RepoSource::Url {
        url: input.to_string(),
        host,
        owner,
        repo,
    })
}

fn parse_ssh_url(input: &str) -> Result<RepoSource> {
    // git@github.com:owner/repo.git
    let without_prefix = input.strip_prefix("git@").unwrap();
    let parts: Vec<&str> = without_prefix.splitn(2, ':').collect();
    if parts.len() != 2 {
        eyre::bail!("Invalid SSH URL: {}", input);
    }

    let host = parts[0].to_string();
    let path_parts: Vec<&str> = parts[1].split('/').collect();
    if path_parts.len() < 2 {
        eyre::bail!("Invalid repository path in SSH URL: {}", input);
    }

    let owner = path_parts[0].to_string();
    let repo = path_parts[1].trim_end_matches(".git").to_string();

    let url = format!("https://{}/{}/{}", host, owner, repo);

    Ok(RepoSource::Url {
        url,
        host,
        owner,
        repo,
    })
}

fn cache_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME").wrap_err("HOME not set")?;
    Ok(PathBuf::from(home).join(".cache/cmd/repos"))
}

fn repo_cache_path(source: &RepoSource) -> Result<PathBuf> {
    let base = cache_dir()?;
    match source {
        RepoSource::GitHub { owner, repo } => Ok(base.join("github.com").join(owner).join(repo)),
        RepoSource::Url {
            host, owner, repo, ..
        } => Ok(base.join(host).join(owner).join(repo)),
        RepoSource::Local { path } => Ok(path.clone()),
    }
}

fn get_current_branch(sh: &Shell, path: &PathBuf) -> Result<String> {
    let _guard = sh.push_dir(path);
    let branch = cmd!(sh, "git rev-parse --abbrev-ref HEAD").quiet().read()?;
    Ok(branch.trim().to_string())
}

fn checkout_ref(sh: &Shell, path: &PathBuf, git_ref: &str) -> Result<()> {
    let _guard = sh.push_dir(path);
    cmd!(sh, "git checkout {git_ref}").quiet().run()?;
    Ok(())
}
