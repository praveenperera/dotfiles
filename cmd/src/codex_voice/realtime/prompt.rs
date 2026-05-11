use crate::codex_voice::codex::types::ReadableItem;

pub const SESSION_INSTRUCTIONS: &str = r#"You are a voice companion for a Codex CLI session.

First, read the provided Codex message aloud in a clear, faithful way. Preserve meaning, ordering, and technical detail. You may lightly adapt bullets or code references for speech, but do not add new technical claims.

After reading, answer the user's spoken questions about the message. Help them decide what they want Codex to do next.

When the user is ready, produce one concise prompt they can paste into Codex. Write it directly to Codex in the user's voice. Include all important decisions from the conversation. Do not include meta commentary.

When you produce the final prompt, begin the text response with FINAL_PROMPT: followed by only the paste-ready prompt."#;

pub fn readout_message(item: &ReadableItem) -> String {
    format!(
        "Read this Codex {kind:?} faithfully, then discuss follow-up questions. When I say I am ready, produce FINAL_PROMPT.\n\n{message}",
        kind = item.kind,
        message = item.text
    )
}

pub fn prompt_only_message(thread_id: &str) -> String {
    format!(
        "We are discussing Codex thread {thread_id}. Ask what I want Codex to do next, then produce FINAL_PROMPT when I am ready."
    )
}
