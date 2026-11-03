//! Commands for the ship and gh-fix-ci skills: `plan` resolves /ship words
//! into a step plan, the waits block on CI, and `ci-failures` reports failures
//!
//! The waits, `watch-pr` and `watch-run`, block until a PR's CI or a workflow run
//! ends, stay quiet on stdout meanwhile, then print one line and exit 0 only
//! when everything passed. They run as homebased tasks, so the thread that
//! submitted one wakes once, when there is a result to act on. Waiting on CI or
//! a run has no time limit of its own; the task's `--timeout` decides how long
//! that may take. Only things that may never happen are bounded: Greptile gets
//! 1 hour after CI ends, and a workflow run gets 10 minutes to appear
//!
//! `ci-failures` is a one-shot report of a PR's failing checks with a snippet of
//! each failed job's log

mod ci_failures;
mod plan;
mod watch_pr;
mod watch_run;

use std::future::Future;
use std::process::Command;

use clap::Subcommand;
use eyre::{bail, eyre, Context, Result};

use crate::github::{self, ApiStatusError, GraphqlQueryError};
use crate::runtime;

#[derive(Debug, Clone, Subcommand)]
pub enum ShipCmd {
    /// Resolve /ship words and repos.toml into an ordered step plan
    #[command(visible_alias = "p")]
    Plan(plan::PlanArgs),

    /// Wait for a PR's CI, and with --greptile for Greptile's review of the head
    /// commit, then print one line
    #[command(visible_alias = "wp")]
    WatchPr(watch_pr::WatchPrArgs),

    /// Wait for the GitHub Actions run of a workflow on a commit, then print one line
    #[command(visible_alias = "wr")]
    WatchRun(watch_run::WatchRunArgs),

    /// Report a PR's failing checks now, with a log snippet of each failed
    /// GitHub Actions job; exits 1 when any check failed
    #[command(visible_alias = "cif")]
    CiFailures(ci_failures::CiFailuresArgs),
}

pub fn run_with_flags(subcommand: ShipCmd) -> Result<()> {
    match subcommand {
        ShipCmd::Plan(args) => plan::run(args),
        ShipCmd::WatchPr(args) => watch("watch-pr", watch_pr::run(args)),
        ShipCmd::WatchRun(args) => watch("watch-run", watch_run::run(args)),
        // a report, not a wait, so it prints its own output rather than one line
        ShipCmd::CiFailures(args) => ci_failures::run(args),
    }
}

/// Runs a wait and prints its one line, exiting 1 unless everything passed
fn watch(name: &str, wait: impl Future<Output = Result<Report>>) -> Result<()> {
    let result = runtime::block_on(wait)?;

    // a setup failure, such as an unknown PR, still ends with the one result line
    let report =
        result.unwrap_or_else(|error| Report::new(format!("{name}: {}", one_line(&error)), false));

    println!("{}", report.line);
    if !report.ok {
        std::process::exit(1);
    }

    Ok(())
}

/// The one line a watch prints, and whether it ends with exit 0
#[derive(Debug, Clone, PartialEq, Eq)]
struct Report {
    line: String,
    ok: bool,
}

impl Report {
    fn new(line: String, ok: bool) -> Self {
        // a check or job name can hold a newline, which would split the one line
        let line = line.replace(['\n', '\r'], " ");
        Self { line, ok }
    }
}

/// A repository as `owner/name`
#[derive(Debug, Clone)]
struct Repo {
    owner: String,
    name: String,
}

impl Repo {
    fn parse(value: &str) -> Result<Self> {
        let Some((owner, name)) = value.trim().split_once('/') else {
            bail!("expected a repository as owner/name, got `{value}`");
        };

        if owner.is_empty() || name.is_empty() || name.contains('/') {
            bail!("expected a repository as owner/name, got `{value}`");
        }

        Ok(Self {
            owner: owner.to_string(),
            name: name.to_string(),
        })
    }

