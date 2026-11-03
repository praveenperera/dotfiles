//! `cmd ship watch-pr`: wait for a PR's CI on its head commit, and optionally for
//! Greptile's review of that commit

use std::collections::{BTreeSet, HashMap, HashSet};
use std::time::Duration;

use clap::Args;
use eyre::Result;

use super::{one_line, short, Repo, Report, Retry};
use crate::github::ci::{Check, CheckState, GreptileReview, WorkflowRun};
use crate::github::{self, Github};

#[derive(Debug, Clone, Args)]
pub struct WatchPrArgs {
    /// Pull request number or URL
    pr: String,

    /// Also wait for Greptile's review of the head commit, and pass only on 5/5
    #[arg(long)]
    greptile: bool,

    /// Repository as owner/name [default: the current directory's]
    #[arg(long)]
    repo: Option<String>,
}

pub(super) async fn run(args: WatchPrArgs) -> Result<Report> {
    let (repo, number) = match args.pr.parse::<u64>() {
        Ok(number) => (Repo::resolve(args.repo.as_deref())?, number),
        Err(_) => {
            let (owner, name, number) = github::parse_url(&args.pr)?;
            (Repo { owner, name }, number)
        }
    };

    let github = Github::new(None)?;
    let timing = Timing::GITHUB;

    // an outage when the task starts should not end a wait meant to last hours
    let mut errors = Retry::new();
    let head = loop {
        match github.fetch_pr_head(&repo.owner, &repo.name, number).await {
            Ok(head) => break head,
            Err(error) => errors.absorb(error)?,
        }

        tokio::time::sleep(timing.poll).await;
    };
    let source = GithubPr {
        github,
        repo,
        number,
    };
    let watch = PrWatch {
        source,
        number,
        url: head.url,
        sha: head.sha,
        timing,
    };

    Ok(watch.run(args.greptile).await)
}

/// Poll cadence and patience, in polls so tests can run them without waiting
#[derive(Debug, Clone, Copy)]
struct Timing {
    poll: Duration,
    /// Polls in a row that must agree CI has ended before it counts. While a job
    /// is rerun, its old attempt can leave the rollup before the new one appears,
    /// so a single quiet poll can be a false pass
    settle_polls: u32,
    /// Polls without any check or run before the PR counts as having no CI, since
    /// checks take a few seconds to register after a push
    empty_polls: u32,
    /// Answered polls to wait for Greptile's review once CI has ended
    greptile_polls: u32,
}

impl Timing {
    /// 30-second polls: about a minute to settle, 10 minutes to decide there is
    /// no CI, and 1 hour for Greptile
    const GITHUB: Self = Self {
        poll: Duration::from_secs(30),
        settle_polls: 3,
        empty_polls: 20,
        greptile_polls: 120,
    };
}

/// One poll of a PR: the head the PR points at now, and the CI on the commit
/// being watched
#[derive(Debug, Clone)]
struct Observation {
    head_sha: String,
    checks: Vec<Check>,
    runs: Vec<WorkflowRun>,
}

trait PrSource {
    async fn observe(&self, sha: &str) -> Result<Observation>;
    async fn greptile(&self, sha: &str) -> Result<GreptileReview>;
}

struct GithubPr {
    github: Github,
    repo: Repo,
    number: u64,
}

impl PrSource for GithubPr {
    async fn observe(&self, sha: &str) -> Result<Observation> {
        let Repo { owner, name } = &self.repo;
        let (commit, runs) = tokio::try_join!(
            self.github
                .fetch_commit_checks(owner, name, self.number, sha),
            self.github.fetch_workflow_runs(owner, name, sha),
        )?;

        Ok(Observation {
            head_sha: commit.head_sha,
            checks: commit.checks,
            runs,
        })
    }

    async fn greptile(&self, sha: &str) -> Result<GreptileReview> {
        let Repo { owner, name } = &self.repo;
        self.github
            .fetch_greptile_review(owner, name, self.number, sha)
            .await
    }
}

/// What one poll says about CI
#[derive(Debug, Clone, PartialEq, Eq)]
enum CiState {
    /// No check or run exists yet
    Empty,
    /// Names of the checks and runs still going
    Pending(BTreeSet<String>),
    /// Everything ended; names of what did not pass
    Finished(Vec<String>),
}

