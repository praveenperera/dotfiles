use std::time::Duration;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use color_eyre::eyre::{eyre, Result, WrapErr};

use super::{
    audio::{Microphone, Speaker},
    events::{
        AudioAppend, ClientEventKind, ResponseCreate, ServerEvent, ServerEventKind, SessionUpdate,
        UserMessage,
    },
    prompt,
    websocket::RealtimeWebSocket,
};
use crate::codex_voice::{codex::types::ReadableItem, config::RealtimeConfig};

const AUDIO_PUMP_INTERVAL: Duration = Duration::from_millis(120);

pub struct VoiceSession {
    config: RealtimeConfig,
}

impl VoiceSession {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            config: RealtimeConfig::from_env()?,
        })
    }

    pub async fn ask_with_readout(&self, item: &ReadableItem) -> Result<String> {
        self.run(prompt::readout_message(item)).await
    }

    pub async fn prompt_only(&self, thread_id: &str) -> Result<String> {
        self.run(prompt::prompt_only_message(thread_id)).await
    }

    async fn run(&self, initial_message: String) -> Result<String> {
        eprintln!(
            "Starting Realtime voice session. Say you are ready when you want the final prompt."
        );
        let microphone = Microphone::start()?;
        let speaker = Speaker::start()?;
        let mut websocket =
            RealtimeWebSocket::connect(&self.config.api_key, &self.config.model).await?;
        websocket
            .send_json(SessionUpdate::new(
                prompt::SESSION_INSTRUCTIONS,
                &self.config.model,
                &self.config.voice,
            ))
            .await?;
        websocket
            .send_json(UserMessage::new(&initial_message))
            .await?;
        websocket.send_json(ResponseCreate::new()).await?;

        let mut final_prompt = String::new();
        let mut pre_marker_buffer = String::new();
        let mut saw_final_marker = false;
        let mut audio_tick = tokio::time::interval(AUDIO_PUMP_INTERVAL);
        let mut assistant_audio_started = false;
        let mut assistant_response_done = true;

        loop {
            tokio::select! {
                _ = audio_tick.tick() => {
                    let pcm = microphone.take_pcm16();
                    let speaker_has_pending_audio = speaker.has_pending_audio();
                    if !should_send_microphone(
                        assistant_audio_started,
                        assistant_response_done,
                        speaker_has_pending_audio,
                    ) {
                        continue;
                    }
                    assistant_audio_started = false;
                    if !pcm.is_empty() {
                        let bytes = pcm.iter().flat_map(|sample| sample.to_le_bytes()).collect::<Vec<_>>();
                        let audio = STANDARD.encode(bytes);
                        websocket.send_json(AudioAppend { kind: ClientEventKind::InputAudioBufferAppend, audio: &audio }).await?;
                    }
                }
                text = websocket.next_text() => {
                    let Some(text) = text? else {
                        break;
                    };
                    if handle_server_event(
                        &text,
                        &speaker,
                        &mut final_prompt,
                        &mut pre_marker_buffer,
                        &mut saw_final_marker,
                        &mut assistant_audio_started,
                        &mut assistant_response_done,
                    ).await? {
                        break;
                    }
                }
            }
        }

        let prompt = final_prompt.trim();
        if prompt.is_empty() {
            return Err(eyre!(
                "Realtime session ended before producing FINAL_PROMPT. Try again and say you are ready for the final prompt."
            ));
        }
        Ok(prompt.to_owned())
    }
}