    /// `--repo` when given, else the current directory's repository, read from
    /// git config the way gh picks it: the remote `gh repo set-default` marked,
    /// then `upstream`, `github`, and `origin`. It stays local, so a GitHub
    /// outage when a watch starts cannot end it
    fn resolve(explicit: Option<&str>) -> Result<Self> {
        if let Some(repo) = explicit {
            return Self::parse(repo);
        }

        let remotes = git(&["remote"])?;
        let remotes = remotes.lines().collect::<Vec<_>>();

        // gh stores the default as `remote.<name>.gh-resolved`, set to `base` or
        // to an owner/name that differs from the remote's own
        let resolved =
            git(&["config", "--get-regexp", r"^remote\..*\.gh-resolved$"]).unwrap_or_default();
        for line in resolved.lines() {
            let Some((key, value)) = line.split_once(' ') else {
                continue;
            };

            if value.contains('/') {
                return Self::parse(value);
            }

            if let Some(remote) = key
                .strip_prefix("remote.")
                .and_then(|key| key.strip_suffix(".gh-resolved"))
            {
                return Self::from_remote(remote);
            }
        }

        let remote = ["upstream", "github", "origin"]
            .into_iter()
            .find(|name| remotes.contains(name))
            .or(remotes.first().copied())
            .ok_or_else(|| eyre!("this repository has no git remote; pass --repo owner/name"))?;

        Self::from_remote(remote)
    }

    fn from_remote(remote: &str) -> Result<Self> {
        let url = git(&["remote", "get-url", remote])?;
        let url = url.trim();
        let path = [
            "git@github.com:",
            "ssh://git@github.com/",
            "https://github.com/",
            "http://github.com/",
        ]
        .into_iter()
        .find_map(|prefix| url.strip_prefix(prefix))
        .ok_or_else(|| {
            eyre!("remote {remote} is not a GitHub URL: {url}; pass --repo owner/name")
        })?;
        let path = path.trim_end_matches('/');

        Self::parse(path.strip_suffix(".git").unwrap_or(path))
    }
}

/// The repository and number of a PR given as a number, read in `--repo` or the
/// current directory's repository, or as a URL, which names its own repository
fn pr_target(pr: &str, repo: Option<&str>) -> Result<(Repo, u64)> {
    if let Ok(number) = pr.parse::<u64>() {
        return Ok((Repo::resolve(repo)?, number));
    }

    let (owner, name, number) = github::parse_url(pr)?;
    Ok((Repo { owner, name }, number))
}

fn git(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .context("could not run git")?;

    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn short(sha: &str) -> &str {
    sha.get(..7).unwrap_or(sha)
}

/// Decides which failed polls to wait out. A wait can span hours, so GitHub
/// outages, rate limits, and network errors are retried for as long as the task
/// runs, while a refusal, such as a bad token, a missing PR, or a token without
/// access, ends it at once
struct Retry {
    failing: bool,
}

impl Retry {
    fn new() -> Self {
        Self { failing: false }
    }

    fn succeeded(&mut self) {
        if self.failing {
            eprintln!("GitHub answered again");
        }

        self.failing = false;
    }

    /// Record a failed poll; returns the error when retrying cannot help
    fn absorb(&mut self, error: eyre::Report) -> Result<(), eyre::Report> {
        if is_permanent(&error) {
            return Err(error);
        }

        // one line per outage, not one per poll
        if !self.failing {
            eprintln!(
                "poll failed, retrying until GitHub answers: {}",
                one_line(&error)
            );
        }

        self.failing = true;
        Ok(())
    }
}

fn is_permanent(error: &eyre::Report) -> bool {
    let http = error
        .downcast_ref::<ApiStatusError>()
        .is_some_and(ApiStatusError::is_refusal);
    let graphql = error
        .downcast_ref::<GraphqlQueryError>()
        .is_some_and(|error| error.refused);

    http || graphql
}

/// An error as one bounded line, since callers read only the last lines of the
/// log and an API error can carry a multi-line body or an HTML page
fn one_line(error: &eyre::Report) -> String {
    const MAX_CHARS: usize = 300;

    let text = format!("{error:#}");
    let line = text.lines().next().unwrap_or_default();
    match line.char_indices().nth(MAX_CHARS) {
        Some((end, _)) => format!("{}...", &line[..end]),
        None => line.to_string(),
    }
}
