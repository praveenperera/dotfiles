use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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
    pub audio: AudioConfig<'a>,
}

#[derive(Debug, Serialize)]
pub struct AudioConfig<'a> {
    pub input: AudioInputConfig,
    pub output: AudioOutputConfig<'a>,
}

#[derive(Debug, Serialize)]
pub struct AudioInputConfig {
    pub format: AudioPcmFormat,
    pub turn_detection: Value,
}

#[derive(Debug, Serialize)]
pub struct AudioOutputConfig<'a> {
    pub format: AudioPcmFormat,
    pub voice: &'a str,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct AudioPcmFormat {
    #[serde(rename = "type")]
    pub kind: AudioFormatKind,
    pub rate: u32,
}

#[derive(Debug, Clone, Copy, Serialize)]
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

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    Realtime,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    Text,
    Audio,
}

#[derive(Debug, Clone, Copy, Serialize)]
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
                audio: AudioConfig {
                    input: AudioInputConfig {
                        format: AudioPcmFormat {
                            kind: AudioFormatKind::AudioPcm,
                            rate: 24_000,
                        },
                        turn_detection: json!({ "type": "server_vad" }),
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
    use super::{ClientEventKind, SessionUpdate};

    #[test]
    fn serializes_session_update_with_enum_event_names() {
        let value =
            serde_json::to_value(SessionUpdate::new("instructions", "gpt-realtime", "marin"))
                .unwrap();
        assert_eq!(value["type"], "session.update");
        assert_eq!(value["session"]["type"], "realtime");
        assert_eq!(value["session"]["output_modalities"][0], "audio");
        assert_eq!(
            value["session"]["audio"]["input"]["format"]["type"],
            "audio/pcm"
        );
        assert_eq!(value["session"]["audio"]["input"]["format"]["rate"], 24_000);
    }

    #[test]
    fn serializes_audio_append_event_name() {
        let value = serde_json::to_value(super::AudioAppend {
            kind: ClientEventKind::InputAudioBufferAppend,
            audio: "abc",
        })
        .unwrap();
        assert_eq!(value["type"], "input_audio_buffer.append");
    }
}