async fn handle_server_event(
    text: &str,
    speaker: &Speaker,
    final_prompt: &mut String,
    pre_marker_buffer: &mut String,
    saw_final_marker: &mut bool,
    assistant_audio_started: &mut bool,
    assistant_response_done: &mut bool,
) -> Result<bool> {
    let event =
        serde_json::from_str::<ServerEvent>(text).wrap_err("Failed to parse Realtime event")?;
    if event.kind == ServerEventKind::Error {
        let message = event
            .error
            .map(|error| error.message)
            .unwrap_or_else(|| "unknown Realtime error".to_owned());
        return Err(eyre!("OpenAI Realtime error: {message}"));
    }

    if matches!(
        event.kind,
        ServerEventKind::ResponseOutputAudioDelta | ServerEventKind::ResponseAudioDelta
    ) {
        if let Some(delta) = event.delta.as_deref() {
            *assistant_audio_started = true;
            *assistant_response_done = false;
            let bytes = STANDARD
                .decode(delta.as_bytes())
                .wrap_err("Failed to decode Realtime audio")?;
            speaker.push_pcm16(&bytes).await;
        }
    }

    if matches!(
        event.kind,
        ServerEventKind::ResponseTextDelta
            | ServerEventKind::ResponseOutputTextDelta
            | ServerEventKind::ResponseAudioTranscriptDelta
            | ServerEventKind::ResponseOutputAudioTranscriptDelta
    ) {
        if let Some(delta) = event.delta.or(event.text) {
            capture_final_prompt(&delta, final_prompt, pre_marker_buffer, saw_final_marker);
        }
    }

    if event.kind == ServerEventKind::ResponseDone {
        *assistant_response_done = true;
    }

    Ok(*saw_final_marker && event.kind == ServerEventKind::ResponseDone)
}

fn should_send_microphone(
    assistant_audio_started: bool,
    assistant_response_done: bool,
    speaker_has_pending_audio: bool,
) -> bool {
    !assistant_audio_started || (assistant_response_done && !speaker_has_pending_audio)
}

fn capture_final_prompt(
    delta: &str,
    final_prompt: &mut String,
    pre_marker_buffer: &mut String,
    saw_final_marker: &mut bool,
) {
    const MARKER: &str = "FINAL_PROMPT:";
    if *saw_final_marker {
        final_prompt.push_str(delta);
        return;
    }

    pre_marker_buffer.push_str(delta);
    if let Some((_, prompt)) = pre_marker_buffer.split_once(MARKER) {
        *saw_final_marker = true;
        final_prompt.push_str(prompt);
        pre_marker_buffer.clear();
    } else if pre_marker_buffer.len() > MARKER.len() {
        while pre_marker_buffer.len() > MARKER.len() {
            let first_char_len = pre_marker_buffer
                .chars()
                .next()
                .map(char::len_utf8)
                .unwrap_or_default();
            pre_marker_buffer.replace_range(..first_char_len, "");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{capture_final_prompt, should_send_microphone};

    #[test]
    fn captures_prompt_after_marker_without_network() {
        let mut prompt = String::new();
        let mut buffer = String::new();
        let mut saw_marker = false;
        capture_final_prompt(
            "FINAL_PROMPT: Fix the parser",
            &mut prompt,
            &mut buffer,
            &mut saw_marker,
        );
        capture_final_prompt(" and run clippy", &mut prompt, &mut buffer, &mut saw_marker);
        assert!(saw_marker);
        assert_eq!(prompt, " Fix the parser and run clippy");
    }

    #[test]
    fn captures_prompt_when_marker_is_split_across_deltas() {
        let mut prompt = String::new();
        let mut buffer = String::new();
        let mut saw_marker = false;
        capture_final_prompt("FINAL_", &mut prompt, &mut buffer, &mut saw_marker);
        capture_final_prompt(
            "PROMPT: Keep going",
            &mut prompt,
            &mut buffer,
            &mut saw_marker,
        );
        assert!(saw_marker);
        assert_eq!(prompt, " Keep going");
    }

    #[test]
    fn trims_pre_marker_buffer_on_character_boundaries() {
        let mut prompt = String::new();
        let mut buffer = String::new();
        let mut saw_marker = false;
        capture_final_prompt(
            "Discuss café résumé before ",
            &mut prompt,
            &mut buffer,
            &mut saw_marker,
        );
        capture_final_prompt(
            "FINAL_PROMPT: Keep UTF-8 safe",
            &mut prompt,
            &mut buffer,
            &mut saw_marker,
        );
        assert!(saw_marker);
        assert_eq!(prompt, " Keep UTF-8 safe");
    }

    #[test]
    fn sends_microphone_before_assistant_audio_starts() {
        assert!(should_send_microphone(false, true, false));
    }

    #[test]
    fn gates_microphone_while_assistant_response_is_active() {
        assert!(!should_send_microphone(true, false, false));
    }

    #[test]
    fn gates_microphone_while_assistant_audio_is_queued_after_done() {
        assert!(!should_send_microphone(true, true, true));
    }

    #[test]
    fn resumes_microphone_after_assistant_audio_is_done_and_drained() {
        assert!(should_send_microphone(true, true, false));
    }
}
