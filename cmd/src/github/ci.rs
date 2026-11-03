//! CI state of a commit: its check rollup, its GitHub Actions runs, and Greptile's
//! review score, as read by the `cmd ship` watchers

use color_eyre::eyre::{Context, ContextCompat, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;

use super::{pr_variables, pull_request, Connection, Github, RepositoryData};

const PULL_REQUEST_HEAD_QUERY: &str = r#"
query PullRequestHead($owner: String!, $repo: String!, $number: Int!) {
  repository(owner: $owner, name: $repo) {
    pullRequest(number: $number) {
      url
      headRefOid
    }
  }
}
"#;

const COMMIT_CHECKS_QUERY: &str = r#"
query CommitChecks(
  $owner: String!
  $repo: String!
  $number: Int!
  $sha: GitObjectID!
  $contextsCursor: String
) {
  repository(owner: $owner, name: $repo) {
    pullRequest(number: $number) {
      headRefOid
    }
    object(oid: $sha) {
      ... on Commit {
        statusCheckRollup {
          contexts(first: 100, after: $contextsCursor) {
            nodes {
              __typename
              ... on CheckRun {
                name
                status
                conclusion
                databaseId
                detailsUrl
                checkSuite { workflowRun { databaseId } }
              }
              ... on StatusContext {
                context
                state
                targetUrl
              }
            }
            pageInfo { hasNextPage endCursor }
          }
        }
      }
    }
  }
}
"#;

const PULL_REQUEST_COMMENTS_QUERY: &str = r#"
query PullRequestComments($owner: String!, $repo: String!, $number: Int!, $commentsCursor: String) {
  repository(owner: $owner, name: $repo) {
    pullRequest(number: $number) {
      headRefOid
      comments(first: 100, after: $commentsCursor) {
        nodes {
          body
          author { __typename login }
        }
        pageInfo { hasNextPage endCursor }
      }
    }
  }
}
"#;

/// Login of the Greptile app's bot account as GraphQL reports it, without the
/// `[bot]` suffix the REST API adds
const GREPTILE_LOGIN: &str = "greptile-apps";
const GREPTILE_SUMMARY_MARKER: &str = "<!-- greptile_summary -->";
const GREPTILE_SCORE_PREFIX: &str = "<!-- greptile_confidence_score:";
const GREPTILE_REVIEWED_LABEL: &str = "Last reviewed commit";

/// A pull request's URL and the commit its branch points at
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PullRequestHead {
    pub(crate) url: String,
    #[serde(rename = "headRefOid")]
    pub(crate) sha: String,
}

/// The check rollup of one commit, read alongside the PR's current head so a
/// caller can tell when a push made the commit stale
#[derive(Debug, Clone)]
pub(crate) struct CommitChecks {
    pub(crate) head_sha: String,
    pub(crate) checks: Vec<Check>,
}

/// One entry of a commit's check rollup: a check run or a commit status
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Check {
    pub(crate) name: String,
    pub(crate) state: CheckState,
    /// The Actions run that owns this check, when it is an Actions job
    pub(crate) run_id: Option<u64>,
    /// The job behind this check, when it is an Actions job, whose check run id
    /// is its job id
    pub(crate) job_id: Option<u64>,
    /// The page the check links to: the job's log for an Actions job, or an
    /// external provider's page
    pub(crate) url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CheckState {
    Pending,
    Passed,
    Failed,
}

/// The newest Greptile summary of a PR that names a given commit
#[derive(Debug, Clone)]
pub(crate) struct GreptileReview {
    pub(crate) head_sha: String,
    pub(crate) score: Option<u8>,
}

/// A GitHub Actions workflow run
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WorkflowRun {
    pub(crate) id: u64,
    pub(crate) name: Option<String>,
    pub(crate) path: String,
    pub(crate) event: String,
    pub(crate) status: RunStatus,
    pub(crate) conclusion: Option<RunConclusion>,
    pub(crate) html_url: String,
    /// PRs in this repository the run belongs to; empty for a fork's PR
    #[serde(default)]
    pub(crate) pull_requests: Vec<RunPullRequest>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RunPullRequest {
    pub(crate) number: u64,
}

impl WorkflowRun {
    /// The workflow's display name, or its file path when it has none
    pub(crate) fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.path)
    }

    pub(crate) fn is_completed(&self) -> bool {
        self.status == RunStatus::Completed
    }

    /// Whether a completed run counts as passing; `None` while it is still running
    pub(crate) fn passed(&self) -> Option<bool> {
        if !self.is_completed() {
            return None;
        }

        Some(self.conclusion.is_some_and(RunConclusion::is_passing))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RunStatus {
    Requested,
    Queued,
    Pending,
    Waiting,
    InProgress,
    Completed,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RunConclusion {
    Success,
    Neutral,
    Skipped,
    Failure,
    Cancelled,
    TimedOut,
    ActionRequired,
    StartupFailure,
    Stale,
    #[serde(other)]
    Unknown,
}

impl RunConclusion {
    fn is_passing(self) -> bool {
        matches!(self, Self::Success | Self::Neutral | Self::Skipped)
    }

    /// The conclusion as GitHub spells it, for one-line reports
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Neutral => "neutral",
            Self::Skipped => "skipped",
            Self::Failure => "failure",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed_out",
            Self::ActionRequired => "action_required",
            Self::StartupFailure => "startup_failure",
            Self::Stale => "stale",
            Self::Unknown => "unknown",
        }
    }
}

impl Github {
    pub(crate) async fn fetch_pr_head(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
    ) -> Result<PullRequestHead> {
        let number = graphql_number(number)?;
        let data: RepositoryData<PullRequestHead> = self
            .graphql(
                "PullRequestHead",
                PULL_REQUEST_HEAD_QUERY,
                json!({"owner": owner, "repo": repo, "number": number}),
            )
            .await?;

        pull_request(data, owner, repo, number)
    }

    /// Read the check rollup of `sha` and the current head of PR `number`
    pub(crate) async fn fetch_commit_checks(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        sha: &str,
    ) -> Result<CommitChecks> {
        let number = graphql_number(number)?;
        let mut checks = Vec::new();
        let mut cursor = None;

        loop {
            let data: CommitChecksData = self
                .graphql(
                    "CommitChecks",
                    COMMIT_CHECKS_QUERY,
                    json!({
                        "owner": owner,
                        "repo": repo,
                        "number": number,
                        "sha": sha,
                        "contextsCursor": cursor,
                    }),
                )
                .await?;
            let repository = data
                .repository
                .with_context(|| format!("Repository not found: {owner}/{repo}"))?;
            let head_sha = repository
                .pull_request
                .with_context(|| format!("Pull request not found: {owner}/{repo}#{number}"))?
                .head_ref_oid;

            // a commit with no checks or statuses has a null rollup
            let Some(rollup) = repository
                .object
                .and_then(|commit| commit.status_check_rollup)
            else {
                return Ok(CommitChecks { head_sha, checks });
            };

            checks.extend(
                rollup
                    .contexts
                    .nodes
                    .into_iter()
                    .flatten()
                    .map(RollupContext::into_check),
            );
            cursor = rollup.contexts.page_info.next_cursor("check rollup")?;
            if cursor.is_none() {
                return Ok(CommitChecks { head_sha, checks });
            }
        }
    }

    /// Every Actions run on `sha`, across workflows and events
    pub(crate) async fn fetch_workflow_runs(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
    ) -> Result<Vec<WorkflowRun>> {
        let mut runs = Vec::new();

        for page in 1.. {
            let response: WorkflowRunsPage = self
                .get_json(
                    &format!(
                        "repos/{owner}/{repo}/actions/runs?head_sha={sha}&per_page=100&page={page}"
                    ),
                    &format!("Failed to list workflow runs for {owner}/{repo}@{sha}"),
                )
                .await?;
            let count = response.workflow_runs.len();
            runs.extend(response.workflow_runs);

            if count == 0 || runs.len() >= response.total_count {
                break;
            }
        }

        Ok(runs)
    }

    pub(crate) async fn fetch_workflow_run(
        &self,
        owner: &str,
        repo: &str,
        id: u64,
    ) -> Result<WorkflowRun> {
        self.get_json(
            &format!("repos/{owner}/{repo}/actions/runs/{id}"),
            &format!("Failed to fetch workflow run {id} of {owner}/{repo}"),
        )
        .await
    }

    /// Names of the jobs in the latest attempt of run `id` that did not pass
    pub(crate) async fn fetch_failed_jobs(
        &self,
        owner: &str,
        repo: &str,
        id: u64,
    ) -> Result<Vec<String>> {
        let mut failed = Vec::new();
        let mut seen = 0;

        for page in 1.. {
            let response: JobsPage = self
                .get_json(
                    &format!(
                        "repos/{owner}/{repo}/actions/runs/{id}/jobs?filter=latest&per_page=100&page={page}"
                    ),
                    &format!("Failed to list jobs of workflow run {id} of {owner}/{repo}"),
                )
                .await?;
            seen += response.jobs.len();
            let count = response.jobs.len();
            failed.extend(
                response
                    .jobs
                    .into_iter()
                    .filter(|job| !job.conclusion.is_some_and(RunConclusion::is_passing))
                    .map(|job| job.name),
            );

            if count == 0 || seen >= response.total_count {
                break;
            }
        }

        Ok(failed)
    }

    /// The plain-text log of Actions job `id`. GitHub answers with a redirect to
    /// a short-lived download URL on another host, which reqwest follows without
    /// sending the token there
    pub(crate) async fn fetch_job_log(&self, owner: &str, repo: &str, id: u64) -> Result<String> {
        let context = format!("Failed to fetch the log of job {id} of {owner}/{repo}");
        let request = self
            .client
            .get(self.url(&format!("repos/{owner}/{repo}/actions/jobs/{id}/logs")));
        let response = self.send(self.authorize(request), &context).await?;
        let bytes = response
            .bytes()
            .await
            .with_context(|| format!("{context}: could not read the response"))?;

        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    /// Greptile's confidence score for `sha`, from the summary comment it edits
    /// in place on each review
    pub(crate) async fn fetch_greptile_review(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        sha: &str,
    ) -> Result<GreptileReview> {
        let number = graphql_number(number)?;
        let mut score = None;
        let mut cursor = None;

        loop {
            let data: RepositoryData<CommentsPage> = self
                .graphql(
                    "PullRequestComments",
                    PULL_REQUEST_COMMENTS_QUERY,
                    pr_variables(owner, repo, number, cursor.as_deref()),
                )
                .await?;
            let page = pull_request(data, owner, repo, number)?;

            // the newest matching summary wins, as comments come oldest first
            let page_score = page
                .comments
                .nodes
                .iter()
                .flatten()
                .filter_map(|comment| comment.greptile_score(sha))
                .next_back();
            score = page_score.or(score);

            cursor = page.comments.page_info.next_cursor("PR comments")?;
            if cursor.is_none() {
                return Ok(GreptileReview {
                    head_sha: page.head_ref_oid,
                    score,
                });
            }
        }
    }

    async fn get_json<T>(&self, path: &str, context: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let request = self
            .client
            .get(self.url(path))
            .header("Accept", "application/vnd.github+json");
        let response = self.send(self.authorize(request), context).await?;

        response
            .json()
            .await
            .with_context(|| format!("{context}: could not decode the response"))
    }
}

/// GraphQL's `Int` is 32-bit
fn graphql_number(number: u64) -> Result<i64> {
    i32::try_from(number)
        .map(i64::from)
        .context("Pull request number is too large")
}

#[derive(Deserialize)]
struct CommitChecksData {
    repository: Option<CommitChecksRepository>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommitChecksRepository {
    pull_request: Option<HeadOnly>,
    object: Option<CommitObject>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HeadOnly {
    head_ref_oid: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommitObject {
    status_check_rollup: Option<Rollup>,
}

#[derive(Deserialize)]
struct Rollup {
    contexts: Connection<RollupContext>,
}

#[derive(Deserialize)]
#[serde(tag = "__typename")]
enum RollupContext {
    #[serde(rename_all = "camelCase")]
    CheckRun {
        name: String,
        status: CheckRunStatus,
        conclusion: Option<CheckRunConclusion>,
        database_id: Option<u64>,
        details_url: Option<String>,
        check_suite: Option<CheckSuite>,
    },
    #[serde(rename_all = "camelCase")]
    StatusContext {
        context: String,
        state: StatusState,
        target_url: Option<String>,
    },
}

impl RollupContext {
    fn into_check(self) -> Check {
        match self {
            Self::CheckRun {
                name,
                status,
                conclusion,
                database_id,
                details_url,
                check_suite,
            } => {
                let run_id = check_suite
                    .and_then(|suite| suite.workflow_run)
                    .map(|run| run.database_id);

                Check {
                    name,
                    state: check_run_state(status, conclusion),
                    run_id,
                    // an external app's check run id names no job
                    job_id: database_id.filter(|_| run_id.is_some()),
                    url: details_url,
                }
            }
            Self::StatusContext {
                context,
                state,
                target_url,
            } => Check {
                name: context,
                state: match state {
                    StatusState::Success => CheckState::Passed,
                    StatusState::Pending | StatusState::Expected => CheckState::Pending,
                    StatusState::Failure | StatusState::Error | StatusState::Unknown => {
                        CheckState::Failed
                    }
                },
                run_id: None,
                job_id: None,
                url: target_url,
            },
        }
    }
}

fn check_run_state(status: CheckRunStatus, conclusion: Option<CheckRunConclusion>) -> CheckState {
    if status != CheckRunStatus::Completed {
        return CheckState::Pending;
    }

    match conclusion {
        Some(
            CheckRunConclusion::Success | CheckRunConclusion::Neutral | CheckRunConclusion::Skipped,
        ) => CheckState::Passed,
        // stale means GitHub gave up on a run that never finished
        _ => CheckState::Failed,
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CheckSuite {
    workflow_run: Option<WorkflowRunRef>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkflowRunRef {
    database_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum CheckRunStatus {
    Requested,
    Queued,
    Pending,
    Waiting,
    InProgress,
    Completed,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum CheckRunConclusion {
    Success,
    Neutral,
    Skipped,
    Failure,
    Cancelled,
    TimedOut,
    ActionRequired,
    StartupFailure,
    Stale,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum StatusState {
    Success,
    Pending,
    Expected,
    Failure,
    Error,
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
struct WorkflowRunsPage {
    total_count: usize,
    workflow_runs: Vec<WorkflowRun>,
}

#[derive(Deserialize)]
struct JobsPage {
    total_count: usize,
    jobs: Vec<Job>,
}

#[derive(Deserialize)]
struct Job {
    name: String,
    conclusion: Option<RunConclusion>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommentsPage {
    head_ref_oid: String,
    comments: Connection<Comment>,
}

#[derive(Deserialize)]
struct Comment {
    body: String,
    author: Option<CommentAuthor>,
}

#[derive(Deserialize)]
struct CommentAuthor {
    #[serde(rename = "__typename")]
    typename: String,
    login: String,
}

impl Comment {
    /// The score in a Greptile summary of `sha`; only the bot account counts,
    /// since anyone can post the marker on a public repo, and its "Last reviewed
    /// commit" link names the commit the score belongs to
    fn greptile_score(&self, sha: &str) -> Option<u8> {
        let author = self.author.as_ref()?;
        if author.typename != "Bot" || author.login != GREPTILE_LOGIN {
            return None;
        }

        if !self.body.contains(GREPTILE_SUMMARY_MARKER) || self.reviewed_commit()? != sha {
            return None;
        }

        let (_, rest) = self.body.split_once(GREPTILE_SCORE_PREFIX)?;
        let (score, _) = rest.split_once("-->")?;
        score.trim().parse().ok()
    }

    /// The commit in the summary's "Last reviewed commit" link; other commit
    /// links in the body, such as in the review text, do not name the score's commit
    fn reviewed_commit(&self) -> Option<&str> {
        let (_, rest) = self.body.split_once(GREPTILE_REVIEWED_LABEL)?;
        let (_, rest) = rest.split_once("/commit/")?;
        let end = rest
            .find(|c: char| !c.is_ascii_hexdigit())
            .unwrap_or(rest.len());

        Some(&rest[..end])
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::{
        matchers::{body_partial_json, method, path, query_param},
        Mock, MockServer, ResponseTemplate,
    };

    use super::{Check, CheckState, Comment, CommentAuthor, Github, RunStatus};

    const SHA: &str = "1f975f05498f59a5cd76ab1a706e5a526702a0a2";

    #[tokio::test]
    async fn reads_rollup_and_runs_in_github_shapes() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/graphql"))
            .and(body_partial_json(json!({"operationName": "CommitChecks"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {"repository": {
                    "pullRequest": {"headRefOid": SHA},
                    "object": {"statusCheckRollup": {"contexts": {
                        "nodes": [
                            {"__typename": "CheckRun", "name": "ios-ui", "status": "QUEUED",
                             "conclusion": null, "checkSuite": {"workflowRun": {"databaseId": 41649623712_u64}}},
                            {"__typename": "CheckRun", "name": "clippy", "status": "COMPLETED",
                             "conclusion": "SKIPPED", "checkSuite": {"workflowRun": {"databaseId": 41649623546_u64}}},
                            {"__typename": "CheckRun", "name": "CodeQL", "status": "COMPLETED",
                             "conclusion": "STALE", "checkSuite": {"workflowRun": null}},
                            {"__typename": "StatusContext", "context": "CodeRabbit", "state": "ERROR"}
                        ],
                        "pageInfo": {"hasNextPage": false, "endCursor": null}
                    }}}
                }}
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/repos/o/r/actions/runs"))
            .and(query_param("head_sha", SHA))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "total_count": 1,
                "workflow_runs": [{
                    "id": 41649623712_u64, "name": "UI tests", "path": ".github/workflows/ui-tests.yml",
                    "event": "pull_request", "status": "queued", "conclusion": null,
                    "html_url": "https://github.com/o/r/actions/runs/41649623712", "run_attempt": 2
                }]
            })))
            .mount(&server)
            .await;
        let github = Github::with_base_url(server.uri(), None).unwrap();

        let commit = github
            .fetch_commit_checks("o", "r", 978, SHA)
            .await
            .unwrap();
        let runs = github.fetch_workflow_runs("o", "r", SHA).await.unwrap();

        assert_eq!(commit.head_sha, SHA);
        assert_eq!(
            commit.checks,
            [
                Check {
                    name: "ios-ui".into(),
                    state: CheckState::Pending,
                    run_id: Some(41649623712),
                    job_id: None,
                    url: None
                },
                Check {
                    name: "clippy".into(),
                    state: CheckState::Passed,
                    run_id: Some(41649623546),
                    job_id: None,
                    url: None
                },
                Check {
                    name: "CodeQL".into(),
                    state: CheckState::Failed,
                    run_id: None,
                    job_id: None,
                    url: None
                },
                Check {
                    name: "CodeRabbit".into(),
                    state: CheckState::Failed,
                    run_id: None,
                    job_id: None,
                    url: None
                },
            ]
        );
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].status, RunStatus::Queued);
        assert_eq!(runs[0].passed(), None);
    }

    #[test]
    fn greptile_score_needs_the_bot_and_the_commit() {
        let body = format!(
            "<!-- greptile_summary -->\n<!-- greptile_confidence_score:4 -->\nLast reviewed commit: [x](https://github.com/o/r/commit/{SHA})"
        );
        let comment = |typename: &str, login: &str| Comment {
            body: body.clone(),
            author: Some(CommentAuthor {
                typename: typename.into(),
                login: login.into(),
            }),
        };

        assert_eq!(comment("Bot", "greptile-apps").greptile_score(SHA), Some(4));

        // the head's sha elsewhere in the body does not make an older review current
        let older = Comment {
            body: format!(
                "<!-- greptile_summary -->\nSee https://github.com/o/r/commit/{SHA}\n<!-- greptile_confidence_score:5 -->\nLast reviewed commit: [x](https://github.com/o/r/commit/8973bd9060d5c773b8521540818a6e5670400168)"
            ),
            author: Some(CommentAuthor {
                typename: "Bot".into(),
                login: "greptile-apps".into(),
            }),
        };
        assert_eq!(older.greptile_score(SHA), None);
        assert_eq!(comment("User", "greptile-apps").greptile_score(SHA), None);
        assert_eq!(
            comment("Bot", "greptile-apps").greptile_score("abc123"),
            None
        );
    }
}
