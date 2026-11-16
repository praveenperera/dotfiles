//! Typed prompts, agent replies, and thread views from this machine's T3 state
//! database, opened read-only since T3 keeps writing to it

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::time::Duration;

use chrono::{DateTime, SecondsFormat, TimeZone as _, Utc};
use eyre::{Result, WrapErr};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension as _};

use super::model::{
    prompts_with_replies, Agent, Conversation, Driver, KnownSession, Prompt, SessionId, Source,
    SourceData, Step, View, ViewKind, Window,
};
use crate::fsutil;

/// How far past the window agent replies are read, so the last prompt in it
/// still gets its reply time
const REPLY_LOOKAHEAD: chrono::Duration = chrono::Duration::hours(6);

/// `creationSource` values of messages a person typed; `server` (Homebased
/// events), `mcp` (messages between threads), and `provider` (agent-written)
/// are not, even though Homebased events carry `createdBy: "user"`
const TYPED_SOURCES: &[&str] = &["web", "mobile"];

fn db_path() -> Result<PathBuf> {
    let home = match std::env::var_os("T3CODE_HOME") {
        Some(home) => PathBuf::from(home),
        None => fsutil::home_dir()?.join(".t3"),
    };

    Ok(home.join("userdata/statev2.sqlite"))
}

/// Read the window from T3, or `None` when this machine has no T3 database
pub fn collect(window: &Window) -> Result<Option<SourceData>> {
    let path = db_path()?;
    if !path.is_file() {
        return Ok(None);
    }

    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .wrap_err_with(|| format!("open {}", path.display()))?;
    conn.busy_timeout(Duration::from_secs(10))?;

    let threads = threads(&conn)?;
    let mut data = SourceData {
        projects: threads
            .values()
            .filter_map(|thread| thread.project.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        ..SourceData::default()
    };

    data.sessions = sessions(&conn, &threads)?;
    data.prompts = prompts(&conn, window, &threads)?;
    data.views = views(&conn, window, &threads)?;
    Ok(Some(data))
}

struct Thread {
    project: Option<String>,
    created: DateTime<Utc>,
    /// The latest `opens.openedAt`, which survives T3 pruning its event log
    opened: Option<DateTime<Utc>>,
}

fn timestamp(text: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|at| at.with_timezone(&Utc))
}

/// SQLite compares T3's timestamps as text, which works because T3 writes them
/// all as UTC with milliseconds and a `Z`
fn sql_time(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn millis(value: Option<i64>) -> Option<DateTime<Utc>> {
    value.and_then(|ms| Utc.timestamp_millis_opt(ms).single())
}

fn threads(conn: &Connection) -> Result<HashMap<String, Thread>> {
    let mut statement = conn.prepare(
        "SELECT t.thread_id, t.created_at, p.workspace_root,
                CAST(json_extract(t.payload_json, '$.opens.openedAt') AS INTEGER)
         FROM orchestration_v2_projection_threads AS t
         LEFT JOIN projection_projects AS p ON p.project_id = t.project_id",
    )?;

    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<i64>>(3)?,
        ))
    })?;

    let mut threads = HashMap::new();
    for row in rows {
        let (id, created, project, opened) = row?;
        let Some(created) = timestamp(&created) else {
            continue;
        };

        let opened = millis(opened);
        threads.insert(
            id,
            Thread {
                project,
                created,
                opened,
            },
        );
    }

    Ok(threads)
}

/// The Claude and Codex sessions behind each thread, so the same session in a
/// transcript or ClickHouse is known to be T3's
fn sessions(conn: &Connection, threads: &HashMap<String, Thread>) -> Result<Vec<KnownSession>> {
    let mut statement = conn.prepare(
        "SELECT thread_id, driver, json_extract(payload_json, '$.nativeThreadRef.nativeId')
         FROM orchestration_v2_projection_provider_threads
         WHERE thread_id IS NOT NULL",
    )?;

    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    })?;

    let mut sessions = Vec::new();
    for row in rows {
        let (thread, driver, native) = row?;
        let agent = match driver.as_deref() {
            Some("claudeAgent") => Agent::Claude,
            Some("codex") => Agent::Codex,
            _ => continue,
        };

        let (Some(native), Some(created)) = (native, threads.get(&thread).map(|t| t.created))
        else {
            continue;
        };

        sessions.push(KnownSession {
            session: SessionId { agent, id: native },
            driver: Driver::T3 { thread, created },
        });
    }

    Ok(sessions)
}

struct Message {
    thread: String,
    typed: bool,
    created: DateTime<Utc>,
    updated: DateTime<Utc>,
}

