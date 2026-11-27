//! Turning a conversation's steps into model calls
//!
//! Antigravity stores what happened (steps) and one record per model call (`gen_metadata`), but
//! keeps the prompt it sent only for the newest call. Each call's conversation is rebuilt here
//! as the steps before it, which matches the prompts it kept: every step in order, leaving out
//! error notices the model is not shown. Messages use Gemini's `contents` shape, so rows read
//! like the proxy's `gemini` format ones

use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde_json::{json, Map, Value};

use super::conversation::Conversation;
use super::proto::{
    stop_reason_name, ChatModel, Media, PlannerResponse, Seconds, Step, StepType, ToolCall,
    ToolDefinition, UserInput,
};
use super::rows::{canonical, content_hash, node_hash};

/// A piece of content in its stored form, with its hash
pub struct Content {
    pub text: String,
    pub hash: String,
}

impl Content {
    fn new(value: &Value) -> Self {
        let text = canonical(value);
        let hash = content_hash(&text);
        Self { text, hash }
    }
}

/// One message at its position in a conversation
pub struct Entry {
    pub role: String,
    pub message: Content,
    pub node: String,
    pub parent: String,
    pub depth: u32,
}

/// One model call, ready to become rows
pub struct Call {
    pub request_id: String,
    pub ts_ms: i64,
    pub model: String,
    pub response_model: String,
    pub reasoning_effort: String,
    pub failed: bool,
    pub status_code: u16,
    pub error: String,
    pub latency_ms: i64,
    pub ttft_ms: i64,
    /// prompt tokens including cache reads, as Gemini's promptTokenCount counts them
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub system: Option<Rc<Content>>,
    pub tools: Option<Rc<Content>>,
    /// the request's conversation, oldest first
    pub history: Vec<Rc<Entry>>,
    pub params: String,
    /// empty for a failed call
    pub response: String,
}

/// What the import saw, for the summary it prints
#[derive(Default)]
pub struct Stats {
    pub conversations: usize,
    pub calls: usize,
    pub failed_calls: usize,
    /// failed calls with no step to take a time from, which are skipped
    pub untimed_calls: usize,
    /// calls whose first step came before steps an earlier call already used
    pub out_of_order_calls: usize,
    pub unknown_steps: BTreeMap<i32, usize>,
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} conversations, {} calls ({} failed)",
            self.conversations, self.calls, self.failed_calls
        )?;

        if self.untimed_calls > 0 {
            write!(
                f,
                ", {} failed calls skipped for having no time",
                self.untimed_calls
            )?;
        }

        if self.out_of_order_calls > 0 {
            write!(f, ", {} calls out of step order", self.out_of_order_calls)?;
        }

        if !self.unknown_steps.is_empty() {
            let unknown: Vec<String> = self
                .unknown_steps
                .iter()
                .map(|(kind, count)| format!("{kind}: {count}"))
                .collect();
            write!(
                f,
                ", skipped steps of unknown types {{{}}}",
                unknown.join(", ")
            )?;
        }

        Ok(())
    }
}

/// The model-visible conversation so far
#[derive(Default)]
struct History {
    entries: Vec<Rc<Entry>>,
    /// when the last step added finished, the time of a failed call that follows it
    last_ms: Option<i64>,
}

impl History {
    fn push(&mut self, value: Value) {
        let role = value["role"].as_str().unwrap_or_default().to_owned();
        let message = Content::new(&value);
        let parent = self
            .entries
            .last()
            .map(|entry| entry.node.clone())
            .unwrap_or_default();
        let node = node_hash(&parent, &message.hash);
        let depth = u32::try_from(self.entries.len() + 1).unwrap_or(u32::MAX);
        self.entries.push(Rc::new(Entry {
            role,
            message,
            node,
            parent,
            depth,
        }));
    }

