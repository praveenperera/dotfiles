//! `cmd ship ci-failures`: report a PR's failing checks now, with a snippet of
//! each failed GitHub Actions job's log, for the gh-fix-ci skill
//!
//! A check outside GitHub Actions, such as a commit status or another app's
//! check run, is reported with its link only, since its logs live elsewhere

use std::io::Write as _;

use clap::builder::RangedU64ValueParser;
use clap::Args;
use eyre::Result;
use futures::{stream, StreamExt};
use serde::Serialize;

use super::watch_pr::Observation;
use super::{one_line, pr_target, short, Repo};
use crate::github::ci::{Check, RunConclusion, RunStatus, WorkflowRun};
use crate::github::Github;
use crate::runtime;

/// GitHub's error annotation, which it also writes when a step exits non-zero
const ERROR_ANNOTATION: &str = "##[error]";

/// Substrings, matched case-insensitively, that mark a failure in a log whose
/// failing step wrote no error annotation
const FAILURE_MARKERS: [&str; 9] = [
    "error",
    "fail",
    "traceback",
    "exception",
    "assert",
    "panic",
    "fatal",
    "timeout",
    "segmentation fault",
];

/// Job logs fetched at once, since a matrix can fail dozens of jobs
const LOG_FETCHES: usize = 4;

const RULE: &str = "------------------------------------------------------------";

#[derive(Debug, Clone, Args)]
pub struct CiFailuresArgs {
    /// Pull request number or URL
    pr: String,

    /// Repository as owner/name [default: the current directory's]
    #[arg(long)]
    repo: Option<String>,

    /// Most log lines in each snippet and tail
    #[arg(long, default_value_t = 160, value_parser = RangedU64ValueParser::<usize>::new().range(1..))]
    max_lines: usize,

    /// Log lines kept on each side of the line that marks the failure
    #[arg(long, default_value_t = 30, value_parser = RangedU64ValueParser::<usize>::new().range(1..))]
    context: usize,

    /// Print JSON instead of text
    #[arg(long)]
    json: bool,
}

pub(super) fn run(args: CiFailuresArgs) -> Result<()> {
    let (repo, number) = pr_target(&args.pr, args.repo.as_deref())?;
    let github = Github::new(None)?;
    let window = LogWindow {
        max_lines: args.max_lines,
        context: args.context,
    };
    let report = runtime::block_on(inspect(&github, &repo, number, window))??;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", report.render());
    }

    // exit 1 on failures so a script can branch on the result
    if !report.results.is_empty() {
        std::io::stdout().flush()?;
        std::process::exit(1);
    }

    Ok(())
}

/// What failed on a PR's head commit
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FailureReport {
    pr: u64,
    url: String,
    head_sha: String,
    /// Checks and runs still going, whose results the report cannot include
    pending: Vec<String>,
    results: Vec<Failure>,
}

