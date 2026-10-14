use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use eyre::{eyre, Context, Result};
use serde_json::Value;

use crate::fsutil;

/// How far back discovery looks when no session is named
const DISCOVERY_WINDOW: Duration = Duration::from_secs(14 * 24 * 60 * 60);
const TITLE_MAX_CHARS: usize = 80;

/// Originators of terminal sessions: stock `codex` and the tmux wrapper in `cmd codex`
const CODEX_TUI_ORIGINATORS: &[&str] = &["codex-tui", crate::cmd::codex::app_server::CLIENT_NAME];

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Provider {
    Claude,
    Codex,
}

impl Provider {
    /// T3 provider instance that owns threads for this CLI
    pub fn instance_id(self) -> &'static str {
        match self {
            Self::Claude => "claudeAgent",
            Self::Codex => "codex",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
}

/// A CLI conversation read from its on-disk transcript
#[derive(Debug, Clone)]
pub struct Session {
    pub provider: Provider,
    pub id: String,
    pub cwd: PathBuf,
    /// Started from the terminal UI rather than an SDK, `-p`, `exec`, or another app
    pub interactive: bool,
    pub model: Option<String>,
    pub title: Option<String>,
    pub has_prompt: bool,
}

impl Session {
    pub fn read(provider: Provider, path: &Path) -> Result<Self> {
        match provider {
            Provider::Claude => read_claude(path),
            Provider::Codex => read_codex(path),
        }
    }

    /// Guess the provider from where the transcript lives
    pub fn read_path(path: &Path) -> Result<Self> {
        let is_codex = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("rollout-"));

        let provider = if is_codex {
            Provider::Codex
        } else {
            Provider::Claude
        };

        Self::read(provider, path)
    }

    /// Find a transcript by session id in either CLI's history
    pub fn find_by_id(id: &str, provider: Option<Provider>) -> Result<Self> {
        if provider != Some(Provider::Codex) {
            if let Some(path) = find_claude_transcript(id)? {
                return read_claude(&path);
            }
        }

        if provider != Some(Provider::Claude) {
            if let Some(path) = find_codex_rollout(id)? {
                return read_codex(&path);
            }
        }

        Err(eyre!(
            "no Claude or Codex transcript found for session {id}"
        ))
    }

    /// Newest interactive session that ran in `cwd`
    pub fn newest_in(cwd: &Path, provider: Option<Provider>) -> Result<Option<Self>> {
        let mut candidates = Vec::new();
        if provider != Some(Provider::Codex) {
            candidates.extend(claude_transcripts_for(cwd)?);
        }

        if provider != Some(Provider::Claude) {
            candidates.extend(codex_rollouts_for(cwd)?);
        }

        candidates.sort_by_key(|(_, modified, _)| *modified);
        while let Some((provider, _, path)) = candidates.pop() {
            let Ok(session) = Session::read(provider, &path) else {
                continue;
            };

            if session.interactive && session.has_prompt && same_dir(&session.cwd, cwd) {
                return Ok(Some(session));
            }
        }

        Ok(None)
    }
}

