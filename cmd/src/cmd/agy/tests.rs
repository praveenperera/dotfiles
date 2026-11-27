//! Builds small conversation databases the way agy lays them out, and checks the rows they become

use super::calls::{self, Stats};
use super::conversation::{login_account, Conversation};
use super::proto::{
    ChatModel, Checkpoint, ErrorDetails, ErrorMessage, GeneratorMetadata, Generic, GenericResult,
    PlannerResponse, Seconds, Step, StepMetadata, ToolCall, ToolDefinition, TrajectoryMetadata,
    Usage, UserInput,
};
use super::rows::{Recorder, Session};
use prost::Message;
use rusqlite::Connection;
use serde_json::Value;

const USER_INPUT: i32 = 14;
const PLANNER_RESPONSE: i32 = 15;
const ERROR_MESSAGE: i32 = 17;
const CHECKPOINT: i32 = 23;
const GENERIC: i32 = 132;

fn at(seconds: i64) -> Option<Seconds> {
    Some(Seconds {
        seconds: 1_795_700_000 + seconds,
        nanos: 0,
    })
}

fn step(kind: i32, created: i64) -> Step {
    Step {
        r#type: kind,
        metadata: Some(StepMetadata {
            created_at: at(created),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn read_file_call() -> ToolCall {
    ToolCall {
        id: "c1".into(),
        name: "read_file".into(),
        arguments_json: r#"{"Path":"/x"}"#.into(),
        ..Default::default()
    }
}

/// user input, a tool call and its result, a hidden error, a checkpoint, then the final answer;
/// the calls are one that made the tool call, one that hit a quota, and the one after the
/// checkpoint, which is the only one Antigravity kept the system prompt and tools on
fn write_conversation(path: &std::path::Path) {
    let db = Connection::open(path).unwrap();
    db.execute_batch(
        "CREATE TABLE steps (idx integer PRIMARY KEY, step_type integer, metadata blob, step_payload blob);
         CREATE TABLE gen_metadata (idx integer PRIMARY KEY, data blob);
         CREATE TABLE trajectory_metadata_blob (id text PRIMARY KEY, data blob);",
    )
    .unwrap();

    let parent = TrajectoryMetadata {
        parent_conversation_id: "parent-1".into(),
    };
    db.execute(
        "INSERT INTO trajectory_metadata_blob VALUES ('main', ?1)",
        [parent.encode_to_vec()],
    )
    .unwrap();

    let mut user = step(USER_INPUT, 0);
    user.user_input = Some(UserInput {
        user_response: "read /x".into(),
        ..Default::default()
    });

    let mut tool_use = step(PLANNER_RESPONSE, 10);
    tool_use.planner_response = Some(PlannerResponse {
        thinking: "need the file".into(),
        tool_calls: vec![read_file_call()],
        stop_reason: 2,
        ..Default::default()
    });

    let mut tool_result = step(GENERIC, 20);
    tool_result.metadata.as_mut().unwrap().tool_call = Some(read_file_call());
    tool_result.generic = Some(Generic {
        result: Some(GenericResult {
            result: "contents of x".into(),
            ..Default::default()
        }),
    });

    let mut quota = step(ERROR_MESSAGE, 30);
    quota.error_message = Some(ErrorMessage {
        error: Some(ErrorDetails {
            model_error_message: "quota".into(),
            ..Default::default()
        }),
        should_show_model: false,
    });

    let mut checkpoint = step(CHECKPOINT, 40);
    checkpoint.checkpoint = Some(Checkpoint {
        session_summary: "read x for the user".into(),
        ..Default::default()
    });

    let mut answer = step(PLANNER_RESPONSE, 50);
    answer.planner_response = Some(PlannerResponse {
        response: "x holds contents".into(),
        ..Default::default()
    });

    for (idx, step) in [user, tool_use, tool_result, quota, checkpoint, answer]
        .iter()
        .enumerate()
    {
        db.execute(
            "INSERT INTO steps (idx, step_type, step_payload) VALUES (?1, ?2, ?3)",
            rusqlite::params![idx as i64, step.r#type, step.encode_to_vec()],
        )
        .unwrap();
    }

    let usage = Usage {
        input_tokens: 10,
        output_tokens: 5,
        cache_read_tokens: 90,
        thinking_output_tokens: 2,
        response_id: "r1".into(),
        ..Default::default()
    };

    let first = GeneratorMetadata {
        chat_model: Some(ChatModel {
            usage: Some(usage.clone()),
            time_to_first_token: Some(Seconds {
                seconds: 1,
                nanos: 0,
            }),
            streaming_duration: Some(Seconds {
                seconds: 2,
                nanos: 500_000_000,
            }),
            response_model: "gemini-4-neon".into(),
            model_display_name: "Gemini 4 Neon (High)".into(),
            ..Default::default()
        }),
        step_indices: vec![1],
        ..Default::default()
    };

    let failed = GeneratorMetadata {
        chat_model: Some(ChatModel {
            model_display_name: "Gemini 4 Neon (High)".into(),
            ..Default::default()
        }),
        error: "transport error (ResourceExhausted, HTTP 429): Individual quota reached".into(),
        ..Default::default()
    };

    let last = GeneratorMetadata {
        chat_model: Some(ChatModel {
            system_prompt: "You are Antigravity".into(),
            usage: Some(usage),
            tools: vec![ToolDefinition {
                name: "read_file".into(),
                description: "Read a file".into(),
                json_schema_string: r#"{"type":"object"}"#.into(),
            }],
            response_model: "gemini-4-neon".into(),
            model_display_name: "Gemini 4 Neon (Medium)".into(),
            ..Default::default()
        }),
        step_indices: vec![5],
        ..Default::default()
    };

    for (idx, call) in [first, failed, last].iter().enumerate() {
        db.execute(
            "INSERT INTO gen_metadata VALUES (?1, ?2)",
            rusqlite::params![idx as i64, call.encode_to_vec()],
        )
        .unwrap();
    }
}

fn ndjson(bytes: &[u8]) -> Vec<Value> {
    std::str::from_utf8(bytes)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn rebuilds_calls_from_steps() {
    // T3 keeps the Antigravity login beside the conversations directory
    let app = tempfile::tempdir().unwrap();
    let dir = app.path().join("conversations");
    std::fs::create_dir(&dir).unwrap();
    let login = r#"{"auth_method":"oauth","id_token_email":"me@example.com","token":{}}"#;
    std::fs::write(app.path().join("antigravity-oauth-token"), login).unwrap();
    let path = dir.join("cascade-1.db");
    write_conversation(&path);

    let conversation = Conversation::load(&path).unwrap();
    assert_eq!(conversation.parent_id.as_deref(), Some("parent-1"));

    let mut stats = Stats::default();
    let mut recorder = Recorder::default();
    let session = Session::new(&conversation, login_account(&dir));
    assert_eq!(session.parent_id, "antigravity:parent-1");
    assert_eq!(session.auth_id, "gemini-8c2a47d3-me@example.com.json");
    for call in calls::calls(&conversation, &mut stats) {
        recorder.add(&session, &call);
    }

    let tables = recorder.take();
    let requests = ndjson(&tables.requests);
    let nodes = ndjson(&tables.nodes);
    let blobs = ndjson(&tables.blobs);
    assert_eq!(requests.len(), 3);
    assert_eq!(stats.failed_calls, 1);

    // the first call sees the user's message; tokens count cache reads inside the prompt
    let first = &requests[0];
    assert_eq!(first["request_id"], "cascade-1:0");
    assert_eq!(first["ts"], "2026-11-26 13:33:30.000");
    assert_eq!(first["message_count"], 1);
    assert_eq!(first["input_tokens"], 100);
    assert_eq!(first["cache_read_tokens"], 90);
    assert_eq!(first["reasoning_tokens"], 2);
    assert_eq!(first["reasoning_effort"], "high");
    assert_eq!(
        (first["ttft_ms"].clone(), first["latency_ms"].clone()),
        (1000.into(), 3500.into())
    );
    let response: Value = serde_json::from_str(first["response"].as_str().unwrap()).unwrap();
    let parts = &response["candidates"][0]["content"]["parts"];
    assert_eq!(
        parts[0],
        serde_json::json!({"text": "need the file", "thought": true})
    );
    assert_eq!(parts[1]["functionCall"]["args"]["Path"], "/x");

    // the failed call follows the tool result and takes the time of the error step after it
    let failed = &requests[1];
    assert_eq!(failed["failed"], true);
    assert_eq!(failed["status_code"], 429);
    assert_eq!(failed["message_count"], 3);
    assert_eq!(failed["ts"], "2026-11-26 13:33:50.000");
    assert_eq!(failed["response"], "");

    // after the checkpoint, the history is the summary alone, and the hidden error never shows
    let last = &requests[2];
    assert_eq!(last["message_count"], 1);
    assert_eq!(last["reasoning_effort"], "medium");
    let summary = nodes
        .iter()
        .find(|node| node["node"] == last["leaf"])
        .unwrap();
    assert_eq!(summary["parent"], "");
    let content = blobs
        .iter()
        .find(|blob| blob["hash"] == summary["message"])
        .unwrap();
    assert!(content["content"]
        .as_str()
        .unwrap()
        .contains("read x for the user"));

    // the newest call's system prompt and tools are given to every call
    assert!(requests
        .iter()
        .all(|request| request["system"] == last["system"] && last["system"] != ""));
    assert!(requests
        .iter()
        .all(|request| request["tools"] == last["tools"] && last["tools"] != ""));

    // user, model, tool result, then the summary that starts over
    let roles: Vec<&str> = nodes
        .iter()
        .map(|node| node["role"].as_str().unwrap())
        .collect();
    assert_eq!(roles, ["user", "model", "user", "user"]);
    let tool = blobs
        .iter()
        .find(|blob| {
            blob["content"]
                .as_str()
                .unwrap()
                .contains("functionResponse")
        })
        .unwrap();
    assert!(tool["content"]
        .as_str()
        .unwrap()
        .contains(r#""output":"contents of x""#));
}

/// A first call that hits a quota still saw the user's message, and a retry that fails too saw
/// the retry's message and takes the time of its own error notice
#[test]
fn failed_calls_see_their_input() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cascade-2.db");
    let db = Connection::open(&path).unwrap();
    db.execute_batch(
        "CREATE TABLE steps (idx integer PRIMARY KEY, step_type integer, metadata blob, step_payload blob);
         CREATE TABLE gen_metadata (idx integer PRIMARY KEY, data blob);
         CREATE TABLE trajectory_metadata_blob (id text PRIMARY KEY, data blob);",
    )
    .unwrap();

    let mut user = step(USER_INPUT, 0);
    user.user_input = Some(UserInput {
        user_response: "hi".into(),
        ..Default::default()
    });
    let quota = step(ERROR_MESSAGE, 5);
    let mut retry = step(USER_INPUT, 60);
    retry.user_input = Some(UserInput {
        user_response: "try again".into(),
        ..Default::default()
    });
    let quota_again = step(ERROR_MESSAGE, 65);
    for (idx, step) in [user, quota, retry, quota_again].iter().enumerate() {
        db.execute(
            "INSERT INTO steps (idx, step_type, step_payload) VALUES (?1, ?2, ?3)",
            rusqlite::params![idx as i64, step.r#type, step.encode_to_vec()],
        )
        .unwrap();
    }

    let failed = GeneratorMetadata {
        error: "transport error (ResourceExhausted, HTTP 429): Individual quota reached".into(),
        ..Default::default()
    };
    for idx in [0i64, 1] {
        db.execute(
            "INSERT INTO gen_metadata VALUES (?1, ?2)",
            rusqlite::params![idx, failed.encode_to_vec()],
        )
        .unwrap();
    }
    drop(db);

    let conversation = Conversation::load(&path).unwrap();
    let calls = calls::calls(&conversation, &mut Stats::default());
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].history.len(), 1);
    assert_eq!(calls[0].history[0].role, "user");
    assert_eq!(calls[0].ts_ms, 1_795_700_005_000);
    assert_eq!(calls[1].history.len(), 2);
    assert!(calls[1].history[1].message.text.contains("try again"));
    assert_eq!(calls[1].ts_ms, 1_795_700_065_000);
}
