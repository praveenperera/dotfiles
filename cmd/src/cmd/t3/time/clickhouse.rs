//! User messages from the ClickHouse chat history that CLIProxyAPI records on
//! `code`, which also holds imported Claude Code and Codex transcripts. They
//! show T3 sessions no reachable machine still has, such as threads on an
//! offline laptop or threads `cmd t3 purge` removed. The report lists them
//! but no estimate counts them: the proxy cannot tell a typed prompt from a
//! message an agent, another thread, or Homebased sent

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::Duration;

use chrono::{DateTime, NaiveDateTime, Utc};
use eyre::{bail, eyre, Result, WrapErr};
use serde::de::DeserializeOwned;
use serde::Deserialize;

use super::model::{
    prompts_with_replies, typed_blocks, Agent, Conversation, Prompt, SessionId, Source, SourceData,
    Step, Window,
};
use crate::{fsutil, runtime};

/// The proxy's ClickHouse, which binds only its LAN and Tailscale addresses
const DEFAULT_URL: &str = "http://code.local:8123";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// How far before the window a message's first sighting is looked for. The
/// proxy re-sends a conversation after compaction, so a message can show up
/// again days after it was typed
const LOOKBACK_DAYS: u32 = 14;

/// A call that shows more new user messages than this replays history, as
/// after a compaction, fork, or resume gives the old messages new nodes, so
/// only its last message can be newly typed. A typed prompt arrives alone,
/// or with a few queued ones or tool results
const REPLAY_NODES: u32 = 4;

/// How far past the window agent replies are read
const REPLY_LOOKAHEAD_HOURS: u32 = 6;

/// Text in the first messages of every session T3 runs, from the context it
/// adds for Claude and Codex alike
const T3_MARKER: &str = "T3 Code";

/// A main Claude Code call sends its whole tool list, tens of KB; Claude
/// Code's side calls, such as titling a session or running a web search, send
/// none or a single tool, and their prompts are not typed. The proxy records
/// no tool list for Codex calls, so the check applies to Claude only
const MAIN_AGENT_TOOLS_MIN_BYTES: u32 = 4096;

struct Credentials {
    url: String,
    user: String,
    password: String,
}

/// `~/.config/chat-history/clickhouse.env`, which setup.sh writes on `code`
fn credentials() -> Result<Option<Credentials>> {
    let path: PathBuf = fsutil::home_dir()?.join(".config/chat-history/clickhouse.env");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(None);
    };

    let vars: HashMap<&str, &str> = text
        .lines()
        .filter_map(|line| line.trim().split_once('='))
        .map(|(key, value)| (key.trim(), value.trim().trim_matches('"')))
        .collect();

    let get = |key: &str| {
        vars.get(key)
            .map(|value| (*value).to_owned())
            .ok_or_else(|| eyre!("{} has no {key}", path.display()))
    };

    Ok(Some(Credentials {
        url: vars
            .get("CLICKHOUSE_URL")
            .map_or(DEFAULT_URL, |url| *url)
            .to_owned(),
        user: get("CLICKHOUSE_USER")?,
        password: get("CLICKHOUSE_PASSWORD")?,
    }))
}

/// Read the window from ClickHouse, or `None` when this machine has no
/// credentials for it
pub fn collect(window: &Window) -> Result<Option<SourceData>> {
    let Some(credentials) = credentials()? else {
        return Ok(None);
    };

    let client = Client {
        http: reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()?,
        credentials,
    };

    // each query takes seconds on the server, so they run at once
    let (prompts, replies, contexts) = (
        prompts_sql(window),
        replies_sql(window),
        contexts_sql(window),
    );
    let (rows, replies, contexts) = runtime::block_on(async {
        futures::try_join!(
            client.query::<PromptRow>(&prompts),
            client.query::<ReplyRow>(&replies),
            client.query::<ContextRow>(&contexts),
        )
    })??;

    Ok(Some(SourceData {
        prompts: build_prompts(window, rows, replies, contexts),
        ..SourceData::default()
    }))
}

struct Client {
    http: reqwest::Client,
    credentials: Credentials,
}