pub fn same_dir(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn claude_home() -> Result<PathBuf> {
    match std::env::var_os("CLAUDE_CONFIG_DIR") {
        Some(dir) => Ok(PathBuf::from(dir)),
        None => Ok(fsutil::home_dir()?.join(".claude")),
    }
}

fn codex_home() -> Result<PathBuf> {
    match std::env::var_os("CODEX_HOME") {
        Some(dir) => Ok(PathBuf::from(dir)),
        None => Ok(fsutil::home_dir()?.join(".codex")),
    }
}

/// JSON records, one per line. Invalid UTF-8 is replaced rather than ending the
/// read, so a later title or model record is not lost
fn read_lines(path: &Path) -> Result<impl Iterator<Item = Value>> {
    let file = File::open(path).wrap_err_with(|| format!("open {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();

    Ok(std::iter::from_fn(move || {
        buffer.clear();
        match reader.read_until(b'\n', &mut buffer) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(String::from_utf8_lossy(&buffer).into_owned()),
        }
    })
    .filter_map(|line| serde_json::from_str(&line).ok()))
}

fn str_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn read_claude(path: &Path) -> Result<Session> {
    let mut id = None;
    let mut cwd = None;
    let mut entrypoint = None;
    let mut model = None;
    let mut custom_title = None;
    let mut ai_title = None;
    let mut first_prompt = None;

    for record in read_lines(path)? {
        if id.is_none() {
            id = str_field(&record, "sessionId").map(str::to_owned);
        }

        if cwd.is_none() {
            cwd = str_field(&record, "cwd").map(PathBuf::from);
        }

        if entrypoint.is_none() {
            entrypoint = str_field(&record, "entrypoint").map(str::to_owned);
        }

        match str_field(&record, "type") {
            Some("custom-title") => {
                custom_title = str_field(&record, "customTitle").map(str::to_owned);
            }
            Some("ai-title") => ai_title = str_field(&record, "aiTitle").map(str::to_owned),
            Some("assistant") => {
                // synthetic messages are local errors and interrupts, not a real model
                if let Some(name) = record
                    .pointer("/message/model")
                    .and_then(Value::as_str)
                    .filter(|name| !name.starts_with('<'))
                {
                    model = Some(name.to_owned());
                }
            }
            Some("user") if first_prompt.is_none() => {
                let skipped = record.get("isMeta").and_then(Value::as_bool) == Some(true)
                    || record.get("isSidechain").and_then(Value::as_bool) == Some(true);

                if !skipped {
                    first_prompt = record.pointer("/message/content").and_then(prompt_text);
                }
            }
            _ => {}
        }
    }

    let id = id
        .or_else(|| file_stem(path))
        .ok_or_else(|| eyre!("{} has no session id", path.display()))?;
    let cwd = cwd.ok_or_else(|| eyre!("{} has no cwd", path.display()))?;

    Ok(Session {
        provider: Provider::Claude,
        id,
        cwd,
        interactive: entrypoint.as_deref() == Some("cli"),
        model,
        has_prompt: first_prompt.is_some(),
        title: custom_title
            .and_then(short_title)
            .or_else(|| ai_title.and_then(short_title))
            .or_else(|| first_prompt.and_then(short_title)),
    })
}

fn read_codex(path: &Path) -> Result<Session> {
    let mut meta = None;
    let mut model = None;
    let mut first_prompt = None;

    for record in read_lines(path)? {
        let payload = record.get("payload").cloned().unwrap_or(Value::Null);
        match str_field(&record, "type") {
            Some("session_meta") if meta.is_none() => meta = Some(payload),
            Some("turn_context") => {
                if let Some(name) = str_field(&payload, "model") {
                    model = Some(name.to_owned());
                }
            }
            Some("event_msg")
                if first_prompt.is_none()
                    && str_field(&payload, "type") == Some("user_message") =>
            {
                first_prompt = payload.get("message").and_then(prompt_text);
            }
            // older rollouts only record the prompt as a model input item
            Some("response_item")
                if first_prompt.is_none()
                    && str_field(&payload, "type") == Some("message")
                    && str_field(&payload, "role") == Some("user") =>
            {
                first_prompt = payload.get("content").and_then(prompt_text);
            }
            _ => {}
        }
    }

    let meta = meta.ok_or_else(|| eyre!("{} has no session_meta", path.display()))?;
    let id = str_field(&meta, "id")
        .map(str::to_owned)
        .ok_or_else(|| eyre!("{} has no session id", path.display()))?;
    let cwd = str_field(&meta, "cwd")
        .map(PathBuf::from)
        .ok_or_else(|| eyre!("{} has no cwd", path.display()))?;

    // the TUI now runs through the shared app-server, so its rollouts say source
    // "vscode" like T3 and Codex Desktop do, and only the originator tells them
    // apart. Older TUI builds wrote source "cli"
    let interactive = matches!(
        str_field(&meta, "originator"),
        Some(originator) if CODEX_TUI_ORIGINATORS.contains(&originator)
    ) || str_field(&meta, "source") == Some("cli");
    let title = codex_thread_name(&id)
        .and_then(short_title)
        .or_else(|| first_prompt.clone().and_then(short_title));

    Ok(Session {
        provider: Provider::Codex,
        id,
        cwd,
        interactive,
        model,
        has_prompt: first_prompt.is_some(),
        title,
    })
}

/// First line of user-typed text, ignoring injected context blocks
fn prompt_text(content: &Value) -> Option<String> {
    let text = match content {
        Value::String(text) => text.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter(|block| matches!(str_field(block, "type"), Some("text" | "input_text")))
            .filter_map(|block| str_field(block, "text"))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => return None,
    };

    let text = text.trim();
    // slash-command wrappers, hook output, and environment context all start with
    // a tag, and Codex injects AGENTS.md as a user message
    if text.is_empty() || text.starts_with('<') || text.starts_with("# AGENTS.md") {
        return None;
    }

    Some(text.to_owned())
}

/// First non-blank line, shortened. T3 rejects blank titles, so blank text is None
fn short_title(text: String) -> Option<String> {
    let line = text.lines().map(str::trim).find(|line| !line.is_empty())?;
    if line.chars().count() <= TITLE_MAX_CHARS {
        return Some(line.to_owned());
    }

    let cut: String = line.chars().take(TITLE_MAX_CHARS - 1).collect();
    Some(format!("{}…", cut.trim_end()))
}

fn file_stem(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::to_owned)
}

/// Codex keeps renamed and generated thread names in an append-only index
fn codex_thread_name(id: &str) -> Option<String> {
    let index = codex_home().ok()?.join("session_index.jsonl");
    read_lines(&index)
        .ok()?
        .filter(|entry| str_field(entry, "id") == Some(id))
        .filter_map(|entry| str_field(&entry, "thread_name").map(str::to_owned))
        .last()
}

/// Claude Code stores each project's transcripts under the cwd with every
/// non-alphanumeric character replaced by a dash
fn claude_project_dir(cwd: &Path) -> Result<PathBuf> {
    let encoded: String = cwd
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();

    Ok(claude_home()?.join("projects").join(encoded))
}

fn find_claude_transcript(id: &str) -> Result<Option<PathBuf>> {
    let projects = claude_home()?.join("projects");
    let Ok(entries) = fs::read_dir(&projects) else {
        return Ok(None);
    };

    let file_name = format!("{id}.jsonl");
    Ok(entries
        .flatten()
        .map(|entry| entry.path().join(&file_name))
        .find(|path| path.is_file()))
}

fn claude_transcripts_for(cwd: &Path) -> Result<Vec<(Provider, SystemTime, PathBuf)>> {
    let mut found = Vec::new();
    let mut dirs = vec![claude_project_dir(cwd)?];
    if let Ok(real) = fs::canonicalize(cwd) {
        dirs.push(claude_project_dir(&real)?);
    }

    dirs.dedup();
    for dir in dirs {
        found.extend(recent_files(&dir, |name| name.ends_with(".jsonl"))?);
    }

    Ok(found
        .into_iter()
        .map(|(modified, path)| (Provider::Claude, modified, path))
        .collect())
}

/// Walk `sessions/YYYY/MM/DD` newest first, stopping past the discovery window
fn codex_day_dirs() -> Result<Vec<PathBuf>> {
    let sessions = codex_home()?.join("sessions");
    let mut days = Vec::new();
    for year in sorted_dirs(&sessions) {
        for month in sorted_dirs(&year) {
            days.extend(sorted_dirs(&month));
        }
    }

    days.sort();
    days.reverse();
    Ok(days)
}

fn find_codex_rollout(id: &str) -> Result<Option<PathBuf>> {
    let suffix = format!("{id}.jsonl");
    for day in codex_day_dirs()? {
        let Ok(entries) = fs::read_dir(&day) else {
            continue;
        };

        if let Some(path) = entries.flatten().map(|entry| entry.path()).find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("rollout-") && name.ends_with(&suffix))
        }) {
            return Ok(Some(path));
        }
    }

    Ok(None)
}

