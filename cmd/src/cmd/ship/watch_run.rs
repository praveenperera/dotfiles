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
use crate::github::ci::{RunConclusion, RunStatus, WorkflowRun};
use crate::github::{ApiStatusError, Github};

#[derive(Debug, Clone, Args)]
pub struct WatchRunArgs {
    /// Workflow name, such as `Release`, or file, such as `release.yml`
    workflow: String,

    /// Commit, branch, or tag the run is for. Without --repo it is resolved in the
    /// current checkout, then on GitHub when the checkout does not have it; with
    /// --repo it is resolved on GitHub, where `HEAD` means the default branch
    rev: String,

    /// Repository as owner/name [default: the current directory's]
    #[arg(long)]
    repo: Option<String>,
}

pub(super) async fn run(args: WatchRunArgs) -> Result<Report> {
    // --repo may name a repository other than the current directory's, whose
    // refs would then resolve to unrelated commits
    let source = match args.repo {
        Some(_) => RevSource::Github,
        None => RevSource::Checkout,
    };
    let watch = RunWatch {
        github: Github::new(None)?,
        repo: Repo::resolve(args.repo.as_deref())?,
        workflow: args.workflow,
        timing: Timing::GITHUB,
    };
    let sha = watch.resolve(&args.rev, source).await?;

    watch.wait(&sha).await
}

/// Where a rev given on the command line is looked up
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RevSource {
    /// The current checkout is the watched repository, so local names such as
    /// `HEAD~1` work; a tag or branch it has not fetched yet is looked up on GitHub
    Checkout,
    /// Only GitHub can say, since the current directory may be another repository
    Github,
}

/// Poll cadence and patience, so tests can run them without waiting
#[derive(Debug, Clone, Copy)]
struct Timing {
    /// Delay between polls while looking up the rev or the run
    find_poll: Duration,
    /// Answered polls without the run before it counts as never starting
    find_polls: u32,
    /// Delay between polls of a run that is still going
    run_poll: Duration,
}

impl Timing {
    /// A new run takes a few seconds to register after a push; give it 10 minutes
    /// of answered polls, so an outage does not use them up
    const GITHUB: Self = Self {
        find_poll: Duration::from_secs(10),
        find_polls: 60,
        run_poll: Duration::from_secs(20),
    };
}

struct RunWatch {
    github: Github,
    repo: Repo,
    workflow: String,
    timing: Timing,
}

impl RunWatch {
    /// The full commit sha of `rev`, peeling an annotated tag to its commit
    async fn resolve(&self, rev: &str, source: RevSource) -> Result<String> {
        if source == RevSource::Checkout {
            match rev_parse(rev) {
                Ok(sha) => return Ok(sha),
                Err(error) => eprintln!("{}; looking {rev} up on GitHub", one_line(&error)),
            }
        }

        let Repo { owner, name } = &self.repo;
        let mut errors = Retry::new();
        loop {
            match self.github.fetch_commit_sha(owner, name, rev).await {
                Ok(sha) => return Ok(sha),
                // GitHub answers an unknown rev with 422, which no retry can fix
                Err(error) if is_unprocessable(&error) => return Err(error),
                Err(error) => errors.absorb(error)?,
            }

            tokio::time::sleep(self.timing.find_poll).await;
        }
    }

    /// Wait for the workflow's run on `sha` to end, and report how it ended
    async fn wait(&self, sha: &str) -> Result<Report> {
        let workflow = &self.workflow;
        let Some(id) = self.find_run(sha).await? else {
            return Ok(Report::new(
                format!("{workflow}: no run for {sha} after 10 minutes"),
                false,
            ));
        };

        let run = self.wait_for_completion(id).await?;
        let attempt = run.run_attempt;
        let url = &run.html_url;
        // only success counts: a run whose jobs were all skipped released nothing
        if run.conclusion == Some(RunConclusion::Success) {
            return Ok(Report::new(
                format!("{workflow} run {id} attempt {attempt}: success {url}"),
                true,
            ));
        }

        let Repo { owner, name } = &self.repo;
        let failed = self
            .github
            .fetch_failed_jobs(owner, name, id, attempt)
            .await
            .map(|jobs| jobs.join(", "))
            .unwrap_or_else(|error| format!("could not list jobs: {}", one_line(&error)));
        let conclusion = run.conclusion.map_or("unknown", |c| c.as_str());

        // the attempt tells this ending apart from a rerun started after it
        Ok(Report::new(
            format!("{workflow} run {id} attempt {attempt}: {conclusion} ({failed}) {url}"),
            false,
        ))
    }

    async fn find_run(&self, sha: &str) -> Result<Option<u64>> {
        let Repo { owner, name } = &self.repo;
        let mut errors = Retry::new();
        let mut answered = 0;

        while answered < self.timing.find_polls {
            match self.github.fetch_workflow_runs(owner, name, sha).await {
                Ok(runs) => {
                    if let Some(run) = run_to_watch(&runs, &self.workflow) {
                        return Ok(Some(run.id));
                    }

                    errors.succeeded();
                    answered += 1;
                }
                Err(error) => errors.absorb(error)?,
            }

            tokio::time::sleep(self.timing.find_poll).await;
        }

        Ok(None)
    }

    /// Poll run `id` until it completes. A job held by an environment's
    /// protection rules, such as a required reviewer, leaves the run `waiting`,
    /// which is not an ending
    async fn wait_for_completion(&self, id: u64) -> Result<WorkflowRun> {
        let Repo { owner, name } = &self.repo;
        let mut errors = Retry::new();
        let mut waiting = false;

        loop {
            match self.github.fetch_workflow_run(owner, name, id).await {
                Ok(run) if run.is_completed() => return Ok(run),
                Ok(run) => {
                    errors.succeeded();

                    let now_waiting = run.status == RunStatus::Waiting;
                    if now_waiting && !waiting {
                        eprintln!(
                            "{} run {id} attempt {} is waiting for an environment approval",
                            self.workflow, run.run_attempt
                        );
                    }

                    waiting = now_waiting;
                }
                Err(error) => errors.absorb(error)?,
            }

            tokio::time::sleep(self.timing.run_poll).await;
        }
    }
}

