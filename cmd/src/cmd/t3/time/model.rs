//! What each machine's collector reads and sends back to the machine that
//! builds the report

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Bumped when `Collection` changes shape, so a machine running an older
/// `cmd` is reported as out of date instead of being misread
pub const SCHEMA_VERSION: u32 = 1;

/// Half-open UTC range `[since, until)` that prompts and views must fall in
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Window {
    pub since: DateTime<Utc>,
    pub until: DateTime<Utc>,
}

impl Window {
    pub fn contains(&self, at: DateTime<Utc>) -> bool {
        self.since <= at && at < self.until
    }
}

/// Where a record was read from
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    T3,
    ClaudeCode,
    Codex,
    #[serde(rename = "clickhouse")]
    ClickHouse,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Self::T3 => "T3",
            Self::ClaudeCode => "Claude Code transcripts",
            Self::Codex => "Codex rollouts",
            Self::ClickHouse => "ClickHouse",
        }
    }
}

/// The agent CLI behind a provider session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Agent {
    Claude,
    Codex,
}

/// A Claude Code or Codex session id, the key that T3, the local transcripts,
/// and ClickHouse all record
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SessionId {
    pub agent: Agent,
    pub id: String,
}

impl fmt::Display for SessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let agent = match self.agent {
            Agent::Claude => "claude",
            Agent::Codex => "codex",
        };

        write!(formatter, "{agent}:{}", self.id)
    }
}

/// What drove a provider session, which decides whose prompts in it count
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Driver {
    /// A T3 thread; T3's own message log says which of its prompts were typed
    T3 {
        thread: String,
        created: DateTime<Utc>,
    },
    /// A terminal session someone typed into
    Terminal,
    /// An SDK, `claude -p`, `codex exec`, or subagent run that no one typed into
    Automated,
}

impl Driver {
    /// Higher wins when two sources describe the same session
    pub fn rank(&self) -> u8 {
        match self {
            Self::T3 { .. } => 2,
            Self::Terminal => 1,
            Self::Automated => 0,
        }
    }
}

/// A session a collector found, whether or not it had prompts in the window
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownSession {
    pub session: SessionId,
    pub driver: Driver,
}

/// The conversation a prompt belongs to, which the gap from an agent reply
/// to the next prompt is measured within
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "kebab-case")]
pub enum Conversation {
    /// A T3 thread, which can span several provider sessions
    Thread(String),
    /// A provider session outside T3, or one only ClickHouse knows
    Session(SessionId),
}

/// A message the user typed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prompt {
    pub at: DateTime<Utc>,
    pub source: Source,
    pub conversation: Conversation,
    /// Project root for T3 prompts, or the session's working directory
    pub place: Option<String>,
    /// When the agent last wrote anything before the next prompt in the same
    /// conversation, or `None` when it never replied
    pub reply_done: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ViewKind {
    /// The user opened the thread, from its route or the command palette
    Open,
    /// T3 recorded that the open thread changed while it was on screen
    Visit,
}

/// A T3 `thread.visited` record
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct View {
    pub at: DateTime<Utc>,
    pub thread: String,
    pub place: Option<String>,
    pub kind: ViewKind,
}