fn codex_rollouts_for(cwd: &Path) -> Result<Vec<(Provider, SystemTime, PathBuf)>> {
    let mut found = Vec::new();
    // resumed sessions append to their original day's rollout, so every day is
    // checked and only file mtimes decide what counts as recent
    for day in codex_day_dirs()? {
        let files = recent_files(&day, |name| {
            name.starts_with("rollout-") && name.ends_with(".jsonl")
        })?;

        // the first record is session_meta, so the cwd check reads one line per rollout
        for (modified, path) in files {
            let Ok(mut records) = read_lines(&path) else {
                continue;
            };

            let first = records.next();
            let rollout_cwd = first
                .as_ref()
                .and_then(|record| record.pointer("/payload/cwd"))
                .and_then(Value::as_str);

            if rollout_cwd.is_some_and(|dir| same_dir(Path::new(dir), cwd)) {
                found.push((Provider::Codex, modified, path));
            }
        }
    }

    Ok(found)
}

fn sorted_dirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut dirs: Vec<_> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();

    dirs.sort();
    dirs
}

fn recent_files(dir: &Path, keep: impl Fn(&str) -> bool) -> Result<Vec<(SystemTime, PathBuf)>> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(Vec::new());
    };

    let cutoff = SystemTime::now() - DISCOVERY_WINDOW;
    Ok(entries
        .flatten()
        .filter(|entry| entry.file_name().to_str().is_some_and(&keep))
        .filter_map(|entry| {
            let modified = entry.metadata().and_then(|meta| meta.modified()).ok()?;
            (modified >= cutoff).then(|| (modified, entry.path()))
        })
        .collect())
}
