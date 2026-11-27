//! Antigravity (agy) operations
//!
//! `cmd agy import` loads agy's conversations into the chat history ClickHouse that
//! homelab's ai5090/chat-history/schema.sql defines. agy talks to Google directly, so the CLI
//! proxy that records Claude and Codex calls never sees these; agy keeps each conversation in a
//! SQLite database of protobuf blobs instead, which this reads. See [`calls`] for how a
//! conversation becomes model calls, and [`proto`] for where the field numbers come from

mod calls;
mod clickhouse;
mod conversation;
mod proto;
mod rows;
mod state;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use eyre::{bail, eyre, Result, WrapErr};

use calls::Stats;
use clickhouse::ClickHouse;
use conversation::{login_account, Conversation};
use rows::{Recorder, Session, Tables};
use state::{Fingerprint, State};

/// Buffered rows past this size are uploaded before the next conversation is read
const FLUSH_BYTES: usize = 64 << 20;

#[derive(Debug, Clone, Subcommand)]
pub enum AgyCmd {
    /// Import agy's conversations into the chat history ClickHouse
    ///
    /// Reads every conversation database of the agy CLI (~/.gemini/antigravity-cli) and of T3
    /// Code's Antigravity provider (~/.t3/userdata/providers/antigravity) on this machine, and
    /// uploads the conversations that changed since the last upload as chat.requests rows with
    /// source antigravity, with their messages in chat.nodes and chat.blobs
    #[command(visible_alias = "i")]
    Import(ImportArgs),
}

#[derive(Debug, Clone, Args)]
pub struct ImportArgs {
    /// A directory of conversation databases; repeatable. Defaults to the agy CLI's and T3's
    /// Antigravity directories on this machine
    #[arg(long = "dir")]
    dirs: Vec<PathBuf>,

    #[arg(long, default_value = "http://code.camel-scylla.ts.net:8123")]
    endpoint: String,

    #[arg(long, default_value = "chat")]
    user: String,

    /// Defaults to ~/.config/chat-history/password
    #[arg(long)]
    password_file: Option<PathBuf>,

    /// Read everything and print the summary without uploading
    #[arg(long)]
    dry_run: bool,

    /// Write the rows to <table>.ndjson files in this directory instead of uploading them,
    /// replacing those of an earlier dump
    #[arg(long, conflicts_with = "dry_run")]
    dump: Option<PathBuf>,

    /// Read every database, not only those changed since the last upload
    #[arg(long)]
    all: bool,

    /// Where uploaded databases are recorded. Defaults to
    /// $XDG_STATE_HOME/agy-import/uploaded.json
    #[arg(long)]
    state_file: Option<PathBuf>,
}

pub fn run(subcommand: AgyCmd) -> Result<()> {
    match subcommand {
        AgyCmd::Import(args) => import(&args),
    }
}

enum Sink {
    Upload(ClickHouse),
    Dump(PathBuf),
    Discard,
}

impl Sink {
    fn write(&self, tables: &Tables) -> Result<()> {
        for (table, rows) in tables.in_order() {
            match self {
                Self::Upload(clickhouse) => clickhouse.insert(table, rows)?,
                Self::Dump(dir) => {
                    let path = dir.join(format!("{table}.ndjson"));
                    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
                    file.write_all(rows)?;
                }
                Self::Discard => {}
            }
        }

        Ok(())
    }
}

