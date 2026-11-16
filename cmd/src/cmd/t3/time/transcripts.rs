//! Prompts from the Claude Code transcripts and Codex rollouts on this
//! machine, which cover terminal sessions that bypass both T3 and the proxy

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use eyre::Result;
use serde_json::Value;

use super::model::{
    prompts_with_replies, typed_content, Agent, Conversation, Driver, KnownSession, Prompt,
    SessionId, Source, SourceData, Step, Window,
};
use crate::cmd::t3::transcript::{
    claude_home, codex_home, read_lines, sorted_dirs, str_field, CODEX_TUI_ORIGINATORS,
};

/// Read Claude Code transcripts changed since the window opened, or `None`
/// when this machine has no Claude Code history
pub fn collect_claude(window: &Window) -> Result<Option<SourceData>> {
    let projects = claude_home()?.join("projects");
    if !projects.is_dir() {
        return Ok(None);
    }

    // subagent transcripts sit one level deeper and are not conversations
    // anyone typed into, so only each project's top-level files are read
    let files: Vec<PathBuf> = sorted_dirs(&projects)
        .iter()
        .flat_map(|dir| changed_since(dir, window, |name| name.ends_with(".jsonl")))
        .collect();

    Ok(Some(gather(parallel_map(&files, |path| {
        read_claude(path, window)
    }))))
}

/// Read Codex rollouts changed since the window opened, or `None` when this
/// machine has no Codex history
pub fn collect_codex(window: &Window) -> Result<Option<SourceData>> {
    let sessions = codex_home()?.join("sessions");
    if !sessions.is_dir() {
        return Ok(None);
    }

    // a resumed session appends to the rollout of the day it started, so every
    // day is listed and the file's mtime decides
    let mut files = Vec::new();
    for year in sorted_dirs(&sessions) {
        for month in sorted_dirs(&year) {
            for day in sorted_dirs(&month) {
                files.extend(changed_since(&day, window, |name| {
                    name.starts_with("rollout-") && name.ends_with(".jsonl")
                }));
            }
        }
    }

    Ok(Some(gather(parallel_map(&files, |path| {
        read_codex(path, window)
    }))))
}

/// One transcript's session and the prompts typed in it
struct Transcript {
    session: KnownSession,
    prompts: Vec<Prompt>,
}

fn gather(transcripts: Vec<Transcript>) -> SourceData {
    let mut data = SourceData::default();
    for transcript in transcripts {
        data.prompts.extend(transcript.prompts);
        data.sessions.push(transcript.session);
    }

    data
}

fn changed_since(dir: &Path, window: &Window, keep: impl Fn(&str) -> bool) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let since = SystemTime::from(window.since);
    entries
        .flatten()
        .filter(|entry| entry.file_name().to_str().is_some_and(&keep))
        .filter(|entry| {
            entry
                .metadata()
                .and_then(|meta| meta.modified())
                .is_ok_and(|modified| modified >= since)
        })
        .map(|entry| entry.path())
        .collect()
}

/// Parse files across threads; sizes vary widely, so each worker pulls the
/// next file instead of taking a fixed share
fn parallel_map<T: Send>(paths: &[PathBuf], read: impl Fn(&Path) -> Option<T> + Sync) -> Vec<T> {
    let workers = std::thread::available_parallelism()
        .map_or(4, std::num::NonZero::get)
        .min(paths.len());

    let next = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                scope.spawn(|| {
                    let mut found = Vec::new();
                    while let Some(path) = paths.get(next.fetch_add(1, Ordering::Relaxed)) {
                        found.extend(read(path));
                    }

                    found
                })
            })
            .collect();

        handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap_or_default())
            .collect()
    })
}

fn timestamp(record: &Value) -> Option<DateTime<Utc>> {
    let text = str_field(record, "timestamp")?;
    DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|at| at.with_timezone(&Utc))
}

fn flag(record: &Value, key: &str) -> bool {
    record.get(key).and_then(Value::as_bool) == Some(true)
}