impl Observation {
    /// Drop the runs, and their checks, that belong to another PR with the same
    /// head commit, such as one opened from the same branch against another base.
    /// A fork's PR runs list no PRs, so they stay
    fn for_pr(mut self, number: u64) -> Self {
        let foreign = self
            .runs
            .iter()
            .filter(|run| {
                !run.pull_requests.is_empty()
                    && !run.pull_requests.iter().any(|pr| pr.number == number)
            })
            .map(|run| run.id)
            .collect::<HashSet<_>>();

        self.runs.retain(|run| !foreign.contains(&run.id));
        self.checks
            .retain(|check| !check.run_id.is_some_and(|id| foreign.contains(&id)));
        self
    }

    fn assess(&self) -> CiState {
        let (current, superseded) = split_runs(&self.runs);

        // checks of a run that a newer run of the same workflow replaced, such as
        // one cancelled by a concurrency group, no longer decide anything
        let checks = self
            .checks
            .iter()
            .filter(|check| !check.run_id.is_some_and(|id| superseded.contains(&id)))
            .collect::<Vec<_>>();

        // a rerun job is queued on its run before its check reappears in the rollup
        let pending = checks
            .iter()
            .filter(|check| check.state == CheckState::Pending)
            .map(|check| check.name.clone())
            .chain(
                current
                    .iter()
                    .filter(|run| !run.is_completed())
                    .map(|run| run.display_name().to_string()),
            )
            .collect::<BTreeSet<_>>();

        if !pending.is_empty() {
            return CiState::Pending(pending);
        }

        if checks.is_empty() && current.is_empty() {
            return CiState::Empty;
        }

        let failed_checks = checks
            .iter()
            .filter(|check| check.state == CheckState::Failed)
            .collect::<Vec<_>>();

        // a run can fail before any job starts, such as on an invalid workflow
        // file, which leaves no failed check to name
        let failed_runs = current
            .iter()
            .filter(|run| run.passed() == Some(false))
            .filter(|run| {
                !failed_checks
                    .iter()
                    .any(|check| check.run_id == Some(run.id))
            })
            .map(|run| {
                let conclusion = run.conclusion.map_or("unknown", |c| c.as_str());
                format!("{} ({conclusion})", run.display_name())
            });

        let failed = failed_checks
            .iter()
            .map(|check| check.name.clone())
            .chain(failed_runs)
            .collect();

        CiState::Finished(failed)
    }
}