/// One failing check, or a run that failed without a failing check
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Failure {
    name: String,
    details_url: Option<String>,
    run_id: Option<u64>,
    job_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    run: Option<RunSummary>,
    #[serde(flatten)]
    log: LogResult,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum LogResult {
    #[serde(rename_all = "camelCase")]
    Ok {
        log_snippet: String,
        log_tail: String,
    },
    /// Not a GitHub Actions job, so only its link is reported
    External {
        note: String,
    },
    /// A run that failed with no failing job, which leaves no job log
    NoJobs {
        note: String,
    },
    LogUnavailable {
        error: String,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RunSummary {
    workflow_name: String,
    status: RunStatus,
    conclusion: Option<RunConclusion>,
    event: String,
    url: String,
}

impl From<&WorkflowRun> for RunSummary {
    fn from(run: &WorkflowRun) -> Self {
        Self {
            workflow_name: run.display_name().to_string(),
            status: run.status,
            conclusion: run.conclusion,
            event: run.event.clone(),
            url: run.html_url.clone(),
        }
    }
}

async fn inspect(
    github: &Github,
    repo: &Repo,
    number: u64,
    window: LogWindow,
) -> Result<FailureReport> {
    let head = github
        .fetch_pr_head(&repo.owner, &repo.name, number)
        .await?;
    let observation = Observation::fetch(github, repo, number, &head.sha)
        .await?
        .for_pr(number);
    let current = observation.current();

    let summary = |run_id: Option<u64>| {
        let run = current.runs.iter().find(|run| Some(run.id) == run_id)?;
        Some(RunSummary::from(*run))
    };

    let mut results = stream::iter(current.failed_checks())
        .map(|check| inspect_check(github, repo, check, summary(check.run_id), window))
        .buffered(LOG_FETCHES)
        .collect::<Vec<_>>()
        .await;

    results.extend(
        current
            .failed_runs_without_checks()
            .into_iter()
            .map(|run| Failure {
                name: run.display_name().to_string(),
                details_url: Some(run.html_url.clone()),
                run_id: Some(run.id),
                job_id: None,
                run: Some(run.into()),
                log: LogResult::NoJobs {
                    note: "The run failed without a failing job, such as on an invalid workflow file; see its page".into(),
                },
            }),
    );

    Ok(FailureReport {
        pr: number,
        url: head.url,
        head_sha: head.sha,
        pending: current.pending().into_iter().collect(),
        results,
    })
}

async fn inspect_check(
    github: &Github,
    repo: &Repo,
    check: &Check,
    run: Option<RunSummary>,
    window: LogWindow,
) -> Failure {
    let log = match (check.run_id, check.job_id) {
        (None, _) => LogResult::External {
            note: "Not a GitHub Actions job; see its details URL".into(),
        },
        (Some(_), None) => LogResult::LogUnavailable {
            error: "GitHub named no job for this check".into(),
        },
        (Some(_), Some(job)) => match github.fetch_job_log(&repo.owner, &repo.name, job).await {
            Ok(log) => {
                let lines = clean_log(&log);
                LogResult::Ok {
                    log_snippet: window.snippet(&lines).join("\n"),
                    log_tail: window.tail(&lines).join("\n"),
                }
            }
            Err(error) => LogResult::LogUnavailable {
                error: one_line(&error),
            },
        },
    };

    Failure {
        name: check.name.clone(),
        details_url: check.url.clone(),
        run_id: check.run_id,
        job_id: check.job_id,
        run,
        log,
    }
}

impl FailureReport {
    fn render(&self) -> String {
        let mut out = String::new();
        let prefix = format!("PR #{} ({})", self.pr, short(&self.head_sha));
        let url = &self.url;

        match self.results.len() {
            0 => out.push_str(&format!("{prefix}: no failing checks detected {url}\n")),
            1 => out.push_str(&format!("{prefix}: 1 failing check {url}\n")),
            count => out.push_str(&format!("{prefix}: {count} failing checks {url}\n")),
        }

        if !self.pending.is_empty() {
            out.push_str(&format!("Still running: {}\n", self.pending.join(", ")));
        }

        if self.results.is_empty() {
            return out;
        }

        for failure in &self.results {
            out.push_str(RULE);
            out.push('\n');
            failure.render(&mut out);
        }

        out.push_str(RULE);
        out.push('\n');
        out
    }
}

impl Failure {
    fn render(&self, out: &mut String) {
        let mut line = |label: &str, value: &dyn std::fmt::Display| {
            out.push_str(&format!("{label}: {value}\n"));
        };

        line("Check", &self.name);
        if let Some(url) = &self.details_url {
            line("Details", url);
        }

        if let Some(id) = self.run_id {
            line("Run ID", &id);
        }

        if let Some(id) = self.job_id {
            line("Job ID", &id);
        }

        let status = match &self.log {
            LogResult::Ok { .. } => "ok",
            LogResult::External { .. } => "external",
            LogResult::NoJobs { .. } => "no_jobs",
            LogResult::LogUnavailable { .. } => "log_unavailable",
        };
        line("Status", &status);

        if let Some(run) = &self.run {
            let conclusion = run.conclusion.map_or("in progress", RunConclusion::as_str);
            line("Workflow", &format!("{} ({conclusion})", run.workflow_name));
            line("Run URL", &run.url);
        }

        match &self.log {
            LogResult::Ok { log_snippet, .. } if log_snippet.is_empty() => {
                out.push_str("No snippet available: the log is empty\n");
            }
            LogResult::Ok { log_snippet, .. } => {
                out.push_str("Failure snippet:\n");
                for line in log_snippet.lines() {
                    out.push_str(&format!("  {line}\n"));
                }
            }
            LogResult::External { note } | LogResult::NoJobs { note } => {
                out.push_str(&format!("Note: {note}\n"));
            }
            LogResult::LogUnavailable { error } => {
                out.push_str(&format!("Error fetching logs: {error}\n"));
            }
        }
    }
}

/// How much of a job log to keep
#[derive(Debug, Clone, Copy)]
struct LogWindow {
    max_lines: usize,
    context: usize,
}

impl LogWindow {
    /// The lines around the failure: `context` lines on each side of the last
    /// error annotation, else of the last line with a failure marker, else the
    /// log's tail, keeping the last `max_lines`. The annotation wins because a
    /// post-job step can log a harmless marker after the failure, such as
    /// actions/cache's "Failed to save" warning
    fn snippet<'a>(&self, lines: &'a [String]) -> &'a [String] {
        let anchor = lines
            .iter()
            .rposition(|line| line.starts_with(ERROR_ANNOTATION))
            .or_else(|| lines.iter().rposition(|line| has_failure_marker(line)));
        let Some(anchor) = anchor else {
            return self.tail(lines);
        };

        let start = anchor.saturating_sub(self.context);
        let end = (anchor + self.context + 1).min(lines.len());
        self.tail(&lines[start..end])
    }

    fn tail<'a>(&self, lines: &'a [String]) -> &'a [String] {
        &lines[lines.len().saturating_sub(self.max_lines)..]
    }
}