fn prompts(
    conn: &Connection,
    window: &Window,
    threads: &HashMap<String, Thread>,
) -> Result<Vec<Prompt>> {
    let mut statement = conn.prepare(
        "SELECT thread_id, role, json_extract(payload_json, '$.creationSource'), created_at, updated_at
         FROM orchestration_v2_projection_messages
         WHERE created_at >= ?1 AND created_at < ?2
         ORDER BY thread_id, created_at",
    )?;

    let until = window.until + REPLY_LOOKAHEAD;
    let rows = statement.query_map(params![sql_time(window.since), sql_time(until)], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;

    let mut messages = Vec::new();
    for row in rows {
        let (thread, role, origin, created, updated) = row?;
        let typed = role == "user"
            && origin
                .as_deref()
                .is_some_and(|o| TYPED_SOURCES.contains(&o));
        // only typed prompts and the agent's own output matter; other user-role
        // messages are events that neither start nor end a reply
        if !typed && role != "assistant" {
            continue;
        }

        let (Some(created), Some(updated)) = (timestamp(&created), timestamp(&updated)) else {
            continue;
        };

        messages.push(Message {
            thread,
            typed,
            created,
            updated,
        });
    }

    // the last prompt before the window, whose reply can end inside it
    let typed = TYPED_SOURCES
        .iter()
        .map(|source| format!("'{source}'"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut before = conn.prepare(&format!(
        "SELECT created_at FROM orchestration_v2_projection_messages
         WHERE thread_id = ?1 AND created_at < ?2 AND role = 'user'
           AND json_extract(payload_json, '$.creationSource') IN ({typed})
         ORDER BY created_at DESC LIMIT 1"
    ))?;

    let mut prompts = Vec::new();
    for thread_messages in messages.chunk_by(|a, b| a.thread == b.thread) {
        let thread = &thread_messages[0].thread;
        let place = threads.get(thread).and_then(|t| t.project.as_deref());
        let earlier = before
            .query_row(params![thread, sql_time(window.since)], |row| {
                row.get::<_, String>(0)
            })
            .optional()?
            .and_then(|created| timestamp(&created))
            .map(|at| Step { at, prompt: true });

        // a reply's own message keeps growing while it streams, so its end is
        // when the message was last updated
        let steps: Vec<Step> = earlier
            .into_iter()
            .chain(thread_messages.iter().map(|message| Step {
                at: if message.typed {
                    message.created
                } else {
                    message.updated
                },
                prompt: message.typed,
            }))
            .collect();

        let conversation = Conversation::Thread(thread.clone());
        prompts.extend(prompts_with_replies(
            &steps,
            window,
            &conversation,
            Source::T3,
            place,
        ));
    }

    Ok(prompts)
}

fn views(
    conn: &Connection,
    window: &Window,
    threads: &HashMap<String, Thread>,
) -> Result<Vec<View>> {
    // no index covers event_type, so this scans the event log; T3 prunes
    // visits older than about a day to each thread's last, which keeps the
    // scan short even in a multi-GB database
    let mut statement = conn.prepare(
        "SELECT stream_id, occurred_at,
                CAST(json_extract(payload_json, '$.opens.openedAt') AS INTEGER)
         FROM orchestration_events
         WHERE event_type = 'thread.visited' AND occurred_at >= ?1 AND occurred_at < ?2",
    )?;

    let rows = statement.query_map(
        params![sql_time(window.since), sql_time(window.until)],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<i64>>(2)?,
            ))
        },
    )?;

    let place = |thread: &str| threads.get(thread).and_then(|t| t.project.clone());
    let mut opens = BTreeSet::new();
    let mut views = Vec::new();
    for row in rows {
        let (thread, occurred, opened) = row?;
        if let Some(opened) = millis(opened) {
            opens.insert((thread.clone(), opened));
        }

        let Some(at) = timestamp(&occurred) else {
            continue;
        };

        views.push(View {
            at,
            place: place(&thread),
            thread,
            kind: ViewKind::Visit,
        });
    }

    // every visit repeats the thread's last open, and the thread row keeps the
    // newest one after the events are pruned
    for (thread, data) in threads {
        if let Some(opened) = data.opened {
            opens.insert((thread.clone(), opened));
        }
    }

    views.extend(
        opens
            .into_iter()
            .filter(|(_, at)| window.contains(*at))
            .map(|(thread, at)| View {
                at,
                place: place(&thread),
                thread,
                kind: ViewKind::Open,
            }),
    );

    Ok(views)
}
