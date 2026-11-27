//! Rows for the `chat` database, in the layout schema.sql describes
//!
//! Hashes follow the proxy's chathistory package: content is JSON with sorted keys and no HTML
//! escaping, a content hash is the first 16 bytes of its SHA-256 in hex, and a node hashes its
//! parent node and its message. serde_json escapes a few control characters differently from Go's
//! encoder, so the same message could hash differently in the two; that costs nothing here,
//! since Antigravity's messages never pass through the proxy

use std::collections::HashSet;

use chrono::DateTime;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::calls::{Call, Content};
use super::conversation::Conversation;

/// The wire format the rows' messages and responses use
const FORMAT: &str = "gemini";

/// The source, provider and session prefix of imported rows
pub const SOURCE: &str = "antigravity";

/// JSON with sorted keys and no HTML escaping, which serde_json's default map gives
pub fn canonical(value: &Value) -> String {
    serde_json::to_string(value).expect("a JSON value always serializes")
}

pub fn content_hash(content: &str) -> String {
    hex::encode(&Sha256::digest(content.as_bytes())[..16])
}

pub fn node_hash(parent: &str, message_hash: &str) -> String {
    content_hash(&format!("{parent}:{message_hash}"))
}

/// ClickHouse's DateTime64(3) text form, in UTC
fn clickhouse_time(ms: i64) -> String {
    DateTime::from_timestamp_millis(ms)
        .unwrap_or_default()
        .format("%Y-%m-%d %H:%M:%S%.3f")
        .to_string()
}

#[derive(Serialize)]
struct BlobRow<'a> {
    hash: &'a str,
    kind: &'a str,
    format: &'a str,
    bytes: usize,
    content: &'a str,
    first_seen: &'a str,
}

#[derive(Serialize)]
struct NodeRow<'a> {
    session_id: &'a str,
    depth: u32,
    node: &'a str,
    parent: &'a str,
    role: &'a str,
    message: &'a str,
    request_id: &'a str,
    ts: &'a str,
}

#[derive(Serialize)]
struct RequestRow<'a> {
    ts: &'a str,
    request_id: &'a str,
    source: &'a str,
    session_id: &'a str,
    parent_session_id: &'a str,
    format: &'a str,
    provider: &'a str,
    model: &'a str,
    response_model: &'a str,
    auth_id: &'a str,
    reasoning_effort: &'a str,
    stream: bool,
    failed: bool,
    status_code: u16,
    error: &'a str,
    latency_ms: u32,
    ttft_ms: u32,
    input_tokens: u64,
    cache_read_tokens: u64,
    cache_write_tokens: u64,
    output_tokens: u64,
    reasoning_tokens: u64,
    system: &'a str,
    tools: &'a str,
    leaf: &'a str,
    message_count: u32,
    params: &'a str,
    response: &'a str,
}

/// The conversation a call belongs to
pub struct Session {
    pub id: String,
    pub parent_id: String,
    /// the Google account, named as the proxy names its auth files, or empty when unknown
    pub auth_id: String,
}

impl Session {
    pub fn new(conversation: &Conversation, auth_id: Option<String>) -> Self {
        let session = |id: &str| format!("{SOURCE}:{id}");
        Self {
            id: session(&conversation.id),
            parent_id: conversation
                .parent_id
                .as_deref()
                .map(session)
                .unwrap_or_default(),
            auth_id: auth_id.unwrap_or_default(),
        }
    }
}

/// NDJSON waiting for upload, one buffer per table
#[derive(Default)]
pub struct Tables {
    pub blobs: Vec<u8>,
    pub nodes: Vec<u8>,
    pub requests: Vec<u8>,
}

impl Tables {
    pub fn byte_len(&self) -> usize {
        self.blobs.len() + self.nodes.len() + self.requests.len()
    }

