use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct SessionUpdate<'a> {
    #[serde(rename = "type")]
    pub kind: ClientEventKind,
    pub session: SessionConfig<'a>,
}

#[derive(Debug, Serialize)]
pub struct SessionConfig<'a> {
    #[serde(rename = "type")]
    pub kind: SessionKind,
    pub model: &'a str,
    pub instructions: &'a str,
    pub output_modalities: [Modality; 1],
    pub reasoning: ReasoningConfig,
    pub audio: AudioConfig<'a>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReasoningConfig {
    pub effort: ReasoningEffort,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    Low,
}

#[derive(Debug, Serialize)]
pub struct AudioConfig<'a> {
    pub input: AudioInputConfig,
    pub output: AudioOutputConfig<'a>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AudioInputConfig {
    pub format: AudioPcmFormat,
    pub turn_detection: TurnDetectionConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TurnDetectionConfig {
    #[serde(rename = "type")]
    pub kind: TurnDetectionKind,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnDetectionKind {
    ServerVad,
}

#[derive(Debug, Serialize)]
pub struct AudioOutputConfig<'a> {
    pub format: AudioPcmFormat,
    pub voice: &'a str,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
pub struct AudioPcmFormat {
    #[serde(rename = "type")]
    pub kind: AudioFormatKind,
    pub rate: u32,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
pub enum ClientEventKind {
    #[serde(rename = "session.update")]
    SessionUpdate,
    #[serde(rename = "conversation.item.create")]
    ConversationItemCreate,
    #[serde(rename = "response.create")]
    ResponseCreate,
    #[serde(rename = "input_audio_buffer.append")]
    InputAudioBufferAppend,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    Realtime,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    Text,
    Audio,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
pub enum AudioFormatKind {
    #[serde(rename = "audio/pcm")]
    AudioPcm,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Message,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentKind {
    InputText,
}

impl<'a> SessionUpdate<'a> {
    pub fn new(instructions: &'a str, model: &'a str, voice: &'a str) -> Self {
        Self {
            kind: ClientEventKind::SessionUpdate,
            session: SessionConfig {
                kind: SessionKind::Realtime,
                model,
                instructions,
                output_modalities: [Modality::Audio],
                reasoning: ReasoningConfig {
                    effort: ReasoningEffort::Low,
                },
                audio: AudioConfig {
                    input: AudioInputConfig {
                        format: AudioPcmFormat {
                            kind: AudioFormatKind::AudioPcm,
                            rate: 24_000,
                        },
                        turn_detection: TurnDetectionConfig {
                            kind: TurnDetectionKind::ServerVad,
                        },
                    },
                    output: AudioOutputConfig {
                        format: AudioPcmFormat {
                            kind: AudioFormatKind::AudioPcm,
                            rate: 24_000,
                        },
                        voice,
                    },
                },
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UserMessage<'a> {
    #[serde(rename = "type")]
    pub kind: ClientEventKind,
    pub item: UserMessageItem<'a>,
}

#[derive(Debug, Serialize)]
pub struct UserMessageItem<'a> {
    #[serde(rename = "type")]
    pub kind: ItemKind,
    pub role: Role,
    pub content: [UserMessageContent<'a>; 1],
}

#[derive(Debug, Serialize)]
pub struct UserMessageContent<'a> {
    #[serde(rename = "type")]
    pub kind: ContentKind,
    pub text: &'a str,
}

impl<'a> UserMessage<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            kind: ClientEventKind::ConversationItemCreate,
            item: UserMessageItem {
                kind: ItemKind::Message,
                role: Role::User,
                content: [UserMessageContent {
                    kind: ContentKind::InputText,
                    text,
                }],
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ResponseCreate {
    #[serde(rename = "type")]
    pub kind: ClientEventKind,
}

impl Default for ResponseCreate {
    fn default() -> Self {
        Self::new()
    }
}

impl ResponseCreate {
    pub fn new() -> Self {
        Self {
            kind: ClientEventKind::ResponseCreate,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AudioAppend<'a> {
    #[serde(rename = "type")]
    pub kind: ClientEventKind,
    pub audio: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct ServerEvent {
    #[serde(rename = "type")]
    pub kind: ServerEventKind,
    #[serde(default)]
    pub delta: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub error: Option<ServerError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ServerEventKind {
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "response.audio.delta")]
    ResponseAudioDelta,
    #[serde(rename = "response.output_audio.delta")]
    ResponseOutputAudioDelta,
    #[serde(rename = "response.text.delta")]
    ResponseTextDelta,
    #[serde(rename = "response.output_text.delta")]
    ResponseOutputTextDelta,
    #[serde(rename = "response.audio_transcript.delta")]
    ResponseAudioTranscriptDelta,
    #[serde(rename = "response.output_audio_transcript.delta")]
    ResponseOutputAudioTranscriptDelta,
    #[serde(rename = "response.done")]
    ResponseDone,
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
pub struct ServerError {
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::{
        AudioFormatKind, AudioInputConfig, ClientEventKind, Modality, ReasoningConfig,
        ReasoningEffort, SessionKind, SessionUpdate, TurnDetectionKind,
    };
    use crate::codex_voice::config::DEFAULT_REALTIME_MODEL;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct OwnedSessionUpdate {
        #[serde(rename = "type")]
        kind: ClientEventKind,
        session: OwnedSessionConfig,
    }

    #[derive(Debug, Deserialize)]
    struct OwnedSessionConfig {
        #[serde(rename = "type")]
        kind: SessionKind,
        model: String,
        output_modalities: [Modality; 1],
        reasoning: ReasoningConfig,
        audio: OwnedAudioConfig,
    }

    #[derive(Debug, Deserialize)]
    struct OwnedAudioConfig {
        input: AudioInputConfig,
        output: OwnedAudioOutputConfig,
    }

    #[derive(Debug, Deserialize)]
    struct OwnedAudioOutputConfig {
        format: super::AudioPcmFormat,
        voice: String,
    }

    #[derive(Debug, Deserialize)]
    struct OwnedAudioAppend {
        #[serde(rename = "type")]
        kind: ClientEventKind,
        audio: String,
    }

    #[test]
    fn session_update_round_trips_ga_realtime_config() {
        let json = serde_json::to_string(&SessionUpdate::new(
            "instructions",
            DEFAULT_REALTIME_MODEL,
            "marin",
        ))
        .unwrap();
        let event = serde_json::from_str::<OwnedSessionUpdate>(&json).unwrap();

        assert_eq!(event.kind, ClientEventKind::SessionUpdate);
        assert_eq!(event.session.kind, SessionKind::Realtime);
        assert_eq!(event.session.model, "gpt-realtime-2");
        assert_eq!(event.session.output_modalities, [Modality::Audio]);
        assert_eq!(event.session.reasoning.effort, ReasoningEffort::Low);
        assert_eq!(
            event.session.audio.input.turn_detection.kind,
            TurnDetectionKind::ServerVad
        );
        assert_eq!(
            event.session.audio.input.format.kind,
            AudioFormatKind::AudioPcm
        );
        assert_eq!(event.session.audio.input.format.rate, 24_000);
        assert_eq!(
            event.session.audio.output.format.kind,
            AudioFormatKind::AudioPcm
        );
        assert_eq!(event.session.audio.output.format.rate, 24_000);
        assert_eq!(event.session.audio.output.voice, "marin");
    }

    #[test]
    fn serializes_audio_append_event_name() {
        let json = serde_json::to_string(&super::AudioAppend {
            kind: ClientEventKind::InputAudioBufferAppend,
            audio: "abc",
        })
        .unwrap();
        let event = serde_json::from_str::<OwnedAudioAppend>(&json).unwrap();

        assert_eq!(event.kind, ClientEventKind::InputAudioBufferAppend);
        assert_eq!(event.audio, "abc");
    }
}
