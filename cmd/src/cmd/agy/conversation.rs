//! Reading one Antigravity conversation database

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use eyre::{Result, WrapErr};
use prost::Message;
use rusqlite::{Connection, OpenFlags};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::proto::{GeneratorMetadata, Step, StepMetadata, TrajectoryMetadata};

/// One conversation (Antigravity calls it a cascade): its steps and the model calls that made them
pub struct Conversation {
    /// the cascade id, which is also the database's file name
    pub id: String,
    /// the conversation that started this one, for a subagent
    pub parent_id: Option<String>,
    pub steps: BTreeMap<u32, Step>,
    /// model calls in the order Antigravity made them, keyed by their `gen_metadata` index
    pub calls: Vec<(u32, GeneratorMetadata)>,
}

impl Conversation {
    /// Reads a conversation database, which Antigravity may be writing to
    pub fn load(path: &Path) -> Result<Self> {
        let id = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_owned();

        // the database is in WAL mode, so a read-only open needs a writable -shm file next to it,
        // and opening it read-write could checkpoint Antigravity's WAL under it; a private copy
        // avoids both, and SQLite drops a torn last WAL frame when it opens the copy
        let scratch = tempfile::tempdir().wrap_err("creating a scratch directory")?;
        let copy = scratch.path().join("conversation.db");
        fs::copy(path, &copy).wrap_err_with(|| format!("copying {}", path.display()))?;

        let wal = path.with_extension("db-wal");
        if wal.exists() {
            fs::copy(&wal, scratch.path().join("conversation.db-wal"))
                .wrap_err_with(|| format!("copying {}", wal.display()))?;
        }

        let db = Connection::open_with_flags(&copy, OpenFlags::SQLITE_OPEN_READ_WRITE)
            .wrap_err_with(|| format!("opening {}", path.display()))?;

        let parent_id = read_parent(&db)?;
        let steps = read_steps(&db)?;
        let calls = read_calls(&db)?;
        Ok(Self {
            id,
            parent_id,
            steps,
            calls,
        })
    }
}

/// The Google account T3's Antigravity provider signs in with, for the conversations in `dir`, as
/// `gemini-<hash>-<email>.json` like the proxy's auth files and gemini-quota.sh's chat.quota rows.
/// T3 keeps the login beside the conversations; the agy CLI keeps its own elsewhere, so its
/// conversations have no account
pub fn login_account(dir: &Path) -> Option<String> {
    #[derive(Deserialize)]
    struct Login {
        id_token_email: String,
    }

    let path = dir.parent()?.join("antigravity-oauth-token");
    let login: Login = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    let email = login.id_token_email;
    if email.is_empty() {
        return None;
    }

    let hash = hex::encode(Sha256::digest(email.as_bytes()));
    Some(format!("gemini-{}-{email}.json", &hash[..8]))
}

fn read_parent(db: &Connection) -> Result<Option<String>> {
    let mut query = db.prepare("SELECT data FROM trajectory_metadata_blob LIMIT 1")?;
    let mut rows = query.query([])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };

    let data: Vec<u8> = row.get(0)?;
    let metadata =
        TrajectoryMetadata::decode(data.as_slice()).wrap_err("decoding trajectory metadata")?;
    Ok(Some(metadata.parent_conversation_id).filter(|id| !id.is_empty()))
}

fn read_steps(db: &Connection) -> Result<BTreeMap<u32, Step>> {
    let mut query = db.prepare("SELECT idx, step_payload, metadata FROM steps ORDER BY idx")?;
    let mut rows = query.query([])?;
    let mut steps = BTreeMap::new();
    while let Some(row) = rows.next()? {
        let idx: u32 = row.get(0)?;
        let payload: Vec<u8> = row.get::<_, Option<Vec<u8>>>(1)?.unwrap_or_default();
        let mut step =
            Step::decode(payload.as_slice()).wrap_err_with(|| format!("decoding step {idx}"))?;

        // the payload normally carries its metadata, and the column holds the same message
        if step.metadata.is_none() {
            let metadata: Option<Vec<u8>> = row.get(2)?;
            step.metadata = metadata
                .map(|bytes| StepMetadata::decode(bytes.as_slice()))
                .transpose()
                .wrap_err_with(|| format!("decoding step {idx} metadata"))?;
        }

        steps.insert(idx, step);
    }

    Ok(steps)
}

fn read_calls(db: &Connection) -> Result<Vec<(u32, GeneratorMetadata)>> {
    let mut query = db.prepare("SELECT idx, data FROM gen_metadata ORDER BY idx")?;
    let mut rows = query.query([])?;
    let mut calls = Vec::new();
    while let Some(row) = rows.next()? {
        let idx: u32 = row.get(0)?;
        let data: Vec<u8> = row.get::<_, Option<Vec<u8>>>(1)?.unwrap_or_default();
        let call = GeneratorMetadata::decode(data.as_slice())
            .wrap_err_with(|| format!("decoding generator metadata {idx}"))?;
        calls.push((idx, call));
    }

    Ok(calls)
}
