//! `cmd ship watch-run`: wait for the GitHub Actions run of one workflow on one
//! commit, such as a release
//!
//! `gh run list --limit 1` right after a push or tag can return the previous run,
//! which makes a deploy install the old version, so the run is matched to the commit

use std::process::Command;
use std::time::Duration;

use clap::Args;
use eyre::{bail, Context, Result};

use super::{one_line, Repo, Report, Retry};
use crate::github::ci::{RunConclusion, WorkflowRun};
use crate::github::Github;

/// A new run takes a few seconds to register after a push; give it 10 minutes
/// of answered polls, so an outage does not use them up
const FIND_POLL: Duration = Duration::from_secs(10);
const FIND_POLLS: u32 = 60;

const RUN_POLL: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Args)]
pub struct WatchRunArgs {
    /// Workflow name, such as `Release`, or file, such as `release.yml`
    workflow: String,

    /// Commit, branch, or tag the run is for, resolved in the current repository
    rev: String,

    /// Repository as owner/name [default: the current directory's]
    #[arg(long)]
    repo: Option<String>,
}

pub(super) async fn run(args: WatchRunArgs) -> Result<Report> {
    let repo = Repo::resolve(args.repo.as_deref())?;
    let sha = rev_parse(&args.rev)?;
    let github = Github::new(None)?;
    let workflow = args.workflow;

    let Some(id) = find_run(&github, &repo, &workflow, &sha).await? else {
        return Ok(Report::new(
            format!("{workflow}: no run for {sha} after 10 minutes"),
            false,
        ));
    };

    let run = wait_for_completion(&github, &repo, id).await?;
    let url = &run.html_url;
    // only success counts: a run whose jobs were all skipped released nothing
    if run.conclusion == Some(RunConclusion::Success) {
        return Ok(Report::new(
            format!("{workflow} run {id}: success {url}"),
            true,
        ));
    }

    let failed = github
        .fetch_failed_jobs(&repo.owner, &repo.name, id)
        .await
        .map(|jobs| jobs.join(", "))
        .unwrap_or_else(|error| format!("could not list jobs: {}", one_line(&error)));
    let conclusion = run.conclusion.map_or("unknown", |c| c.as_str());

    Ok(Report::new(
        format!("{workflow} run {id}: {conclusion} ({failed}) {url}"),
        false,
    ))
}

/// The full commit sha of `rev`, peeling an annotated tag to its commit
fn rev_parse(rev: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", &format!("{rev}^{{commit}}")])
        .output()
        .context("could not run git rev-parse")?;

    if !output.status.success() {
        bail!(
            "git rev-parse {rev} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

async fn find_run(github: &Github, repo: &Repo, workflow: &str, sha: &str) -> Result<Option<u64>> {
    let mut errors = Retry::new();
    let mut answered = 0;

    while answered < FIND_POLLS {
        match github
            .fetch_workflow_runs(&repo.owner, &repo.name, sha)
            .await
        {
            Ok(runs) => {
                if let Some(run) = run_to_watch(&runs, workflow) {
                    return Ok(Some(run.id));
                }

                errors.succeeded();
                answered += 1;
            }
            Err(error) => errors.absorb(error)?,
        }

        tokio::time::sleep(FIND_POLL).await;
    }

    Ok(None)
}

async fn wait_for_completion(github: &Github, repo: &Repo, id: u64) -> Result<WorkflowRun> {
    let mut errors = Retry::new();

    loop {
        match github.fetch_workflow_run(&repo.owner, &repo.name, id).await {
            Ok(run) if run.is_completed() => return Ok(run),
            Ok(_) => errors.succeeded(),
            Err(error) => errors.absorb(error)?,
        }

        tokio::time::sleep(RUN_POLL).await;
    }
}

/// The run of `workflow` to wait for, matched by display name or by workflow
/// path, file, or file stem. A run still going wins over a newer finished one,
/// since a rerun keeps its old id and the release it retries is not out yet
fn run_to_watch<'a>(runs: &'a [WorkflowRun], workflow: &str) -> Option<&'a WorkflowRun> {
    runs.iter()
        .filter(|run| {
            // a reusable or remote workflow's path ends in `@<ref>`
            let path = run
                .path
                .split_once('@')
                .map_or(run.path.as_str(), |(path, _)| path);
            let file = path.rsplit('/').next().unwrap_or(path);
            let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
            run.name.as_deref() == Some(workflow) || [path, file, stem].contains(&workflow)
        })
        .max_by_key(|run| (!run.is_completed(), run.id))
}

#[cfg(test)]
mod tests {
    use super::run_to_watch;
    use crate::github::ci::{RunStatus, WorkflowRun};

    fn run(id: u64, name: &str, path: &str, status: RunStatus) -> WorkflowRun {
        WorkflowRun {
            id,
            name: Some(name.into()),
            path: path.into(),
            event: "push".into(),
            status,
            conclusion: None,
            html_url: String::new(),
            pull_requests: Vec::new(),
        }
    }

    #[test]
    fn matches_workflow_by_name_or_file_and_takes_the_newest() {
        let runs = [
            run(
                10,
                "Release",
                ".github/workflows/release.yml",
                RunStatus::Completed,
            ),
            run(
                12,
                "Release",
                ".github/workflows/release.yml",
                RunStatus::Completed,
            ),
            run(11, "CI", ".github/workflows/ci.yml", RunStatus::Completed),
            run(
                13,
                "Build",
                ".github/workflows/build.yml@main",
                RunStatus::Completed,
            ),
        ];

        let id = |workflow| run_to_watch(&runs, workflow).map(|run| run.id);

        assert_eq!(id("Release"), Some(12));
        assert_eq!(id("release.yml"), Some(12));
        assert_eq!(id(".github/workflows/release.yml"), Some(12));
        assert_eq!(id("release"), Some(12));
        assert_eq!(id("ci"), Some(11));
        assert_eq!(id("build.yml"), Some(13));
        assert_eq!(id("Deploy"), None);
    }

    #[test]
    fn rerun_of_an_older_run_wins_over_a_newer_finished_one() {
        let runs = [
            run(
                10,
                "Release",
                ".github/workflows/release.yml",
                RunStatus::InProgress,
            ),
            run(
                11,
                "Release",
                ".github/workflows/release.yml",
                RunStatus::Completed,
            ),
        ];

        assert_eq!(run_to_watch(&runs, "Release").map(|run| run.id), Some(10));
    }
}