impl Client {
    async fn query<T: DeserializeOwned>(&self, sql: &str) -> Result<Vec<T>> {
        let Credentials {
            url,
            user,
            password,
        } = &self.credentials;

        let response = self
            .http
            .post(url)
            .basic_auth(user, Some(password))
            .body(format!("{sql}\nFORMAT JSONEachRow"))
            .send()
            .await
            .wrap_err_with(|| format!("query ClickHouse at {url}"))?;

        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            bail!("ClickHouse returned {status}: {}", body.trim());
        }

        body.lines()
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_str(line).wrap_err("parse a ClickHouse row"))
            .collect()
    }
}

fn sql_time(at: DateTime<Utc>) -> String {
    format!(
        "toDateTime64('{}', 3, 'UTC')",
        at.format("%Y-%m-%d %H:%M:%S%.3f")
    )
}

/// Bounds and the conversations worth reading: main Claude Code and Codex
/// sessions, without subagents, whose ids end in `:agent:<id>` or which
/// name a parent session
fn preamble(window: &Window) -> String {
    format!(
        "WITH
  {since} AS since,
  {until} AS until,
  since - INTERVAL {LOOKBACK_DAYS} DAY AS lookback,
  sessions AS (
    SELECT DISTINCT session_id FROM chat.nodes
    WHERE role = 'user' AND ts >= since AND ts < until
      AND match(session_id, '^(claude|claude-code|codex):') AND session_id NOT LIKE '%:agent:%'
      AND session_id NOT IN (
        SELECT session_id FROM chat.requests WHERE parent_session_id != '' AND ts >= lookback
      )
  )",
        since = sql_time(window.since),
        until = sql_time(window.until),
    )
}

/// The first sighting of each user message node, which is one message at one
/// place in a conversation, from a main agent call, with each content block's
/// type and the start of its text, which is all `typed_blocks` needs; whole
/// messages carry their injected context and run to hundreds of MB. Sightings
/// from before the window come back too, so `build_prompts` can pair the
/// copies of a session up by order and find the prompt before the window
fn prompts_sql(window: &Window) -> String {
    format!(
        "{preamble},
  main_requests AS (
    SELECT request_id FROM chat.requests
    WHERE ts >= lookback AND ts < until AND session_id IN (SELECT session_id FROM sessions)
      AND (startsWith(session_id, 'codex:')
        OR tools IN (SELECT hash FROM chat.blobs WHERE kind = 'tools' AND bytes >= {MAIN_AGENT_TOOLS_MIN_BYTES}))
  ),
  firsts AS (
    SELECT session_id, node, any(message) AS message, any(depth) AS depth, min(ts) AS at,
      argMin(request_id, ts) AS request
    FROM chat.nodes
    WHERE role = 'user' AND ts >= lookback AND ts < until
      AND session_id IN (SELECT session_id FROM sessions)
    GROUP BY session_id, node
    HAVING anyIf(request_id, request_id IN (SELECT request_id FROM main_requests)) != ''
  ),
  arrivals AS (
    SELECT session_id, request, count() AS fresh, max(depth) AS deepest
    FROM firsts GROUP BY session_id, request
  ),
  sent AS (
    SELECT f.session_id AS session_id, f.message AS message, f.at AS at
    FROM firsts AS f
    INNER JOIN arrivals AS a ON a.session_id = f.session_id AND a.request = f.request
    WHERE a.fresh <= {REPLAY_NODES} OR f.depth = a.deepest
  )
SELECT f.session_id AS session, f.message AS message, toString(f.at) AS at, b.blocks AS blocks
FROM sent AS f
INNER JOIN (
  SELECT hash,
    any(if(JSONType(content, 'content') = 'String',
      [('text', substring(trimLeft(JSONExtractString(content, 'content')), 1, 64))],
      arrayMap(x -> (JSONExtractString(x, 'type'), substring(trimLeft(JSONExtractString(x, 'text')), 1, 64)),
        JSONExtractArrayRaw(content, 'content')))) AS blocks
  FROM chat.blobs
  WHERE kind = 'message' AND hash IN (SELECT message FROM sent)
  GROUP BY hash
) AS b ON b.hash = f.message
WHERE NOT has(arrayMap(x -> x.1, b.blocks), 'tool_result')
  AND arrayExists(x -> x.1 IN ('text', 'input_text') AND x.2 != '', b.blocks)",
        preamble = preamble(window),
    )
}