/// The full commit sha of `rev` in the current checkout
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

fn is_unprocessable(error: &eyre::Report) -> bool {
    error
        .downcast_ref::<ApiStatusError>()
        .is_some_and(|error| error.status == reqwest::StatusCode::UNPROCESSABLE_ENTITY)
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
    use std::time::Duration;

    use serde_json::{json, Value};
    use wiremock::{
        matchers::{header, method, path, query_param},
        Mock, MockServer, ResponseTemplate,
    };

    use super::{run_to_watch, RevSource, RunWatch, Timing};
    use crate::cmd::ship::{Repo, Report};
    use crate::github::ci::{RunStatus, WorkflowRun};
    use crate::github::Github;

    const SHA: &str = "1b5a76f47e17c128a3b5b544e98c901600c36c4b";
    const URL: &str = "https://github.com/o/r/actions/runs/7";

    const FAST: Timing = Timing {
        find_poll: Duration::ZERO,
        find_polls: 3,
        run_poll: Duration::ZERO,
    };

    fn run(id: u64, name: &str, path: &str, status: RunStatus) -> WorkflowRun {
        WorkflowRun {
            id,
            name: Some(name.into()),
            path: path.into(),
            event: "push".into(),
            status,
            conclusion: None,
            run_attempt: 1,
            html_url: String::new(),
            pull_requests: Vec::new(),
        }
    }

    /// Run 7 of the Package workflow on `SHA`, as the runs API spells it
    fn package_run(status: &str, conclusion: Option<&str>) -> Value {
        json!({
            "id": 7, "name": "Package", "path": ".github/workflows/package.yml",
            "event": "push", "status": status, "conclusion": conclusion,
            "run_attempt": 1, "html_url": URL
        })
    }

    async fn mount_runs_list(server: &MockServer, run: Value) {
        Mock::given(method("GET"))
            .and(path("/repos/o/r/actions/runs"))
            .and(query_param("head_sha", SHA))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"total_count": 1, "workflow_runs": [run]})),
            )
            .mount(server)
            .await;
    }

    fn watch(server: &MockServer) -> RunWatch {
        RunWatch {
            github: Github::with_base_url(server.uri(), None).unwrap(),
            repo: Repo::parse("o/r").unwrap(),
            workflow: "Package".into(),
            timing: FAST,
        }
    }

    #[tokio::test]
    async fn approval_gate_is_waited_through() {
        // the Publish to PyPI job waits for the pypi environment's reviewer for
        // three polls, then fails after approval
        let server = MockServer::start().await;
        mount_runs_list(&server, package_run("waiting", None)).await;
        Mock::given(method("GET"))
            .and(path("/repos/o/r/actions/runs/7"))
            .respond_with(ResponseTemplate::new(200).set_body_json(package_run("waiting", None)))
            .up_to_n_times(3)
            .expect(3)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/repos/o/r/actions/runs/7"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(package_run("completed", Some("failure"))),
            )
            .mount(&server)
            .await;
        // only the ended attempt's jobs name the failure; a rerun's queued jobs do not
        Mock::given(method("GET"))
            .and(path("/repos/o/r/actions/runs/7/attempts/1/jobs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "total_count": 2,
                "jobs": [
                    {"name": "Publish to TestPyPI", "conclusion": "success"},
                    {"name": "Publish to PyPI", "conclusion": "failure"}
                ]
            })))
            .mount(&server)
            .await;

        let report = watch(&server).wait(SHA).await.unwrap();

        assert_eq!(
            report,
            Report::new(
                format!("Package run 7 attempt 1: failure (Publish to PyPI) {URL}"),
                false
            )
        );
    }

    #[tokio::test]
    async fn rev_only_on_github_is_resolved_there() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/o/r/commits/v0.8.0-only-on-github"))
            .and(header("Accept", "application/vnd.github.sha"))
            .respond_with(ResponseTemplate::new(200).set_body_string(SHA))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/repos/o/r/commits/v9.9.9"))
            .respond_with(ResponseTemplate::new(422).set_body_json(json!({
                "message": "No commit found for SHA: v9.9.9", "status": "422"
            })))
            .expect(1)
            .mount(&server)
            .await;
        mount_runs_list(&server, package_run("completed", Some("success"))).await;
        Mock::given(method("GET"))
            .and(path("/repos/o/r/actions/runs/7"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(package_run("completed", Some("success"))),
            )
            .mount(&server)
            .await;
        let watch = watch(&server);

        // with --repo the local checkout is never consulted, and without it a tag
        // the checkout has not fetched is looked up on GitHub
        for source in [RevSource::Github, RevSource::Checkout] {
            let sha = watch
                .resolve("v0.8.0-only-on-github", source)
                .await
                .unwrap();
            assert_eq!(sha, SHA);
        }

        let report = watch.wait(SHA).await.unwrap();
        assert_eq!(
            report,
            Report::new(format!("Package run 7 attempt 1: success {URL}"), true)
        );

        // a rev GitHub does not know ends the watch rather than retrying forever
        let unknown = tokio::time::timeout(
            Duration::from_secs(5),
            watch.resolve("v9.9.9", RevSource::Github),
        )
        .await
        .expect("an unknown rev is not retried");
        assert!(format!("{:#}", unknown.unwrap_err()).contains("No commit found for SHA: v9.9.9"));
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