/// How reading one source on one machine went
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum SourceOutcome {
    Read {
        prompts: usize,
        views: usize,
        sessions: usize,
    },
    /// The machine has no such source, such as no T3 install
    Absent {
        reason: String,
    },
    Failed {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceReport {
    pub source: Source,
    pub outcome: SourceOutcome,
}

/// Everything one machine's collector read for the window
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collection {
    pub schema: u32,
    pub machine: String,
    pub sources: Vec<SourceReport>,
    pub sessions: Vec<KnownSession>,
    pub prompts: Vec<Prompt>,
    pub views: Vec<View>,
    /// T3 project roots, so working directories elsewhere map onto them
    pub projects: Vec<String>,
}

/// What one source read, before it is folded into a `Collection`
#[derive(Debug, Default)]
pub struct SourceData {
    pub sessions: Vec<KnownSession>,
    pub prompts: Vec<Prompt>,
    pub views: Vec<View>,
    pub projects: Vec<String>,
}

impl Collection {
    pub fn new(machine: String) -> Self {
        Self {
            schema: SCHEMA_VERSION,
            machine,
            ..Self::default()
        }
    }

    /// Fold one source's result in, recording how it went
    pub fn add(&mut self, source: Source, result: eyre::Result<Option<SourceData>>) {
        let outcome = match result {
            Ok(Some(data)) => {
                let outcome = SourceOutcome::Read {
                    prompts: data.prompts.len(),
                    views: data.views.len(),
                    sessions: data.sessions.len(),
                };

                self.sessions.extend(data.sessions);
                self.prompts.extend(data.prompts);
                self.views.extend(data.views);
                self.projects.extend(data.projects);
                outcome
            }
            Ok(None) => SourceOutcome::Absent {
                reason: "not installed".to_owned(),
            },
            Err(error) => SourceOutcome::Failed {
                reason: format!("{error:#}"),
            },
        };

        self.sources.push(SourceReport { source, outcome });
    }
}

/// A record in conversation order: a typed prompt at `at`, or agent output
/// that ended at `at`
#[derive(Debug, Clone, Copy)]
pub struct Step {
    pub at: DateTime<Utc>,
    pub prompt: bool,
}

/// The prompts in the window, each with the time the agent's output last
/// ended before the next prompt in the same conversation
pub fn prompts_with_replies(
    steps: &[Step],
    window: &Window,
    conversation: &Conversation,
    source: Source,
    place: Option<&str>,
) -> Vec<Prompt> {
    let mut prompts: Vec<Prompt> = Vec::new();
    for step in steps {
        if step.prompt {
            prompts.push(Prompt {
                at: step.at,
                source,
                conversation: conversation.clone(),
                place: place.map(str::to_owned),
                reply_done: None,
            });
            continue;
        }

        if let Some(last) = prompts.last_mut() {
            last.reply_done = last.reply_done.max(Some(step.at));
        }
    }

    prompts.retain(|prompt| window.contains(prompt.at));
    prompts
}

/// Text prefixes of messages that land in the user role without anyone typing
/// them: Homebased callbacks, T3 subagent briefs, skill bodies Claude Code
/// expands from a slash command, and the summary that resumes a compacted
/// conversation
const INJECTED_PREFIXES: &[&str] = &[
    "HOMEBASED_",
    "Delegated task node:",
    "Base directory for this skill:",
    "This session is being continued from a previous conversation",
    "# AGENTS.md",
];

/// Whether one text block reads like something a person typed. Injected
/// context starts with a tag, except a slash command, which the user did type
pub fn typed_block(text: &str) -> bool {
    let text = text.trim_start();
    if text.is_empty() {
        return false;
    }

    if text.starts_with('<') {
        return text.starts_with("<command-name>") || text.starts_with("<command-message>");
    }

    !INJECTED_PREFIXES
        .iter()
        .any(|prefix| text.starts_with(prefix))
}

/// Whether a Claude or Codex message body holds text a person typed: a plain
/// string, or text blocks with no tool result among them
pub fn typed_content(content: &Value) -> bool {
    match content {
        Value::String(text) => typed_block(text),
        Value::Array(blocks) => typed_blocks(blocks.iter().map(|block| {
            let kind = block.get("type").and_then(Value::as_str).unwrap_or("");
            let text = block.get("text").and_then(Value::as_str).unwrap_or("");
            (kind, text)
        })),
        _ => false,
    }
}

/// `typed_content` over `(type, text)` pairs, which ClickHouse returns
/// already cut down to a prefix
pub fn typed_blocks<'a>(blocks: impl IntoIterator<Item = (&'a str, &'a str)>) -> bool {
    let mut typed = false;
    for (kind, text) in blocks {
        match kind {
            "tool_result" | "function_call_output" => return false,
            "text" | "input_text" => typed |= typed_block(text),
            _ => {}
        }
    }

    typed
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn minute(minute: u32) -> DateTime<Utc> {
        use chrono::TimeZone as _;
        Utc.with_ymd_and_hms(2026, 11, 2, 12, minute, 0).unwrap()
    }

    #[test]
    fn reply_done_is_the_last_agent_output_before_the_next_prompt() {
        let window = Window {
            since: minute(0),
            until: minute(30),
        };

        let step = |at, prompt| Step {
            at: minute(at),
            prompt,
        };

        let steps = [
            step(1, false),
            step(2, true),
            step(4, false),
            step(6, false),
            step(10, true),
            step(12, true),
            step(41, false),
            step(45, true),
        ];

        let conversation = Conversation::Thread("t".to_owned());
        let prompts = prompts_with_replies(&steps, &window, &conversation, Source::T3, None);
        let got: Vec<_> = prompts.iter().map(|p| (p.at, p.reply_done)).collect();

        // output before the first prompt is ignored, and the reply after the
        // window still counts for the window's last prompt
        assert_eq!(
            got,
            [
                (minute(2), Some(minute(6))),
                (minute(10), None),
                (minute(12), Some(minute(41))),
            ]
        );
    }

    #[test]
    fn typed_content_keeps_user_text_and_drops_injected_messages() {
        let cases = [
            (json!("fix the failing test"), true),
            (json!("<command-name>/ship</command-name>"), true),
            (json!("<system-reminder>context</system-reminder>"), false),
            (json!("HOMEBASED_EVENT {\"event\":\"TASK_DONE\"}"), false),
            (
                json!([
                    {"type": "text", "text": "<system-reminder>context</system-reminder>"},
                    {"type": "text", "text": "rebase on master"}
                ]),
                true,
            ),
            (
                json!([{"type": "tool_result", "content": "ok"}, {"type": "text", "text": "hi"}]),
                false,
            ),
            (
                json!([{"type": "input_text", "text": "# AGENTS.md instructions"}]),
                false,
            ),
            (json!([{"type": "input_text", "text": "add a flag"}]), true),
        ];

        for (content, want) in cases {
            assert_eq!(typed_content(&content), want, "{content}");
        }
    }
}
