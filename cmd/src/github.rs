use color_eyre::eyre::{Context, ContextCompat, Result};
use reqwest::RequestBuilder;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use std::process::Command;

const PULL_REQUEST_METADATA_QUERY: &str = r#"
query PullRequestMetadata($owner: String!, $repo: String!, $number: Int!) {
  repository(owner: $owner, name: $repo) {
    pullRequest(number: $number) {
      number
      url
      title
      state
    }
  }
}
"#;

const CONVERSATION_COMMENTS_QUERY: &str = r#"
query ConversationComments(
  $owner: String!
  $repo: String!
  $number: Int!
  $commentsCursor: String
) {
  repository(owner: $owner, name: $repo) {
    pullRequest(number: $number) {
      comments(first: 100, after: $commentsCursor) {
        nodes {
          id
          body
          createdAt
          updatedAt
          author { login }
        }
        pageInfo { hasNextPage endCursor }
      }
    }
  }
}
"#;

const REVIEWS_QUERY: &str = r#"
query Reviews($owner: String!, $repo: String!, $number: Int!, $reviewsCursor: String) {
  repository(owner: $owner, name: $repo) {
    pullRequest(number: $number) {
      reviews(first: 100, after: $reviewsCursor) {
        nodes {
          id
          state
          body
          submittedAt
          author { login }
        }
        pageInfo { hasNextPage endCursor }
      }
    }
  }
}
"#;

const REVIEW_THREADS_QUERY: &str = r#"
query ReviewThreads($owner: String!, $repo: String!, $number: Int!, $threadsCursor: String) {
  repository(owner: $owner, name: $repo) {
    pullRequest(number: $number) {
      reviewThreads(first: 100, after: $threadsCursor) {
        nodes {
          id
          isResolved
          isOutdated
          path
          line
          diffSide
          startLine
          startDiffSide
          originalLine
          originalStartLine
          subjectType
          resolvedBy { login }
          comments(first: 100) {
            nodes {
              id
              body
              createdAt
              updatedAt
              diffHunk
              author { login }
            }
            pageInfo { hasNextPage endCursor }
          }
        }
        pageInfo { hasNextPage endCursor }
      }
    }
  }
}
"#;

const THREAD_COMMENTS_QUERY: &str = r#"
query ThreadComments($threadId: ID!, $commentsCursor: String) {
  node(id: $threadId) {
    ... on PullRequestReviewThread {
      comments(first: 100, after: $commentsCursor) {
        nodes {
          id
          body
          createdAt
          updatedAt
          diffHunk
          author { login }
        }
        pageInfo { hasNextPage endCursor }
      }
    }
  }
}
"#;

pub(crate) struct Github {
    client: reqwest::Client,
    token: Option<String>,
    base_url: String,
}

impl Github {
    const BASE_URL: &'static str = "https://api.github.com";

    pub(crate) fn new(token: Option<String>) -> Result<Self> {
        Self::with_base_url(Self::BASE_URL, resolve_token(token))
    }

    fn with_base_url(base_url: impl Into<String>, token: Option<String>) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("pr-context-cli")
            .build()?;