/// When each call in those sessions finished, which is when the agent last
/// wrote something
fn replies_sql(window: &Window) -> String {
    format!(
        "{preamble}
SELECT session_id AS session, toString(ts + toIntervalMillisecond(latency_ms)) AS done
FROM chat.requests
WHERE ts >= since AND ts < until + INTERVAL {REPLY_LOOKAHEAD_HOURS} HOUR AND NOT failed
  AND session_id IN (SELECT session_id FROM sessions)",
        preamble = preamble(window),
    )
}

/// Each session's working directory, from the environment block Claude Code
/// puts in a system message and the `<cwd>` Codex puts in its environment
/// context, and whether T3 ran it, which its context says. Stored content is
/// JSON, so `/` arrives escaped as `\/`
fn contexts_sql(window: &Window) -> String {
    format!(
        r#"{preamble},
  heads AS (
    SELECT session_id, message FROM chat.nodes
    WHERE role IN ('system', 'developer', 'user') AND depth <= 8
      AND session_id IN (SELECT session_id FROM sessions)
  )
SELECT h.session_id AS session, anyIf(c.cwd, c.cwd != '') AS cwd, max(c.t3) AS t3
FROM heads AS h
INNER JOIN (
  SELECT hash, any(if(position(content, 'Primary working directory: ') > 0,
      extract(replaceAll(content, '\\/', '/'), 'Primary working directory: ([^\\\\"]+)'),
      extract(replaceAll(content, '\\/', '/'), '<cwd>([^<]+)</cwd>'))) AS cwd,
    toUInt8(max(position(content, '{T3_MARKER}') > 0)) AS t3
  FROM chat.blobs
  WHERE kind = 'message' AND hash IN (SELECT message FROM heads)
    AND (position(content, 'Primary working directory: ') > 0 OR position(content, '<cwd>') > 0
      OR position(content, '{T3_MARKER}') > 0)
  GROUP BY hash
) AS c ON c.hash = h.message
GROUP BY session"#,
        preamble = preamble(window),
    )
}

#[derive(Deserialize)]
struct PromptRow {
    session: String,
    message: String,
    at: String,
    blocks: Vec<(String, String)>,
}

#[derive(Deserialize)]
struct ReplyRow {
    session: String,
    done: String,
}

#[derive(Deserialize)]
struct ContextRow {
    session: String,
    cwd: String,
    t3: u8,
}

/// `claude:<id>` from the proxy and `claude-code:<id>` from an imported
/// transcript are the same Claude Code session
fn session_id(text: &str) -> Option<SessionId> {
    let (prefix, id) = text.split_once(':')?;
    let agent = match prefix {
        "claude" | "claude-code" => Agent::Claude,
        "codex" => Agent::Codex,
        _ => return None,
    };

    Some(SessionId {
        agent,
        id: id.to_owned(),
    })
}

/// ClickHouse prints `DateTime64` in UTC as `2026-11-01 05:00:00.000`
fn timestamp(text: &str) -> Option<DateTime<Utc>> {
    NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S%.f")
        .ok()
        .map(|at| at.and_utc())
}