    fn apply(&mut self, step: &Step, stats: &mut Stats) {
        let metadata = step.metadata.as_ref();
        if let Some(time) = metadata.and_then(|m| m.completed_at.or(m.created_at)) {
            self.last_ms = Some(time.millis());
        }

        let Some(kind) = StepType::from_i32(step.r#type) else {
            *stats.unknown_steps.entry(step.r#type).or_default() += 1;
            return;
        };

        match kind {
            StepType::UserInput => {
                if let Some(input) = &step.user_input {
                    self.push(user_message(input));
                }
            }
            StepType::PlannerResponse => {
                if let Some(response) = &step.planner_response {
                    self.push(model_message(response));
                }
            }
            StepType::Generic => {
                let call = metadata.and_then(|m| m.tool_call.as_ref());
                let result = step.generic.as_ref().and_then(|g| g.result.as_ref());
                let output = result.map(|r| r.result.as_str()).unwrap_or_default();
                let media = result.map(|r| r.media.as_slice()).unwrap_or_default();
                self.push(tool_message(call, output, media));
            }
            StepType::SystemMessage => {
                if let Some(system) = &step.system_message {
                    let mut parts = vec![json!({"text": system.message})];
                    parts.extend(system.media.iter().map(omitted_part));
                    self.push(json!({"role": "user", "parts": parts}));
                }
            }
            StepType::Checkpoint => {
                // a checkpoint summarizes the conversation so far, and later calls see the summary
                // instead of the steps it covers, like a Codex compaction
                if let Some(checkpoint) = &step.checkpoint {
                    let summary = if checkpoint.session_summary.is_empty() {
                        &checkpoint.user_intent
                    } else {
                        &checkpoint.session_summary
                    };

                    self.entries.clear();
                    self.push(json!({"role": "user", "parts": [{"text": summary}]}));
                }
            }
            StepType::ErrorMessage => {
                let Some(error) = step.error_message.as_ref().filter(|e| e.should_show_model)
                else {
                    return;
                };

                let text = error
                    .error
                    .as_ref()
                    .map(|d| d.model_error_message.as_str())
                    .unwrap_or_default();
                self.push(json!({"role": "user", "parts": [{"text": text}]}));
            }
        }
    }
}

/// Rebuilds every model call of a conversation
pub fn calls(conversation: &Conversation, stats: &mut Stats) -> Vec<Call> {
    let steps = &conversation.steps;
    let no_chat = ChatModel::default();
    let mut history = History::default();
    // the first step not yet in the history
    let mut next = 0u32;
    // the step after the last one a call produced
    let mut produced = 0u32;
    let mut model = String::new();
    let mut out = Vec::with_capacity(conversation.calls.len());

    // Antigravity keeps the system prompt and tools only on the newest call, clearing them from
    // the one before when it makes another, so every call gets the newest call's. They rarely
    // change within a conversation, but a call made before a change is shown the later ones
    let chats = || {
        conversation
            .calls
            .iter()
            .rev()
            .filter_map(|(_, g)| g.chat_model.as_ref())
    };
    let system = chats()
        .find(|chat| !chat.system_prompt.is_empty())
        .map(|chat| {
            Rc::new(Content::new(
                &json!({"parts": [{"text": chat.system_prompt}]}),
            ))
        });
    let tools = chats()
        .find(|chat| !chat.tools.is_empty())
        .map(|chat| Rc::new(Content::new(&tools_value(&chat.tools))));

    stats.conversations += 1;
    for (index, generation) in &conversation.calls {
        let chat = generation.chat_model.as_ref().unwrap_or(&no_chat);
        if !chat.response_model.is_empty() {
            model.clone_from(&chat.response_model);
        }

        let first = generation.step_indices.iter().copied().min();
        let ts_ms = match first {
            Some(first) => {
                if first < next {
                    stats.out_of_order_calls += 1;
                } else {
                    for step in steps.range(next..first).map(|(_, step)| step) {
                        history.apply(step, stats);
                    }
                }

                next = next.max(first);
                produced = generation
                    .step_indices
                    .iter()
                    .max()
                    .map_or(produced, |last| last + 1);
                steps.get(&first).and_then(created_ms).or(history.last_ms)
            }
            None => {
                // a failed call made no steps; it saw the previous call's response and every
                // step after it up to the error notice its failure added, or the response of the
                // retry that worked
                for step in steps.range(next..produced.max(next)).map(|(_, step)| step) {
                    history.apply(step, stats);
                }

                next = next.max(produced);
                for (&idx, step) in steps.range(next..) {
                    let kind = StepType::from_i32(step.r#type);
                    if matches!(
                        kind,
                        Some(StepType::ErrorMessage | StepType::PlannerResponse)
                    ) {
                        break;
                    }

                    history.apply(step, stats);
                    next = idx + 1;
                }

                // the call's own error notice gives its time, and a retry that fails too starts
                // after it rather than taking the same time and history
                match steps.range(next..).next() {
                    Some((&idx, step))
                        if StepType::from_i32(step.r#type) == Some(StepType::ErrorMessage) =>
                    {
                        history.apply(step, stats);
                        next = idx + 1;
                        created_ms(step).or(history.last_ms)
                    }
                    Some((_, step)) => created_ms(step).or(history.last_ms),
                    None => history.last_ms,
                }
            }
        };

        let failed = !generation.error.is_empty();
        stats.calls += 1;
        stats.failed_calls += usize::from(failed);
        let Some(ts_ms) = ts_ms else {
            stats.untimed_calls += 1;
            continue;
        };

        let planner = generation
            .step_indices
            .iter()
            .filter_map(|idx| steps.get(idx))
            .find_map(|step| step.planner_response.as_ref());

        let usage = chat.usage.clone().unwrap_or_default();
        let ttft_ms = chat.time_to_first_token.as_ref().map_or(0, Seconds::millis);
        let streaming_ms = chat.streaming_duration.as_ref().map_or(0, Seconds::millis);
        let response = match planner {
            Some(planner) if !failed => canonical(&response_value(planner, chat, &usage)),
            _ => String::new(),
        };

        let params = json!({
            "execution_id": generation.execution_id,
            "model_display_name": chat.model_display_name,
        });

        out.push(Call {
            request_id: format!("{}:{index}", conversation.id),
            ts_ms,
            model: model.clone(),
            response_model: chat.response_model.clone(),
            reasoning_effort: effort(&chat.model_display_name),
            failed,
            status_code: if failed {
                http_status(&generation.error)
            } else {
                200
            },
            error: generation.error.clone(),
            latency_ms: ttft_ms + streaming_ms,
            ttft_ms,
            input_tokens: usage.input_tokens + usage.cache_read_tokens,
            cache_read_tokens: usage.cache_read_tokens,
            cache_write_tokens: usage.cache_write_tokens,
            output_tokens: usage.output_tokens,
            reasoning_tokens: usage.thinking_output_tokens,
            system: system.clone(),
            tools: tools.clone(),
            history: history.entries.clone(),
            params: canonical(&params),
            response,
        });
    }

    out
}

fn created_ms(step: &Step) -> Option<i64> {
    step.metadata
        .as_ref()?
        .created_at
        .as_ref()
        .map(Seconds::millis)
}

/// The reasoning effort in a display name such as `Gemini 4 Neon (High)`
fn effort(display_name: &str) -> String {
    let Some(inner) = display_name.trim_end().strip_suffix(')') else {
        return String::new();
    };

    inner
        .rsplit_once('(')
        .map(|(_, effort)| effort.to_lowercase())
        .unwrap_or_default()
}

/// The HTTP status in an error such as `transport error (ResourceExhausted, HTTP 429): ...`
fn http_status(error: &str) -> u16 {
    error
        .match_indices("HTTP ")
        .find_map(|(at, _)| {
            let digits = error.get(at + 5..at + 8)?;
            digits
                .bytes()
                .all(|b| b.is_ascii_digit())
                .then(|| digits.parse().ok())?
        })
        .unwrap_or(0)
}

fn user_message(input: &UserInput) -> Value {
    let text = if input.user_response.is_empty() {
        &input.query
    } else {
        &input.user_response
    };

    let mut parts = vec![json!({"text": text})];
    for image in &input.images {
        parts.push(json!({"inlineData": {"data": image.base64_data, "mimeType": image.mime_type}}));
    }

    // images a person pasted are kept, like the proxy does; other attachments are described only
    for media in &input.media {
        if media.mime_type.starts_with("image/") && !media.inline_data.is_empty() {
            let data = BASE64.encode(&media.inline_data);
            parts.push(json!({"inlineData": {"data": data, "mimeType": media.mime_type}}));
        } else {
            parts.push(omitted_part(media));
        }
    }

    json!({"role": "user", "parts": parts})
}

fn model_message(response: &PlannerResponse) -> Value {
    json!({"role": "model", "parts": model_parts(response)})
}

fn model_parts(response: &PlannerResponse) -> Vec<Value> {
    let mut parts = Vec::new();
    if !response.thinking.is_empty() {
        parts.push(json!({"text": response.thinking, "thought": true}));
    }

    let text = if response.response.is_empty() {
        &response.modified_response
    } else {
        &response.response
    };

    if !text.is_empty() {
        parts.push(json!({"text": text}));
    }

    parts.extend(response.tool_calls.iter().map(function_call));
    parts.extend(response.media.iter().map(omitted_part));
    parts
}

fn function_call(call: &ToolCall) -> Value {
    let args = serde_json::from_str::<Value>(&call.arguments_json).unwrap_or_else(
        |_| json!({"_invalid_json": call.invalid_json_str.clone() + &call.arguments_json}),
    );

    json!({"functionCall": {"id": call.id, "name": call.name, "args": args}})
}

fn tool_message(call: Option<&ToolCall>, output: &str, media: &[Media]) -> Value {
    let id = call.map(|c| c.id.as_str()).unwrap_or_default();
    let name = call.map(|c| c.name.as_str()).unwrap_or_default();
    let mut parts =
        vec![json!({"functionResponse": {"id": id, "name": name, "response": {"output": output}}})];

    // screenshots and other media a tool returns are dropped, as the proxy drops them
    parts.extend(media.iter().map(omitted_part));
    json!({"role": "user", "parts": parts})
}

/// A dropped attachment, in the shape the proxy's chat history uses for a dropped image
fn omitted_part(media: &Media) -> Value {
    json!({"omitted": {"media_type": media.mime_type, "bytes": media.inline_data.len()}})
}

fn tools_value(tools: &[ToolDefinition]) -> Value {
    let declarations: Vec<Value> = tools
        .iter()
        .map(|tool| {
            let parameters = serde_json::from_str::<Value>(&tool.json_schema_string)
                .unwrap_or_else(|_| Value::String(tool.json_schema_string.clone()));
            json!({"name": tool.name, "description": tool.description, "parameters": parameters})
        })
        .collect();

    json!([{"functionDeclarations": declarations}])
}

/// The call's output in the shape of a Gemini generateContent response
fn response_value(
    planner: &PlannerResponse,
    chat: &ChatModel,
    usage: &super::proto::Usage,
) -> Value {
    let mut usage_metadata = Map::new();
    usage_metadata.insert(
        "promptTokenCount".into(),
        json!(usage.input_tokens + usage.cache_read_tokens),
    );
    usage_metadata.insert(
        "cachedContentTokenCount".into(),
        json!(usage.cache_read_tokens),
    );
    usage_metadata.insert(
        "candidatesTokenCount".into(),
        json!(usage
            .output_tokens
            .saturating_sub(usage.thinking_output_tokens)),
    );
    usage_metadata.insert(
        "thoughtsTokenCount".into(),
        json!(usage.thinking_output_tokens),
    );

    json!({
        "candidates": [{
            "content": {"role": "model", "parts": model_parts(planner)},
            "finishReason": stop_reason_name(planner.stop_reason),
        }],
        "modelVersion": chat.response_model,
        "responseId": usage.response_id,
        "usageMetadata": usage_metadata,
    })
}