        Ok(Self {
            client,
            token: token.and_then(non_empty_token),
            base_url: base_url.into().trim_end_matches('/').to_string(),
        })
    }

    pub(crate) async fn fetch_pr_review(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
    ) -> Result<PullRequestReviewData> {
        let number = i64::try_from(number).context("Pull request number is too large")?;
        let (pull_request, conversation_comments, reviews, review_threads) = tokio::try_join!(
            self.fetch_pull_request(owner, repo, number),
            self.fetch_conversation_comments(owner, repo, number),
            self.fetch_reviews(owner, repo, number),
            self.fetch_review_threads(owner, repo, number),
        )?;

        Ok(PullRequestReviewData {
            repository: format!("{owner}/{repo}"),
            pull_request,
            conversation_comments,
            reviews,
            review_threads,
        })
    }

    async fn fetch_pull_request(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
    ) -> Result<PullRequestMetadata> {
        let data: RepositoryData<PullRequestMetadata> = self
            .graphql(
                "PullRequestMetadata",
                PULL_REQUEST_METADATA_QUERY,
                json!({"owner": owner, "repo": repo, "number": number}),
            )
            .await?;

        pull_request(data, owner, repo, number)
    }

    async fn fetch_conversation_comments(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
    ) -> Result<Vec<ConversationComment>> {
        let mut comments = Vec::new();
        let mut cursor = None;

        loop {
            let data: RepositoryData<ConversationCommentsPage> = self
                .graphql(
                    "ConversationComments",
                    CONVERSATION_COMMENTS_QUERY,
                    pr_variables(owner, repo, number, cursor.as_deref()),
                )
                .await?;
            let page = pull_request(data, owner, repo, number)?.comments;
            comments.extend(page.nodes.into_iter().flatten());
            cursor = page.page_info.next_cursor("conversation comments")?;
            if cursor.is_none() {
                break;
            }
        }

        Ok(comments)
    }

    async fn fetch_reviews(&self, owner: &str, repo: &str, number: i64) -> Result<Vec<Review>> {
        let mut reviews = Vec::new();
        let mut cursor = None;

        loop {
            let data: RepositoryData<ReviewsPage> = self
                .graphql(
                    "Reviews",
                    REVIEWS_QUERY,
                    json!({
                        "owner": owner,
                        "repo": repo,
                        "number": number,
                        "reviewsCursor": cursor,
                    }),
                )
                .await?;
            let page = pull_request(data, owner, repo, number)?.reviews;
            reviews.extend(page.nodes.into_iter().flatten());
            cursor = page.page_info.next_cursor("reviews")?;
            if cursor.is_none() {
                break;
            }
        }

        Ok(reviews)
    }

    async fn fetch_review_threads(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
    ) -> Result<Vec<ReviewThread>> {
        let mut threads = Vec::new();
        let mut cursor = None;

        loop {
            let data: RepositoryData<ReviewThreadsPage> = self
                .graphql(
                    "ReviewThreads",
                    REVIEW_THREADS_QUERY,
                    json!({
                        "owner": owner,
                        "repo": repo,
                        "number": number,
                        "threadsCursor": cursor,
                    }),
                )
                .await?;
            let page = pull_request(data, owner, repo, number)?.review_threads;
            for thread in page.nodes.into_iter().flatten() {
                threads.push(self.finish_thread(thread).await?);
            }
            cursor = page.page_info.next_cursor("review threads")?;
            if cursor.is_none() {
                break;
            }
        }

        Ok(threads)
    }

    async fn finish_thread(&self, thread: ReviewThreadPage) -> Result<ReviewThread> {
        let mut comments = thread
            .comments
            .nodes
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let mut cursor = thread
            .comments
            .page_info
            .next_cursor("review thread comments")?;

        while let Some(current_cursor) = cursor {
            let data: ThreadCommentsData = self
                .graphql(
                    "ThreadComments",
                    THREAD_COMMENTS_QUERY,
                    json!({
                        "threadId": &thread.id,
                        "commentsCursor": current_cursor,
                    }),
                )
                .await?;
            let page = data
                .node
                .context("Review thread disappeared while fetching its comments")?
                .comments;
            comments.extend(page.nodes.into_iter().flatten());
            cursor = page.page_info.next_cursor("review thread comments")?;
        }

        Ok(ReviewThread {
            id: thread.id,
            is_resolved: thread.is_resolved,
            is_outdated: thread.is_outdated,
            path: thread.path,
            line: thread.line,
            diff_side: thread.diff_side,
            start_line: thread.start_line,
            start_diff_side: thread.start_diff_side,
            original_line: thread.original_line,
            original_start_line: thread.original_start_line,
            subject_type: thread.subject_type,
            resolved_by: thread.resolved_by,
            comments,
        })
    }

    async fn graphql<T>(&self, operation_name: &str, query: &str, variables: Value) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let request = self.client.post(self.url("graphql")).json(&GraphqlRequest {
            operation_name,
            query,
            variables,
        });
        let response = self
            .send(
                self.authorize(request),
                "Failed to query GitHub GraphQL API",
            )
            .await?;
        let payload: GraphqlResponse<T> = response
            .json()
            .await
            .context("Failed to decode GitHub GraphQL response")?;

        if !payload.errors.is_empty() {
            let messages = payload
                .errors
                .into_iter()
                .map(|error| error.message)
                .collect::<Vec<_>>()
                .join("; ");
            eyre::bail!("GitHub GraphQL query {operation_name} failed: {messages}");
        }

        payload
            .data
            .with_context(|| format!("GitHub GraphQL query {operation_name} returned no data"))
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    fn authorize(&self, request: RequestBuilder) -> RequestBuilder {
        if let Some(token) = &self.token {
            request.header("Authorization", format!("Bearer {token}"))
        } else {
            request
        }
    }

    async fn send(&self, request: RequestBuilder, context: &str) -> Result<reqwest::Response> {
        let response = request.send().await?;

        if response.status().is_success() {
            return Ok(response);
        }

        let status = response.status();
        let body = response.text().await?;

        if matches!(
            status,
            reqwest::StatusCode::UNAUTHORIZED
                | reqwest::StatusCode::FORBIDDEN
                | reqwest::StatusCode::NOT_FOUND
        ) {
            eyre::bail!(
                "{context}: GitHub API request failed with status {status}: {body}\nIf this is a private repository, authenticate with --token, GITHUB_TOKEN, GH_TOKEN, or `gh auth login` with repo read access"
            );
        }

        eyre::bail!("{context}: GitHub API request failed with status {status}: {body}");
    }
}