    /// The tables in upload order: a request's blobs and nodes land before it does
    pub fn in_order(&self) -> [(&'static str, &[u8]); 3] {
        [
            ("blobs", &self.blobs),
            ("nodes", &self.nodes),
            ("requests", &self.requests),
        ]
    }
}

fn push_row(buffer: &mut Vec<u8>, row: &impl Serialize) {
    serde_json::to_writer(&mut *buffer, row).expect("rows always serialize");
    buffer.push(b'\n');
}

/// Builds rows, skipping content and nodes it already wrote in this run. Rows repeated across
/// runs are collapsed by the tables' ReplacingMergeTree keys
#[derive(Default)]
pub struct Recorder {
    blobs: HashSet<String>,
    nodes: HashSet<String>,
    pub tables: Tables,
    pub request_count: usize,
    pub node_count: usize,
    pub blob_count: usize,
}

impl Recorder {
    /// Adds one call's rows
    pub fn add(&mut self, session: &Session, call: &Call) {
        let session_id = session.id.as_str();
        let ts = clickhouse_time(call.ts_ms);
        let system = call
            .system
            .as_deref()
            .map(|c| self.blob("system", c, &ts))
            .unwrap_or_default();
        let tools = call
            .tools
            .as_deref()
            .map(|c| self.blob("tools", c, &ts))
            .unwrap_or_default();
        for entry in &call.history {
            self.blob("message", &entry.message, &ts);
            if !self.nodes.insert(format!("{session_id}|{}", entry.node)) {
                continue;
            }

            self.node_count += 1;
            let row = NodeRow {
                session_id,
                depth: entry.depth,
                node: &entry.node,
                parent: &entry.parent,
                role: &entry.role,
                message: &entry.message.hash,
                request_id: &call.request_id,
                ts: &ts,
            };
            push_row(&mut self.tables.nodes, &row);
        }

        let leaf = call
            .history
            .last()
            .map(|entry| entry.node.as_str())
            .unwrap_or_default();
        let row = RequestRow {
            ts: &ts,
            request_id: &call.request_id,
            source: SOURCE,
            session_id,
            parent_session_id: &session.parent_id,
            format: FORMAT,
            provider: SOURCE,
            model: &call.model,
            response_model: &call.response_model,
            auth_id: &session.auth_id,
            reasoning_effort: &call.reasoning_effort,
            // Antigravity calls streamGenerateContent
            stream: true,
            failed: call.failed,
            status_code: call.status_code,
            error: &call.error,
            latency_ms: clamp_u32(call.latency_ms),
            ttft_ms: clamp_u32(call.ttft_ms),
            input_tokens: call.input_tokens,
            cache_read_tokens: call.cache_read_tokens,
            cache_write_tokens: call.cache_write_tokens,
            output_tokens: call.output_tokens,
            reasoning_tokens: call.reasoning_tokens,
            system: &system,
            tools: &tools,
            leaf,
            message_count: call.history.last().map_or(0, |entry| entry.depth),
            params: &call.params,
            response: &call.response,
        };

        push_row(&mut self.tables.requests, &row);
        self.request_count += 1;
    }

    /// Writes a blob unless this run already did, and returns its hash, or an empty string for
    /// empty content, which the proxy stores as no blob
    fn blob(&mut self, kind: &str, content: &Content, ts: &str) -> String {
        if matches!(content.text.as_str(), "" | "null" | "[]" | "{}" | "\"\"") {
            return String::new();
        }

        if self.blobs.insert(content.hash.clone()) {
            self.blob_count += 1;
            let row = BlobRow {
                hash: &content.hash,
                kind,
                format: FORMAT,
                bytes: content.text.len(),
                content: &content.text,
                first_seen: ts,
            };
            push_row(&mut self.tables.blobs, &row);
        }

        content.hash.clone()
    }

    /// Hands over the buffered rows, keeping what was written for deduplication
    pub fn take(&mut self) -> Tables {
        std::mem::take(&mut self.tables)
    }
}

fn clamp_u32(value: i64) -> u32 {
    u32::try_from(value.max(0)).unwrap_or(u32::MAX)
}
