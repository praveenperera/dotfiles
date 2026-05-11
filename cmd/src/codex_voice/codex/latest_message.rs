use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use color_eyre::eyre::{eyre, Result, WrapErr};
use serde_json::Value;

use super::types::{CodexContext, ReadableItem, ReadableKind, ThreadId, TurnId};

pub struct LatestMessageSelector;

impl Default for LatestMessageSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl LatestMessageSelector {
    pub fn new() -> Self {
        Self
    }

    pub fn select(&self, context: &CodexContext, offset: usize) -> Result<ReadableItem> {
        self.list(context)?
            .into_iter()
            .nth(offset)
            .ok_or_else(|| eyre!("Readable Codex message offset not found: {offset}"))
    }

    pub fn list(&self, context: &CodexContext) -> Result<Vec<ReadableItem>> {
        let file = File::open(&context.thread.rollout_path).wrap_err_with(|| {
            format!(
                "Failed to open Codex rollout {}",
                context.thread.rollout_path.display()
            )
        })?;
        let lines = BufReader::new(file)
            .lines()
            .collect::<std::io::Result<Vec<_>>>()
            .wrap_err("Failed to read Codex rollout")?;

        select_from_lines(
            lines.iter().map(String::as_str),
            context.thread.id.clone(),
            context.thread.source,
        )
        .ok_or_else(|| eyre!("No readable assistant plan or message found in Codex rollout"))
    }
}

pub fn select_from_lines<'a>(
    lines: impl IntoIterator<Item = &'a str>,
    thread_id: ThreadId,
    source: super::types::ResolutionSource,
) -> Option<Vec<ReadableItem>> {
    let mut turns = Vec::new();
    let mut current = TurnMessages::new(TurnId("unknown".to_owned()));
    let mut current_turn = TurnId("unknown".to_owned());

    for line in lines {
        let value = serde_json::from_str::<Value>(line).ok()?;
        if value.get("type").and_then(Value::as_str) == Some("turn_context") {
            if let Some(turn_id) = value.pointer("/payload/turn_id").and_then(Value::as_str) {
                if current.has_readable_items() {
                    turns.push(current);
                }
                current_turn = TurnId(turn_id.to_owned());
                current = TurnMessages::new(current_turn.clone());
            }
        }

        if let Some(text) = plan_text(&value) {
            current.plans.push(ReadableItem {
                thread_id: thread_id.clone(),
                turn_id: current_turn.clone(),
                kind: ReadableKind::Plan,
                text,
                source,
            });
            continue;
        }

        if let Some(text) = agent_message_text(&value) {
            current.agent_messages.push(ReadableItem {
                thread_id: thread_id.clone(),
                turn_id: current_turn.clone(),
                kind: ReadableKind::AgentMessage,
                text,
                source,
            });
        }
    }

    if current.has_readable_items() {
        turns.push(current);
    }

    let selected = turns
        .into_iter()
        .rev()
        .filter_map(TurnMessages::selected_item)
        .collect::<Vec<_>>();
    (!selected.is_empty()).then_some(selected)
}

pub fn preview(text: &str) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = compact.chars();
    let preview = chars.by_ref().take(120).collect::<String>();
    if chars.next().is_some() {
        format!("{preview}...")
    } else {
        preview
    }
}

struct TurnMessages {
    turn_id: TurnId,
    plans: Vec<ReadableItem>,
    agent_messages: Vec<ReadableItem>,
}

impl TurnMessages {
    fn new(turn_id: TurnId) -> Self {
        Self {
            turn_id,
            plans: Vec::new(),
            agent_messages: Vec::new(),
        }
    }

    fn has_readable_items(&self) -> bool {
        !self.plans.is_empty() || !self.agent_messages.is_empty()
    }

    fn selected_item(self) -> Option<ReadableItem> {
        let _turn_id = self.turn_id;
        self.plans
            .into_iter()
            .last()
            .or_else(|| self.agent_messages.into_iter().last())
    }
}