fn has_failure_marker(line: &str) -> bool {
    let line = line.to_lowercase();
    FAILURE_MARKERS.iter().any(|marker| line.contains(marker))
}

/// A job log's lines without the byte order mark it starts with, the timestamp
/// GitHub puts on each line, or ANSI color codes, which only cost tokens
fn clean_log(log: &str) -> Vec<String> {
    log.trim_start_matches('\u{feff}')
        .lines()
        .map(|line| strip_ansi(strip_timestamp(line)))
        .collect()
}

/// `line` without a leading `2025-06-01T12:34:56.1234567Z ` timestamp
fn strip_timestamp(line: &str) -> &str {
    let (stamp, rest) = line.split_once(' ').unwrap_or((line, ""));
    let bytes = stamp.as_bytes();
    let is_stamp = bytes.len() >= 20
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[10] == b'T'
        && stamp.ends_with('Z');

    if is_stamp {
        rest
    } else {
        line
    }
}

/// `line` without ANSI escape sequences, such as color codes
fn strip_ansi(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars();

    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
            continue;
        }

        // a CSI sequence runs from `ESC [` to its final byte, `@` through `~`
        if chars.clone().next() == Some('[') {
            chars.next();
            for c in chars.by_ref() {
                if ('@'..='~').contains(&c) {
                    break;
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::{
        matchers::{body_partial_json, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use super::{inspect, LogWindow};
    use crate::cmd::ship::Repo;
    use crate::github::Github;

    const SHA: &str = "1f975f05498f59a5cd76ab1a706e5a526702a0a2";
    const PR_URL: &str = "https://github.com/o/r/pull/7";

    fn check_run(
        name: &str,
        status: &str,
        conclusion: Option<&str>,
        job: u64,
        run: u64,
    ) -> serde_json::Value {
        json!({
            "__typename": "CheckRun", "name": name, "status": status, "conclusion": conclusion,
            "databaseId": job,
            "detailsUrl": format!("https://github.com/o/r/actions/runs/{run}/job/{job}"),
            "checkSuite": {"workflowRun": {"databaseId": run}}
        })
    }

    fn workflow_run(
        id: u64,
        name: &str,
        file: &str,
        status: &str,
        conclusion: Option<&str>,
    ) -> serde_json::Value {
        json!({
            "id": id, "name": name, "path": format!(".github/workflows/{file}"),
            "event": "pull_request", "status": status, "conclusion": conclusion,
            "html_url": format!("https://github.com/o/r/actions/runs/{id}"),
            "pull_requests": [{"number": 7}]
        })
    }

    #[tokio::test]
    async fn reports_failed_jobs_with_snippets_and_external_checks_by_link() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/graphql"))
            .and(body_partial_json(
                json!({"operationName": "PullRequestHead"}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {"repository": {"pullRequest": {"url": PR_URL, "headRefOid": SHA}}}
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/graphql"))
            .and(body_partial_json(json!({"operationName": "CommitChecks"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {"repository": {
                    "pullRequest": {"headRefOid": SHA},
                    "object": {"statusCheckRollup": {"contexts": {
                        "nodes": [
                            // the attempt a newer run of the same workflow replaced
                            check_run("test (ubuntu)", "COMPLETED", Some("CANCELLED"), 191, 90),
                            check_run("test (ubuntu)", "COMPLETED", Some("FAILURE"), 201, 100),
                            check_run("lint", "COMPLETED", Some("SUCCESS"), 202, 100),
                            check_run("test (macos)", "COMPLETED", Some("TIMED_OUT"), 203, 100),
                            check_run("build", "IN_PROGRESS", None, 204, 100),
                            {"__typename": "CheckRun", "name": "Vercel", "status": "COMPLETED",
                             "conclusion": "FAILURE", "databaseId": 999,
                             "detailsUrl": "https://github.com/o/r/runs/999",
                             "checkSuite": {"workflowRun": null}},
                            {"__typename": "StatusContext", "context": "ci/circleci",
                             "state": "ERROR", "targetUrl": "https://circleci.com/gh/o/r/1"}
                        ],
                        "pageInfo": {"hasNextPage": false, "endCursor": null}
                    }}}
                }}
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/repos/o/r/actions/runs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "total_count": 3,
                "workflow_runs": [
                    workflow_run(90, "CI", "ci.yml", "completed", Some("cancelled")),
                    // a failed job's run can still be going
                    workflow_run(100, "CI", "ci.yml", "in_progress", None),
                    workflow_run(110, "Lint", "lint.yml", "completed", Some("startup_failure")),
                ]
            })))
            .mount(&server)
            .await;
        let log = [
            "\u{feff}2026-11-03T15:00:00.0000000Z ##[group]Run cargo test",
            "2026-11-03T15:00:01.0000000Z running 2 tests",
            "2026-11-03T15:00:02.0000000Z test ok_case ... ok",
            "2026-11-03T15:00:03.0000000Z test parses_pr ... \u{1b}[31mFAILED\u{1b}[0m",
            "2026-11-03T15:00:04.0000000Z thread 'parses_pr' panicked at src/lib.rs:9:5",
            "2026-11-03T15:00:05.0000000Z ##[error]Process completed with exit code 101.",
            "2026-11-03T15:00:06.0000000Z Post job cleanup.",
            "2026-11-03T15:00:07.0000000Z Warning: Failed to save: cache entry exists",
        ]
        .join("\n");
        Mock::given(method("GET"))
            .and(path("/repos/o/r/actions/jobs/201/logs"))
            .respond_with(ResponseTemplate::new(200).set_body_string(log))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/repos/o/r/actions/jobs/203/logs"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&server)
            .await;
        let github = Github::with_base_url(server.uri(), None).unwrap();
        let repo = Repo {
            owner: "o".into(),
            name: "r".into(),
        };
        let window = LogWindow {
            max_lines: 4,
            context: 2,
        };

        let report = inspect(&github, &repo, 7, window).await.unwrap();

        let ci_run = json!({
            "workflowName": "CI", "status": "in_progress", "conclusion": null,
            "event": "pull_request", "url": "https://github.com/o/r/actions/runs/100"
        });
        assert_eq!(
            serde_json::to_value(&report).unwrap(),
            json!({
                "pr": 7,
                "url": PR_URL,
                "headSha": SHA,
                "pending": ["CI", "build"],
                "results": [
                    {
                        "name": "test (ubuntu)",
                        "detailsUrl": "https://github.com/o/r/actions/runs/100/job/201",
                        "runId": 100,
                        "jobId": 201,
                        "run": ci_run,
                        "status": "ok",
                        "logSnippet": "thread 'parses_pr' panicked at src/lib.rs:9:5\n##[error]Process completed with exit code 101.\nPost job cleanup.\nWarning: Failed to save: cache entry exists",
                        "logTail": "thread 'parses_pr' panicked at src/lib.rs:9:5\n##[error]Process completed with exit code 101.\nPost job cleanup.\nWarning: Failed to save: cache entry exists"
                    },
                    {
                        "name": "test (macos)",
                        "detailsUrl": "https://github.com/o/r/actions/runs/100/job/203",
                        "runId": 100,
                        "jobId": 203,
                        "run": ci_run,
                        "status": "log_unavailable",
                        "error": "Failed to fetch the log of job 203 of o/r: GitHub API request failed with status 404 Not Found: Not Found"
                    },
                    {
                        "name": "Vercel",
                        "detailsUrl": "https://github.com/o/r/runs/999",
                        "runId": null,
                        "jobId": null,
                        "status": "external",
                        "note": "Not a GitHub Actions job; see its details URL"
                    },
                    {
                        "name": "ci/circleci",
                        "detailsUrl": "https://circleci.com/gh/o/r/1",
                        "runId": null,
                        "jobId": null,
                        "status": "external",
                        "note": "Not a GitHub Actions job; see its details URL"
                    },
                    {
                        "name": "Lint",
                        "detailsUrl": "https://github.com/o/r/actions/runs/110",
                        "runId": 110,
                        "jobId": null,
                        "run": {
                            "workflowName": "Lint", "status": "completed", "conclusion": "startup_failure",
                            "event": "pull_request", "url": "https://github.com/o/r/actions/runs/110"
                        },
                        "status": "no_jobs",
                        "note": "The run failed without a failing job, such as on an invalid workflow file; see its page"
                    }
                ]
            })
        );

        // the window around the error annotation, not the cache warning after it
        let wide = LogWindow {
            max_lines: 160,
            context: 2,
        };
        let report = inspect(&github, &repo, 7, wide).await.unwrap();
        let text = report.render();

        assert!(text.starts_with(&format!(
            "PR #7 (1f975f0): 5 failing checks {PR_URL}\nStill running: CI, build\n"
        )));
        assert!(text.contains(
            "Failure snippet:\n  test parses_pr ... FAILED\n  thread 'parses_pr' panicked at src/lib.rs:9:5\n  ##[error]Process completed with exit code 101.\n  Post job cleanup.\n  Warning: Failed to save: cache entry exists\n"
        ));
    }
}