fn import(args: &ImportArgs) -> Result<()> {
    let home = PathBuf::from(std::env::var_os("HOME").ok_or_else(|| eyre!("HOME is not set"))?);

    let dirs = if args.dirs.is_empty() {
        default_dirs(&home)
    } else {
        args.dirs.clone()
    };
    let files = conversation_files(&dirs)?;
    let state_path = args
        .state_file
        .clone()
        .unwrap_or_else(|| default_state_path(&home));
    let sink = sink(args, &home)?;
    // only an upload is remembered; a dry run or dump reads everything each time
    let remember = matches!(sink, Sink::Upload(_));
    let mut state = if remember && !args.all {
        State::load(&state_path)?
    } else {
        State::default()
    };

    let mut stats = Stats::default();
    let mut recorder = Recorder::default();
    let mut pending = Vec::new();
    let mut accounts: HashMap<PathBuf, Option<String>> = HashMap::new();
    let mut unchanged = 0usize;
    let mut failures = 0usize;
    for path in &files {
        let fingerprint = Fingerprint::of(path);
        if state.unchanged(path, &fingerprint) {
            unchanged += 1;
            continue;
        }

        let conversation = match Conversation::load(path) {
            Ok(conversation) => conversation,
            Err(error) => {
                eprintln!("skipping {}: {error:#}", path.display());
                failures += 1;
                continue;
            }
        };

        let dir = path.parent().unwrap_or(path);
        let account = accounts
            .entry(dir.to_owned())
            .or_insert_with(|| login_account(dir))
            .clone();
        let session = Session::new(&conversation, account);
        for call in calls::calls(&conversation, &mut stats) {
            recorder.add(&session, &call);
        }

        pending.push((path.clone(), fingerprint));
        if recorder.tables.byte_len() >= FLUSH_BYTES {
            let saved = remember.then_some(state_path.as_path());
            flush(&sink, &mut recorder, &mut pending, &mut state, saved)?;
        }
    }

    let saved = remember.then_some(state_path.as_path());
    flush(&sink, &mut recorder, &mut pending, &mut state, saved)?;

    println!(
        "{} databases in {} directories, {unchanged} unchanged since the last upload",
        files.len(),
        dirs.len()
    );
    println!("{stats}");
    let done = match &sink {
        Sink::Upload(_) => "uploaded",
        Sink::Dump(_) => "written",
        Sink::Discard => "found",
    };
    println!(
        "{} requests, {} nodes, {} blobs {done}",
        recorder.request_count, recorder.node_count, recorder.blob_count
    );

    if failures > 0 {
        bail!("{failures} databases could not be read");
    }

    Ok(())
}

/// Uploads the buffered rows, then records the databases they came from
fn flush(
    sink: &Sink,
    recorder: &mut Recorder,
    pending: &mut Vec<(PathBuf, Fingerprint)>,
    state: &mut State,
    state_path: Option<&Path>,
) -> Result<()> {
    sink.write(&recorder.take())?;
    for (path, fingerprint) in pending.drain(..) {
        state.mark(path, fingerprint);
    }

    match state_path {
        Some(path) => state.save(path),
        None => Ok(()),
    }
}

fn sink(args: &ImportArgs, home: &Path) -> Result<Sink> {
    if args.dry_run {
        return Ok(Sink::Discard);
    }

    if let Some(dir) = &args.dump {
        fs::create_dir_all(dir)?;
        // each table's file gets appended to as rows are flushed, so start from empty ones
        for (table, _) in Tables::default().in_order() {
            let path = dir.join(format!("{table}.ndjson"));
            fs::write(&path, b"").wrap_err_with(|| format!("emptying {}", path.display()))?;
        }

        return Ok(Sink::Dump(dir.clone()));
    }

    let password_file = args
        .password_file
        .clone()
        .unwrap_or_else(|| home.join(".config/chat-history/password"));
    let password = fs::read_to_string(&password_file).wrap_err_with(|| {
        format!(
            "reading the ClickHouse password from {}",
            password_file.display()
        )
    })?;

    let clickhouse = ClickHouse::new(&args.endpoint, &args.user, password.trim())?;
    Ok(Sink::Upload(clickhouse))
}

/// The agy CLI keeps conversations under ~/.gemini; T3 runs Antigravity with a directory of its
/// own per provider instance and app, such as antigravity-acp
fn default_dirs(home: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![home.join(".gemini/antigravity-cli/conversations")];
    let providers = home.join(".t3/userdata/providers/antigravity");
    for instance in subdirs(&providers) {
        let apps = subdirs(&instance).into_iter();
        dirs.extend(apps.map(|app| app.join("conversations")));
    }

    dirs.retain(|dir| dir.is_dir());
    dirs
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut dirs: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    dirs
}

fn conversation_files(dirs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for dir in dirs {
        let entries = fs::read_dir(dir).wrap_err_with(|| format!("listing {}", dir.display()))?;
        for entry in entries {
            let path = entry?.path();
            if path.extension().is_some_and(|ext| ext == "db") {
                files.push(path);
            }
        }
    }

    files.sort();
    Ok(files)
}

fn default_state_path(home: &Path) -> PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/state"));
    base.join("agy-import/uploaded.json")
}