/// The newest run of each workflow and event, and the ids of the older runs
/// they replace
fn split_runs(runs: &[WorkflowRun]) -> (Vec<&WorkflowRun>, HashSet<u64>) {
    let mut newest: HashMap<(&str, &str), &WorkflowRun> = HashMap::new();
    for run in runs {
        let key = (run.path.as_str(), run.event.as_str());
        let entry = newest.entry(key).or_insert(run);
        if run.id > entry.id {
            *entry = run;
        }
    }

    // by id, so failure names come out in the same order on every poll
    let mut current = newest.into_values().collect::<Vec<_>>();
    current.sort_by_key(|run| run.id);

    let superseded = runs
        .iter()
        .map(|run| run.id)
        .filter(|id| !current.iter().any(|run| run.id == *id))
        .collect();

    (current, superseded)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CiResult {
    NoChecks,
    Passed,
    Failed(Vec<String>),
}

impl CiResult {
    fn passed(&self) -> bool {
        !matches!(self, Self::Failed(_))
    }

    fn describe(&self) -> String {
        match self {
            Self::NoChecks => "no CI checks".to_string(),
            Self::Passed => "CI passed".to_string(),
            Self::Failed(names) => format!("CI failed ({})", names.join(", ")),
        }
    }
}

struct GreptileWait {
    score: Option<u8>,
    /// Whether the wait slept, so CI may have changed since it was read
    waited: bool,
}

/// Why a wait ended before it had a result
enum Stop {
    /// A push moved the head, which makes the checks and review being waited on stale
    HeadMoved(String),
    /// GitHub refused the request in a way retrying cannot fix
    Refused(eyre::Report),
}

struct PrWatch<S> {
    source: S,
    number: u64,
    url: String,
    sha: String,
    timing: Timing,
}

impl<S: PrSource> PrWatch<S> {
    async fn run(&self, greptile: bool) -> Report {
        let ci = match self.wait_for_ci().await {
            Ok(ci) => ci,
            Err(stop) => return self.stopped(stop),
        };

        if !greptile {
            return self.report(&ci, None);
        }

        let review = match self.wait_for_greptile().await {
            Ok(review) => review,
            Err(stop) => return self.stopped(stop),
        };

        // a job rerun while Greptile was reviewing changes the CI result, so a
        // wait that slept reads CI again rather than reusing the old verdict
        let ci = match review.waited {
            false => ci,
            true => match self.wait_for_ci().await {
                Ok(ci) => ci,
                Err(stop) => return self.stopped(stop),
            },
        };

        self.report(&ci, Some(review.score))
    }

    fn report(&self, ci: &CiResult, greptile: Option<Option<u8>>) -> Report {
        let prefix = format!("PR #{} ({})", self.number, short(&self.sha));
        let url = &self.url;
        let ci_line = ci.describe();

        match greptile {
            None => Report::new(format!("{prefix}: {ci_line} {url}"), ci.passed()),
            Some(Some(score)) => Report::new(
                format!("{prefix}: {ci_line}; Greptile {score}/5 {url}"),
                ci.passed() && score == 5,
            ),
            Some(None) => Report::new(
                format!(
                    "{prefix}: {ci_line}; no Greptile review of {} after 1 hour {url}",
                    short(&self.sha)
                ),
                false,
            ),
        }
    }

    async fn wait_for_ci(&self) -> Result<CiResult, Stop> {
        let mut errors = Retry::new();
        let mut settled = 0;
        let mut empty = 0;
        let mut last_pending = BTreeSet::new();

        loop {
            let observation = match self.source.observe(&self.sha).await {
                Ok(observation) => observation,
                Err(error) => {
                    errors.absorb(error).map_err(Stop::Refused)?;
                    self.sleep().await;
                    continue;
                }
            };

            errors.succeeded();
            if observation.head_sha != self.sha {
                return Err(Stop::HeadMoved(observation.head_sha));
            }

            match observation.for_pr(self.number).assess() {
                CiState::Pending(pending) => {
                    settled = 0;
                    empty = 0;
                    if pending != last_pending {
                        eprintln!("waiting on {}", join(&pending));
                        last_pending = pending;
                    }
                }

                CiState::Empty => {
                    settled = 0;
                    empty += 1;
                    if empty >= self.timing.empty_polls {
                        return Ok(CiResult::NoChecks);
                    }
                }

                CiState::Finished(failed) => {
                    empty = 0;
                    settled += 1;
                    if settled >= self.timing.settle_polls {
                        return Ok(match failed.is_empty() {
                            true => CiResult::Passed,
                            false => CiResult::Failed(failed),
                        });
                    }
                }
            }

            self.sleep().await;
        }
    }

    /// Poll for Greptile's score; failed polls do not count toward its hour,
    /// so an outage cannot end the wait early
    async fn wait_for_greptile(&self) -> Result<GreptileWait, Stop> {
        let mut errors = Retry::new();
        let mut answered = 0;
        let mut waited = false;

        while answered < self.timing.greptile_polls {
            match self.source.greptile(&self.sha).await {
                Ok(review) if review.head_sha != self.sha => {
                    return Err(Stop::HeadMoved(review.head_sha));
                }
                Ok(GreptileReview {
                    score: Some(score), ..
                }) => {
                    return Ok(GreptileWait {
                        score: Some(score),
                        waited,
                    })
                }
                Ok(_) => {
                    errors.succeeded();
                    answered += 1;
                }
                Err(error) => errors.absorb(error).map_err(Stop::Refused)?,
            }

            self.sleep().await;
            waited = true;
        }

        Ok(GreptileWait {
            score: None,
            waited,
        })
    }

    fn stopped(&self, stop: Stop) -> Report {
        let url = &self.url;
        let number = self.number;
        let line = match stop {
            Stop::HeadMoved(now) => format!(
                "PR #{number}: head moved from {} to {} while waiting; resubmit for the new head {url}",
                short(&self.sha),
                short(&now)
            ),
            Stop::Refused(error) => format!(
                "PR #{number} ({}): gave up, {} {url}",
                short(&self.sha),
                one_line(&error)
            ),
        };

        Report::new(line, false)
    }

    async fn sleep(&self) {
        tokio::time::sleep(self.timing.poll).await;
    }
}

fn join(names: &BTreeSet<String>) -> String {
    names.iter().cloned().collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::time::Duration;

    use eyre::{eyre, Result};
    use reqwest::StatusCode;

    use super::{Observation, PrSource, PrWatch, Timing};
    use crate::cmd::ship::Report;
    use crate::github::ci::{
        Check, CheckState, GreptileReview, RunConclusion, RunPullRequest, RunStatus, WorkflowRun,
    };
    use crate::github::ApiStatusError;

    const SHA: &str = "1f975f05498f59a5cd76ab1a706e5a526702a0a2";
    const URL: &str = "https://github.com/o/r/pull/978";

    const FAST: Timing = Timing {
        poll: Duration::ZERO,
        settle_polls: 3,
        empty_polls: 4,
        greptile_polls: 3,
    };

    /// Replays scripted polls in order and repeats the last one once they run out
    struct Script {
        polls: RefCell<VecDeque<Result<Observation>>>,
        last: RefCell<Option<Observation>>,
        greptile: RefCell<VecDeque<GreptileReview>>,
    }

    impl Script {
        fn new(polls: Vec<Result<Observation>>) -> Self {
            Self {
                polls: RefCell::new(polls.into()),
                last: RefCell::new(None),
                greptile: RefCell::new(VecDeque::new()),
            }
        }

        fn with_greptile(self, reviews: Vec<GreptileReview>) -> Self {
            *self.greptile.borrow_mut() = reviews.into();
            self
        }
    }

    impl PrSource for Script {
        async fn observe(&self, _sha: &str) -> Result<Observation> {
            let Some(poll) = self.polls.borrow_mut().pop_front() else {
                return self
                    .last
                    .borrow()
                    .clone()
                    .ok_or_else(|| eyre!("script is empty"));
            };

            if let Ok(observation) = &poll {
                *self.last.borrow_mut() = Some(observation.clone());
            }

            poll
        }

        async fn greptile(&self, _sha: &str) -> Result<GreptileReview> {
            Ok(self
                .greptile
                .borrow_mut()
                .pop_front()
                .unwrap_or(GreptileReview {
                    head_sha: SHA.into(),
                    score: None,
                }))
        }
    }

    fn check(name: &str, state: CheckState, run_id: u64) -> Check {
        Check {
            name: name.into(),
            state,
            run_id: Some(run_id),
        }
    }

    fn run(
        id: u64,
        name: &str,
        status: RunStatus,
        conclusion: Option<RunConclusion>,
    ) -> WorkflowRun {
        WorkflowRun {
            id,
            name: Some(name.into()),
            path: format!(
                ".github/workflows/{}.yml",
                name.to_lowercase().replace(' ', "-")
            ),
            event: "pull_request".into(),
            status,
            conclusion,
            html_url: format!("https://github.com/o/r/actions/runs/{id}"),
            pull_requests: vec![RunPullRequest { number: 978 }],
        }
    }

    fn poll(checks: Vec<Check>, runs: Vec<WorkflowRun>) -> Result<Observation> {
        Ok(Observation {
            head_sha: SHA.into(),
            checks,
            runs,
        })
    }

    async fn watch(script: Script, greptile: bool) -> Report {
        PrWatch {
            source: script,
            number: 978,
            url: URL.into(),
            sha: SHA.into(),
            timing: FAST,
        }
        .run(greptile)
        .await
    }

    fn done(id: u64, name: &str) -> WorkflowRun {
        run(id, name, RunStatus::Completed, Some(RunConclusion::Success))
    }

    #[tokio::test]
    async fn rerun_gap_does_not_pass_ci() {
        // polls 1-2: the rerun ios-ui check has left the rollup and its run still
        // reads completed, so everything listed passed; poll 3: the run is queued
        // again; then the new attempt fails
        let passed_only = || {
            poll(
                vec![check("clippy", CheckState::Passed, 1)],
                vec![done(1, "CI"), done(2, "UI tests")],
            )
        };
        let script = Script::new(vec![
            passed_only(),
            passed_only(),
            poll(
                vec![check("clippy", CheckState::Passed, 1)],
                vec![done(1, "CI"), run(2, "UI tests", RunStatus::Queued, None)],
            ),
            poll(
                vec![
                    check("clippy", CheckState::Passed, 1),
                    check("ios-ui", CheckState::Failed, 2),
                ],
                vec![
                    done(1, "CI"),
                    run(
                        2,
                        "UI tests",
                        RunStatus::Completed,
                        Some(RunConclusion::Failure),
                    ),
                ],
            ),
        ]);

        let report = watch(script, false).await;

        assert_eq!(
            report,
            Report::new(
                format!("PR #978 (1f975f0): CI failed (ios-ui) {URL}"),
                false
            )
        );
    }

    #[tokio::test]
    async fn queued_run_without_checks_keeps_ci_pending() {
        let script = Script::new(vec![
            poll(
                vec![check("clippy", CheckState::Passed, 1)],
                vec![done(1, "CI"), run(2, "UI tests", RunStatus::Queued, None)],
            ),
            poll(
                vec![check("clippy", CheckState::Passed, 1)],
                vec![done(1, "CI"), run(2, "UI tests", RunStatus::Queued, None)],
            ),
            poll(
                vec![
                    check("clippy", CheckState::Passed, 1),
                    check("ios-ui", CheckState::Passed, 2),
                ],
                vec![done(1, "CI"), done(2, "UI tests")],
            ),
        ]);

        let report = watch(script, false).await;

        assert_eq!(
            report,
            Report::new(format!("PR #978 (1f975f0): CI passed {URL}"), true)
        );
    }

    #[tokio::test]
    async fn run_that_fails_before_any_job_is_reported() {
        let script = Script::new(vec![poll(
            vec![check("clippy", CheckState::Passed, 1)],
            vec![
                done(1, "CI"),
                run(
                    3,
                    "Lint",
                    RunStatus::Completed,
                    Some(RunConclusion::StartupFailure),
                ),
            ],
        )]);

        let report = watch(script, false).await;

        assert_eq!(
            report,
            Report::new(
                format!("PR #978 (1f975f0): CI failed (Lint (startup_failure)) {URL}"),
                false
            )
        );
    }

    #[tokio::test]
    async fn superseded_run_does_not_fail_ci() {
        // run 4 replaced cancelled run 2 of the same workflow on the same commit
        let script = Script::new(vec![poll(
            vec![
                check("ios-ui", CheckState::Failed, 2),
                check("ios-ui", CheckState::Passed, 4),
            ],
            vec![
                run(
                    2,
                    "UI tests",
                    RunStatus::Completed,
                    Some(RunConclusion::Cancelled),
                ),
                done(4, "UI tests"),
            ],
        )]);

        let report = watch(script, false).await;

        assert_eq!(
            report,
            Report::new(format!("PR #978 (1f975f0): CI passed {URL}"), true)
        );
    }

    #[tokio::test]
    async fn head_move_ends_the_wait() {
        let script = Script::new(vec![Ok(Observation {
            head_sha: "abcdef0123456789".into(),
            checks: vec![],
            runs: vec![],
        })]);

        let report = watch(script, false).await;

        assert_eq!(
            report,
            Report::new(
                format!("PR #978: head moved from 1f975f0 to abcdef0 while waiting; resubmit for the new head {URL}"),
                false
            )
        );
    }

    #[tokio::test]
    async fn no_checks_after_the_grace_period() {
        let report = watch(Script::new(vec![poll(vec![], vec![])]), false).await;

        assert_eq!(
            report,
            Report::new(format!("PR #978 (1f975f0): no CI checks {URL}"), true)
        );
    }

    #[tokio::test]
    async fn outages_are_waited_out_and_bad_credentials_end_the_wait() {
        let passed = || {
            poll(
                vec![check("clippy", CheckState::Passed, 1)],
                vec![done(1, "CI")],
            )
        };
        let api_error = |status, rate_limited, message: &str| {
            Err(eyre::Report::new(ApiStatusError {
                status,
                rate_limited,
                message: message.into(),
            }))
        };

        // an outage and a rate limit longer than any fixed budget, then GitHub
        // answers again
        let mut outage = (0..50)
            .map(|_| api_error(StatusCode::BAD_GATEWAY, false, "502"))
            .collect::<Vec<_>>();
        outage.push(api_error(
            StatusCode::FORBIDDEN,
            true,
            "403: API rate limit exceeded",
        ));
        outage.push(Err(eyre!("connection reset")));
        outage.push(passed());
        let revoked = Script::new(vec![api_error(
            StatusCode::UNAUTHORIZED,
            false,
            "status 401: Bad credentials\nIf this is a private repository, authenticate",
        )]);
        let no_access = Script::new(vec![api_error(
            StatusCode::FORBIDDEN,
            false,
            "status 403: Resource not accessible by integration",
        )]);

        assert!(watch(Script::new(outage), false).await.ok);
        assert_eq!(
            watch(revoked, false).await,
            Report::new(
                format!("PR #978 (1f975f0): gave up, status 401: Bad credentials {URL}"),
                false
            )
        );
        assert!(!watch(no_access, false).await.ok);
    }

    #[tokio::test]
    async fn greptile_needs_five_of_five() {
        let passed = || {
            poll(
                vec![check("clippy", CheckState::Passed, 1)],
                vec![done(1, "CI")],
            )
        };
        let review = |score| GreptileReview {
            head_sha: SHA.into(),
            score,
        };
        let four = Script::new(vec![passed()]).with_greptile(vec![review(Some(4))]);
        let silent = Script::new(vec![passed()]);

        assert_eq!(
            watch(four, true).await,
            Report::new(
                format!("PR #978 (1f975f0): CI passed; Greptile 4/5 {URL}"),
                false
            )
        );
        assert_eq!(
            watch(silent, true).await,
            Report::new(
                format!("PR #978 (1f975f0): CI passed; no Greptile review of 1f975f0 after 1 hour {URL}"),
                false
            )
        );
    }

    #[tokio::test]
    async fn ci_is_read_again_after_waiting_for_greptile() {
        // CI passed and settled, then a job was rerun and failed while Greptile
        // was still reviewing
        let passed = || {
            poll(
                vec![check("ios-ui", CheckState::Passed, 2)],
                vec![done(2, "UI tests")],
            )
        };
        let failed = poll(
            vec![check("ios-ui", CheckState::Failed, 2)],
            vec![run(
                2,
                "UI tests",
                RunStatus::Completed,
                Some(RunConclusion::Failure),
            )],
        );
        let review = |score| GreptileReview {
            head_sha: SHA.into(),
            score,
        };
        let script = Script::new(vec![passed(), passed(), passed(), failed])
            .with_greptile(vec![review(None), review(Some(5))]);

        assert_eq!(
            watch(script, true).await,
            Report::new(
                format!("PR #978 (1f975f0): CI failed (ios-ui); Greptile 5/5 {URL}"),
                false
            )
        );
    }

    #[tokio::test]
    async fn result_stays_on_one_line() {
        let script = Script::new(vec![poll(
            vec![check("test\nmacOS", CheckState::Failed, 1)],
            vec![run(
                1,
                "CI",
                RunStatus::Completed,
                Some(RunConclusion::Failure),
            )],
        )]);

        let report = watch(script, false).await;

        assert_eq!(
            report.line,
            format!("PR #978 (1f975f0): CI failed (test macOS) {URL}")
        );
    }

    #[tokio::test]
    async fn runs_of_another_pr_on_the_same_commit_do_not_count() {
        // PR 979 shares the head commit; its newer run passed while ours failed
        let mut other = done(5, "CI");
        other.pull_requests = vec![RunPullRequest { number: 979 }];
        let script = Script::new(vec![poll(
            vec![
                check("clippy", CheckState::Failed, 1),
                check("clippy", CheckState::Passed, 5),
            ],
            vec![
                run(1, "CI", RunStatus::Completed, Some(RunConclusion::Failure)),
                other,
            ],
        )]);

        let report = watch(script, false).await;

        assert_eq!(
            report,
            Report::new(
                format!("PR #978 (1f975f0): CI failed (clippy) {URL}"),
                false
            )
        );
    }
}