fn plan_text(value: &Value) -> Option<String> {
    if value.get("type").and_then(Value::as_str) != Some("response_item") {
        return None;
    }
    let payload = value.get("payload")?;
    let payload_type = payload.get("type").and_then(Value::as_str)?;
    if !matches!(payload_type, "plan" | "plan_update") {
        return None;
    }

    payload
        .get("text")
        .or_else(|| payload.get("message"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| render_plan_steps(payload))
}

fn render_plan_steps(payload: &Value) -> Option<String> {
    let steps = payload.get("plan")?.as_array()?;
    let rendered = steps
        .iter()
        .filter_map(|step| {
            let text = step.get("step").and_then(Value::as_str)?;
            let status = step
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("pending");
            Some(format!("- [{status}] {text}"))
        })
        .collect::<Vec<_>>();
    (!rendered.is_empty()).then(|| rendered.join("\n"))
}

fn agent_message_text(value: &Value) -> Option<String> {
    if value.get("type").and_then(Value::as_str) == Some("event_msg") {
        let payload = value.get("payload")?;
        if payload.get("type").and_then(Value::as_str) != Some("agent_message") {
            return None;
        }
        if payload.get("phase").and_then(Value::as_str) == Some("commentary") {
            return None;
        }
        return payload
            .get("message")
            .and_then(Value::as_str)
            .filter(|message| !message.trim().is_empty())
            .map(str::to_owned);
    }

    if value.get("type").and_then(Value::as_str) != Some("response_item") {
        return None;
    }
    let payload = value.get("payload")?;
    if payload.get("type").and_then(Value::as_str) != Some("message")
        || payload.get("role").and_then(Value::as_str) != Some("assistant")
    {
        return None;
    }
    message_content_text(payload)
}

fn message_content_text(payload: &Value) -> Option<String> {
    let content = payload.get("content")?.as_array()?;
    let text = content
        .iter()
        .filter_map(|item| item.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    (!text.trim().is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::select_from_lines;
    use crate::codex_voice::codex::types::{ReadableKind, ResolutionSource, ThreadId};

    #[test]
    fn prefers_latest_plan_over_later_agent_message() {
        let lines = [
            r#"{"type":"turn_context","payload":{"turn_id":"turn-1"}}"#,
            r#"{"type":"event_msg","payload":{"type":"agent_message","message":"answer","phase":"final_answer"}}"#,
            r#"{"type":"response_item","payload":{"type":"plan_update","plan":[{"status":"in_progress","step":"Inspect files"}]}}"#,
            r#"{"type":"event_msg","payload":{"type":"agent_message","message":"comment","phase":"commentary"}}"#,
        ];
        let item = select_from_lines(
            lines,
            ThreadId("thread-1".to_owned()),
            ResolutionSource::ExplicitThread,
        )
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
        assert_eq!(item.kind, ReadableKind::Plan);
        assert!(item.text.contains("Inspect files"));
    }

    #[test]
    fn reads_final_agent_message() {
        let lines = [
            r#"{"type":"turn_context","payload":{"turn_id":"turn-1"}}"#,
            r#"{"type":"event_msg","payload":{"type":"agent_message","message":"done","phase":"final_answer"}}"#,
        ];
        let item = select_from_lines(
            lines,
            ThreadId("thread-1".to_owned()),
            ResolutionSource::ExplicitThread,
        )
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
        assert_eq!(item.kind, ReadableKind::AgentMessage);
        assert_eq!(item.text, "done");
    }

    #[test]
    fn returns_previous_turns_for_item_picker() {
        let lines = [
            r#"{"type":"turn_context","payload":{"turn_id":"turn-1"}}"#,
            r#"{"type":"event_msg","payload":{"type":"agent_message","message":"older","phase":"final_answer"}}"#,
            r#"{"type":"turn_context","payload":{"turn_id":"turn-2"}}"#,
            r#"{"type":"event_msg","payload":{"type":"agent_message","message":"newer","phase":"final_answer"}}"#,
        ];
        let items = select_from_lines(
            lines,
            ThreadId("thread-1".to_owned()),
            ResolutionSource::ExplicitThread,
        )
        .unwrap();
        assert_eq!(items[0].text, "newer");
        assert_eq!(items[1].text, "older");
    }
}