fn pr_variables(owner: &str, repo: &str, number: i64, cursor: Option<&str>) -> Value {
    json!({
        "owner": owner,
        "repo": repo,
        "number": number,
        "commentsCursor": cursor,
    })
}

fn pull_request<T>(data: RepositoryData<T>, owner: &str, repo: &str, number: i64) -> Result<T> {
    data.repository
        .and_then(|repository| repository.pull_request)
        .with_context(|| format!("Pull request not found: {owner}/{repo}#{number}"))
}

pub(crate) fn resolve_token(explicit_token: Option<String>) -> Option<String> {
    resolve_token_with(
        explicit_token,
        |name| std::env::var(name).ok(),
        gh_auth_token,
    )
}

fn resolve_token_with(
    explicit_token: Option<String>,
    env_token: impl Fn(&str) -> Option<String>,
    gh_token: impl Fn() -> Option<String>,
) -> Option<String> {
    explicit_token
        .and_then(non_empty_token)
        .or_else(|| env_token("GITHUB_TOKEN").and_then(non_empty_token))
        .or_else(|| env_token("GH_TOKEN").and_then(non_empty_token))
        .or_else(|| gh_token().and_then(non_empty_token))
}

fn gh_auth_token() -> Option<String> {
    let output = Command::new("gh")
        .args(["auth", "token", "--hostname", "github.com"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8(output.stdout)
        .ok()
        .and_then(non_empty_token)
}

fn non_empty_token(token: String) -> Option<String> {
    let token = token.trim().to_string();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

pub(crate) struct PullRequestReviewData {
    pub(crate) repository: String,
    pub(crate) pull_request: PullRequestMetadata,
    pub(crate) conversation_comments: Vec<ConversationComment>,
    pub(crate) reviews: Vec<Review>,
    pub(crate) review_threads: Vec<ReviewThread>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PullRequestMetadata {
    pub(crate) number: u64,
    pub(crate) url: String,
    pub(crate) title: String,
    pub(crate) state: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConversationComment {
    pub(crate) id: String,
    pub(crate) body: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) author: Option<Author>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Review {
    pub(crate) id: String,
    pub(crate) state: String,
    pub(crate) body: String,
    pub(crate) submitted_at: Option<String>,
    pub(crate) author: Option<Author>,
}

#[derive(Debug)]
pub(crate) struct ReviewThread {
    pub(crate) id: String,
    pub(crate) is_resolved: bool,
    pub(crate) is_outdated: bool,
    pub(crate) path: String,
    pub(crate) line: Option<u64>,
    pub(crate) diff_side: String,
    pub(crate) start_line: Option<u64>,
    pub(crate) start_diff_side: Option<String>,
    pub(crate) original_line: Option<u64>,
    pub(crate) original_start_line: Option<u64>,
    pub(crate) subject_type: String,
    pub(crate) resolved_by: Option<Author>,
    pub(crate) comments: Vec<ReviewThreadComment>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewThreadComment {
    pub(crate) id: String,
    pub(crate) body: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) diff_hunk: String,
    pub(crate) author: Option<Author>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Author {
    pub(crate) login: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GraphqlRequest<'a> {
    operation_name: &'a str,
    query: &'a str,
    variables: Value,
}

#[derive(Deserialize)]
struct GraphqlResponse<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Vec<GraphqlError>,
}

#[derive(Deserialize)]
struct GraphqlError {
    message: String,
}

#[derive(Deserialize)]
struct RepositoryData<T> {
    repository: Option<RepositoryPullRequest<T>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepositoryPullRequest<T> {
    pull_request: Option<T>,
}

#[derive(Deserialize)]
struct Connection<T> {
    nodes: Vec<Option<T>>,
    #[serde(rename = "pageInfo")]
    page_info: PageInfo,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    has_next_page: bool,
    end_cursor: Option<String>,
}

impl PageInfo {
    fn next_cursor(&self, connection: &str) -> Result<Option<String>> {
        if !self.has_next_page {
            return Ok(None);
        }

        self.end_cursor
            .clone()
            .map(Some)
            .with_context(|| format!("GitHub omitted the next cursor for {connection}"))
    }
}

#[derive(Deserialize)]
struct ConversationCommentsPage {
    comments: Connection<ConversationComment>,
}

#[derive(Deserialize)]
struct ReviewsPage {
    reviews: Connection<Review>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReviewThreadsPage {
    review_threads: Connection<ReviewThreadPage>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReviewThreadPage {
    id: String,
    is_resolved: bool,
    is_outdated: bool,
    path: String,
    line: Option<u64>,
    diff_side: String,
    start_line: Option<u64>,
    start_diff_side: Option<String>,
    original_line: Option<u64>,
    original_start_line: Option<u64>,
    subject_type: String,
    resolved_by: Option<Author>,
    comments: Connection<ReviewThreadComment>,
}

#[derive(Deserialize)]
struct ThreadCommentsData {
    node: Option<ThreadCommentsPage>,
}

#[derive(Deserialize)]
struct ThreadCommentsPage {
    comments: Connection<ReviewThreadComment>,
}

pub(crate) fn parse_url(url: &str) -> Result<(String, String, u64)> {
    let url = url.trim_end_matches('/');
    let url = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let url = url
        .strip_prefix("github.com/")
        .ok_or_else(|| eyre::eyre!("URL must be a GitHub URL"))?;
    let parts: Vec<&str> = url.split('/').collect();

    if parts.len() < 4 {
        eyre::bail!(
            "Invalid GitHub PR URL format. Expected: https://github.com/owner/repo/pull/123"
        );
    }
    if parts[2] != "pull" {
        eyre::bail!("URL must be a pull request URL (contain '/pull/')");
    }

    let number = parts[3]
        .parse::<u64>()
        .context("Failed to parse PR number from URL")?;
    Ok((parts[0].to_string(), parts[1].to_string(), number))
}

#[cfg(test)]
mod tests {
    use super::{pull_request, resolve_token_with, Github, PullRequestMetadata, RepositoryData};
    use serde_json::{json, Value};
    use wiremock::{
        matchers::{body_partial_json, header, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[test]
    fn token_resolver_prefers_explicit_token() {
        let token = resolve_token_with(
            Some(" explicit ".to_string()),
            |_| Some("env-token".to_string()),
            || Some("gh-token".to_string()),
        );

        assert_eq!(token.as_deref(), Some("explicit"));
    }

    #[test]
    fn token_resolver_uses_env_before_gh() {
        let token = resolve_token_with(
            None,
            |name| match name {
                "GITHUB_TOKEN" => Some(" github ".to_string()),
                "GH_TOKEN" => Some(" gh-env ".to_string()),
                _ => None,
            },
            || Some("gh-cli".to_string()),
        );

        assert_eq!(token.as_deref(), Some("github"));
    }

    #[test]
    fn token_resolver_skips_empty_tokens_and_uses_gh_token_env() {
        let token = resolve_token_with(
            Some(" ".to_string()),
            |name| match name {
                "GITHUB_TOKEN" => Some(" ".to_string()),
                "GH_TOKEN" => Some(" gh-env ".to_string()),
                _ => None,
            },
            || Some("gh-cli".to_string()),
        );

        assert_eq!(token.as_deref(), Some("gh-env"));
    }

    #[test]
    fn token_resolver_falls_back_to_gh_auth_token() {
        let token = resolve_token_with(None, |_| None, || Some(" gh-cli ".to_string()));

        assert_eq!(token.as_deref(), Some("gh-cli"));
    }

    #[test]
    fn missing_pull_request_has_a_scoped_error() {
        let error = pull_request(
            RepositoryData::<PullRequestMetadata> { repository: None },
            "owner",
            "repo",
            7,
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("owner/repo#7"));
    }

    #[tokio::test]
    async fn graphql_sends_auth_and_reports_api_errors() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/graphql"))
            .and(header("authorization", "Bearer token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": null,
                "errors": [{"message": "scope denied"}]
            })))
            .mount(&server)
            .await;
        let github = Github::with_base_url(server.uri(), Some("token".to_string())).unwrap();

        let error = github
            .graphql::<serde_json::Value>("Test", "query Test { viewer { login } }", json!({}))
            .await
            .unwrap_err()
            .to_string();

        assert!(error.contains("scope denied"));
    }

    #[tokio::test]
    async fn graphql_auth_failures_explain_how_to_authenticate() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/graphql"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "message": "Bad credentials"
            })))
            .mount(&server)
            .await;
        let github = Github::with_base_url(server.uri(), None).unwrap();

        let error = github
            .graphql::<serde_json::Value>("Test", "query Test { viewer { login } }", json!({}))
            .await
            .unwrap_err()
            .to_string();

        assert!(error.contains("GITHUB_TOKEN"));
        assert!(error.contains("GH_TOKEN"));
        assert!(error.contains("gh auth login"));
    }

    #[tokio::test]
    async fn fetches_and_paginates_thread_aware_review_data() {
        let server = MockServer::start().await;
        mount_response(
            &server,
            json!({"operationName": "PullRequestMetadata"}),
            json!({"data": {"repository": {"pullRequest": {
                "number": 7,
                "url": "https://github.com/owner/repo/pull/7",
                "title": "Thread-aware comments",
                "state": "OPEN"
            }}}}),
        )
        .await;
        mount_response(
            &server,
            json!({
                "operationName": "ConversationComments",
                "variables": {"commentsCursor": null}
            }),
            connection_response(
                "comments",
                json!([comment("IC_1", "first")]),
                true,
                Some("comments-2"),
            ),
        )
        .await;
        mount_response(
            &server,
            json!({
                "operationName": "ConversationComments",
                "variables": {"commentsCursor": "comments-2"}
            }),
            connection_response("comments", json!([comment("IC_2", "second")]), false, None),
        )
        .await;
        mount_response(
            &server,
            json!({
                "operationName": "Reviews",
                "variables": {"reviewsCursor": null}
            }),
            connection_response(
                "reviews",
                json!([{
                    "id": "PRR_1",
                    "state": "CHANGES_REQUESTED",
                    "body": "Please fix this",
                    "submittedAt": "2026-01-01T00:00:00Z",
                    "author": {"login": "reviewer"}
                }]),
                true,
                Some("reviews-2"),
            ),
        )
        .await;
        mount_response(
            &server,
            json!({
                "operationName": "Reviews",
                "variables": {"reviewsCursor": "reviews-2"}
            }),
            connection_response(
                "reviews",
                json!([{
                    "id": "PRR_2",
                    "state": "APPROVED",
                    "body": "Looks good",
                    "submittedAt": "2026-01-02T00:00:00Z",
                    "author": {"login": "maintainer"}
                }]),
                false,
                None,
            ),
        )
        .await;
        mount_response(
            &server,
            json!({
                "operationName": "ReviewThreads",
                "variables": {"threadsCursor": null}
            }),
            connection_response(
                "reviewThreads",
                json!([{
                    "id": "PRRT_1",
                    "isResolved": false,
                    "isOutdated": true,
                    "path": "src/lib.rs",
                    "line": null,
                    "diffSide": "RIGHT",
                    "startLine": null,
                    "startDiffSide": null,
                    "originalLine": 12,
                    "originalStartLine": null,
                    "subjectType": "LINE",
                    "resolvedBy": null,
                    "comments": {
                        "nodes": [review_comment("PRRC_1", "root")],
                        "pageInfo": {"hasNextPage": true, "endCursor": "thread-comments-2"}
                    }
                }]),
                true,
                Some("threads-2"),
            ),
        )
        .await;
        mount_response(
            &server,
            json!({
                "operationName": "ReviewThreads",
                "variables": {"threadsCursor": "threads-2"}
            }),
            connection_response(
                "reviewThreads",
                json!([{
                    "id": "PRRT_2",
                    "isResolved": true,
                    "isOutdated": false,
                    "path": "README.md",
                    "line": null,
                    "diffSide": "RIGHT",
                    "startLine": null,
                    "startDiffSide": null,
                    "originalLine": null,
                    "originalStartLine": null,
                    "subjectType": "FILE",
                    "resolvedBy": {"login": "maintainer"},
                    "comments": {
                        "nodes": [],
                        "pageInfo": {"hasNextPage": false, "endCursor": null}
                    }
                }]),
                false,
                None,
            ),
        )
        .await;
        mount_response(
            &server,
            json!({
                "operationName": "ThreadComments",
                "variables": {
                    "threadId": "PRRT_1",
                    "commentsCursor": "thread-comments-2"
                }
            }),
            json!({"data": {"node": {"comments": {
                "nodes": [review_comment("PRRC_2", "reply")],
                "pageInfo": {"hasNextPage": false, "endCursor": null}
            }}}}),
        )
        .await;

        let github = Github::with_base_url(server.uri(), Some("token".to_string())).unwrap();
        let data = github.fetch_pr_review("owner", "repo", 7).await.unwrap();

        assert_eq!(data.repository, "owner/repo");
        assert_eq!(data.conversation_comments.len(), 2);
        assert_eq!(data.reviews.len(), 2);
        assert_eq!(data.review_threads.len(), 2);
        assert_eq!(data.review_threads[0].comments.len(), 2);
        assert!(data.review_threads[0].is_outdated);
    }

    async fn mount_response(server: &MockServer, matcher: serde_json::Value, response: Value) {
        Mock::given(method("POST"))
            .and(path("/graphql"))
            .and(body_partial_json(matcher))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(server)
            .await;
    }

    fn connection_response(
        field: &str,
        nodes: Value,
        has_next_page: bool,
        end_cursor: Option<&str>,
    ) -> Value {
        json!({"data": {"repository": {"pullRequest": {
            (field): {
                "nodes": nodes,
                "pageInfo": {
                    "hasNextPage": has_next_page,
                    "endCursor": end_cursor
                }
            }
        }}}})
    }

    fn comment(id: &str, body: &str) -> Value {
        json!({
            "id": id,
            "body": body,
            "createdAt": "2026-01-01T00:00:00Z",
            "updatedAt": "2026-01-01T00:00:00Z",
            "author": {"login": "reviewer"}
        })
    }

    fn review_comment(id: &str, body: &str) -> Value {
        let mut value = comment(id, body);
        value["diffHunk"] = json!("@@ -1 +1 @@");
        value
    }
}