fn read_claude(path: &Path, window: &Window) -> Option<Transcript> {
    let mut id = None;
    let mut cwd = None;
    let mut entrypoint = None;
    let mut steps = Vec::new();

    for record in read_lines(path).ok()? {
        if id.is_none() {
            id = str_field(&record, "sessionId").map(str::to_owned);
        }

        if cwd.is_none() {
            cwd = str_field(&record, "cwd").map(str::to_owned);
        }

        if entrypoint.is_none() {
            entrypoint = str_field(&record, "entrypoint").map(str::to_owned);
        }

        // T3 and other SDK hosts write transcripts too, and only the terminal
        // UI is someone typing straight into Claude Code. Those transcripts
        // are most of the history by size, so reading stops once one shows
        if let (Some(id), Some(entrypoint)) = (&id, &entrypoint) {
            if entrypoint != "cli" {
                return Some(automated(SessionId {
                    agent: Agent::Claude,
                    id: id.clone(),
                }));
            }
        }

        let Some(at) = timestamp(&record) else {
            continue;
        };

        let prompt = str_field(&record, "type") == Some("user")
            && !flag(&record, "isSidechain")
            && !flag(&record, "isMeta")
            && !flag(&record, "isCompactSummary")
            && record
                .pointer("/message/content")
                .is_some_and(typed_content);

        steps.push(Step { at, prompt });
    }

    let session = SessionId {
        agent: Agent::Claude,
        id: id?,
    };

    // a transcript that never names its entrypoint predates the field, and
    // only terminal sessions wrote transcripts then
    Some(Transcript {
        prompts: prompts_with_replies(
            &steps,
            window,
            &Conversation::Session(session.clone()),
            Source::ClaudeCode,
            cwd.as_deref(),
        ),
        session: KnownSession {
            session,
            driver: Driver::Terminal,
        },
    })
}

fn automated(session: SessionId) -> Transcript {
    Transcript {
        session: KnownSession {
            session,
            driver: Driver::Automated,
        },
        prompts: Vec::new(),
    }
}

fn read_codex(path: &Path, window: &Window) -> Option<Transcript> {
    let mut records = read_lines(path).ok()?;
    let meta = records.next()?;
    let meta = meta.get("payload")?;
    let session = SessionId {
        agent: Agent::Codex,
        id: str_field(meta, "id")?.to_owned(),
    };

    let cwd = str_field(meta, "cwd").map(str::to_owned);
    // a subagent's source is an object naming its parent, so only a plain
    // "cli" source or a terminal originator marks a session someone typed into
    let terminal = matches!(
        str_field(meta, "originator"),
        Some(originator) if CODEX_TUI_ORIGINATORS.contains(&originator)
    ) || str_field(meta, "source") == Some("cli");

    if !terminal {
        return Some(automated(session));
    }

    // newer rollouts mark typed prompts with a user_message event; older ones
    // only record the model input item, so that is the fallback
    let mut events = Vec::new();
    let mut items = Vec::new();
    for record in records {
        let Some(at) = timestamp(&record) else {
            continue;
        };

        let payload = record.get("payload").unwrap_or(&Value::Null);
        let event = str_field(&record, "type") == Some("event_msg")
            && str_field(payload, "type") == Some("user_message");
        let item = str_field(&record, "type") == Some("response_item")
            && str_field(payload, "type") == Some("message")
            && str_field(payload, "role") == Some("user")
            && payload.get("content").is_some_and(typed_content);

        events.push(Step { at, prompt: event });
        items.push(Step { at, prompt: item });
    }

    let steps = match events.iter().any(|step| step.prompt) {
        true => events,
        false => items,
    };

    Some(Transcript {
        prompts: prompts_with_replies(
            &steps,
            window,
            &Conversation::Session(session.clone()),
            Source::Codex,
            cwd.as_deref(),
        ),
        session: KnownSession {
            session,
            driver: Driver::Terminal,
        },
    })
}