fn build_prompts(
    window: &Window,
    rows: Vec<PromptRow>,
    replies: Vec<ReplyRow>,
    contexts: Vec<ContextRow>,
) -> Vec<Prompt> {
    let contexts: HashMap<SessionId, ContextRow> = contexts
        .into_iter()
        .filter_map(|row| Some((session_id(&row.session)?, row)))
        .collect();

    // each row is one occurrence of a message, so the same words typed twice
    // stay two prompts. The proxy's copy and an imported transcript's copy of
    // a session share a SessionId but not their nodes, so the nth occurrence
    // of a message in each copy is the same prompt, seen first in the earlier
    let mut occurrences: HashMap<(String, SessionId, String), Vec<DateTime<Utc>>> = HashMap::new();
    for row in rows {
        let typed = typed_blocks(
            row.blocks
                .iter()
                .map(|(kind, text)| (kind.as_str(), text.as_str())),
        );

        let (Some(session), Some(at)) = (session_id(&row.session), timestamp(&row.at)) else {
            continue;
        };

        // nothing in the proxy's record tells a typed prompt from one a script
        // sent, and most sessions no machine reports are one-shot runs such
        // as security reviews, so only T3's sessions, where people type, count
        let from_t3 = contexts.get(&session).is_some_and(|context| context.t3 > 0);
        if !typed || !from_t3 {
            continue;
        }

        occurrences
            .entry((row.session, session, row.message))
            .or_default()
            .push(at);
    }

    let mut firsts: HashMap<(SessionId, String, usize), DateTime<Utc>> = HashMap::new();
    for ((_, session, message), mut times) in occurrences {
        times.sort();
        for (nth, at) in times.into_iter().enumerate() {
            firsts
                .entry((session.clone(), message.clone(), nth))
                .and_modify(|first| *first = (*first).min(at))
                .or_insert(at);
        }
    }

    let mut steps: BTreeMap<SessionId, Vec<Step>> = BTreeMap::new();
    for ((session, ..), at) in firsts {
        steps
            .entry(session)
            .or_default()
            .push(Step { at, prompt: true });
    }

    for reply in replies {
        let (Some(session), Some(at)) = (session_id(&reply.session), timestamp(&reply.done)) else {
            continue;
        };

        if let Some(session_steps) = steps.get_mut(&session) {
            session_steps.push(Step { at, prompt: false });
        }
    }

    let mut prompts = Vec::new();
    for (session, mut session_steps) in steps {
        // a prompt and the call that sent it share a timestamp, so prompts
        // sort first and the call's finish counts as the reply
        session_steps.sort_by_key(|step| (step.at, !step.prompt));
        let place = contexts
            .get(&session)
            .map(|context| context.cwd.as_str())
            .filter(|cwd| !cwd.is_empty());
        let conversation = Conversation::Session(session.clone());
        prompts.extend(prompts_with_replies(
            &session_steps,
            window,
            &conversation,
            Source::ClickHouse,
            place,
        ));
    }

    prompts
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone as _;

    use super::*;

    /// A sighting at `HH:MM` on November 2, or at `DD HH:MM` in November
    fn prompt(session: &str, message: &str, at: &str, text: &str) -> PromptRow {
        let at = match at.len() {
            5 => format!("2026-11-02 {at}:00.000"),
            _ => format!("2026-11-{at}:00.000"),
        };

        PromptRow {
            session: session.to_owned(),
            message: message.to_owned(),
            at,
            blocks: vec![("text".to_owned(), text.to_owned())],
        }
    }

    fn context(session: &str, cwd: &str, t3: u8) -> ContextRow {
        ContextRow {
            session: session.to_owned(),
            cwd: cwd.to_owned(),
            t3,
        }
    }

    #[test]
    fn only_t3_sessions_count_and_each_typed_prompt_counts_once() {
        let window = Window {
            since: Utc.with_ymd_and_hms(2026, 11, 2, 0, 0, 0).unwrap(),
            until: Utc.with_ymd_and_hms(2026, 11, 3, 0, 0, 0).unwrap(),
        };

        let rows = vec![
            // the proxy's sighting and the later imported copy of one message
            prompt("claude:t3", "m1", "10:00", "fix the bug"),
            prompt("claude-code:t3", "m1", "10:30", "fix the bug"),
            prompt("claude:t3", "m2", "10:20", "now the tests"),
            // the same words typed again later, and the import's copy of them
            prompt("claude:t3", "m1", "10:40", "fix the bug"),
            prompt("claude-code:t3", "m1", "10:50", "fix the bug"),
            // typed the day before, so it only orders the occurrences
            prompt("claude:t3", "m0", "01 09:00", "start"),
            // a scripted review run with no T3 context
            prompt("claude:script", "m3", "11:00", "Review this change"),
        ];
        let replies = vec![ReplyRow {
            session: "claude:t3".to_owned(),
            done: "2026-11-02 10:05:00.000".to_owned(),
        }];
        let contexts = vec![
            context("claude:t3", "/home/user/code/app", 1),
            context("claude:script", "/home/user/code/app", 0),
        ];

        let prompts = build_prompts(&window, rows, replies, contexts);
        let summary: Vec<_> = prompts
            .iter()
            .map(|prompt| {
                (
                    prompt.at.format("%H:%M").to_string(),
                    prompt.place.as_deref(),
                    prompt.reply_done.map(|at| at.format("%H:%M").to_string()),
                )
            })
            .collect();

        assert_eq!(
            summary,
            [
                (
                    "10:00".to_owned(),
                    Some("/home/user/code/app"),
                    Some("10:05".to_owned())
                ),
                ("10:20".to_owned(), Some("/home/user/code/app"), None),
                ("10:40".to_owned(), Some("/home/user/code/app"), None),
            ]
        );
    }
}
