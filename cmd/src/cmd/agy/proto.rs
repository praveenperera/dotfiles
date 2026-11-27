//! The parts of agy's protobuf messages `cmd agy import` reads
//!
//! agy publishes no schema. These field numbers come from the descriptors embedded in the
//! `antigravity` binary: it is a Go program, so each `.proto` file's descriptor sits in it whole,
//! field names included. `cmd/scripts/agy-proto.py` prints a message's fields or decodes a stored
//! blob with them, to update this file after an agy update:
//!
//! ```text
//! B=$(ls ~/.t3/tools/antigravity-acp/linux-x64/versions/*/antigravity | head -1)   # agy is ~/.local/bin/agy on the Mac
//! cmd/scripts/agy-proto.py "$B" describe 'cortex_pb\.CortexStepPlannerResponse$'
//! sqlite3 copy.db "SELECT writefile('step.bin', step_payload) FROM steps WHERE idx = 3"
//! cmd/scripts/agy-proto.py "$B" decode .gemini_coder.Step step.bin
//! ```
//!
//! Only the fields used here are declared, and prost skips the rest. A step type the importer
//! does not know is counted in its summary, which is the sign agy has added one

use prost::Message;

/// `google.protobuf.Timestamp` and `google.protobuf.Duration`, which share a wire shape
#[derive(Clone, Copy, PartialEq, Eq, Message)]
pub struct Seconds {
    #[prost(int64, tag = "1")]
    pub seconds: i64,
    #[prost(int32, tag = "2")]
    pub nanos: i32,
}

impl Seconds {
    pub fn millis(&self) -> i64 {
        self.seconds * 1000 + i64::from(self.nanos) / 1_000_000
    }
}

/// `exa.cortex_pb.CortexStepGeneratorMetadata`, one row of `gen_metadata`: a model call
#[derive(Clone, PartialEq, Message)]
pub struct GeneratorMetadata {
    #[prost(message, optional, tag = "1")]
    pub chat_model: Option<ChatModel>,
    /// the steps this call produced, starting with its planner response
    #[prost(uint32, repeated, tag = "2")]
    pub step_indices: Vec<u32>,
    #[prost(string, tag = "4")]
    pub execution_id: String,
    #[prost(string, tag = "5")]
    pub error: String,
}

/// `exa.cortex_pb.ChatModelMetadata`
#[derive(Clone, PartialEq, Message)]
pub struct ChatModel {
    /// kept only on the conversation's newest call
    #[prost(string, tag = "1")]
    pub system_prompt: String,
    #[prost(message, optional, tag = "4")]
    pub usage: Option<Usage>,
    #[prost(message, repeated, tag = "8")]
    pub tools: Vec<ToolDefinition>,
    #[prost(message, optional, tag = "11")]
    pub time_to_first_token: Option<Seconds>,
    #[prost(message, optional, tag = "12")]
    pub streaming_duration: Option<Seconds>,
    #[prost(string, tag = "19")]
    pub response_model: String,
    /// such as `Gemini 4 Neon (High)`; the parenthesized part is the reasoning effort
    #[prost(string, tag = "21")]
    pub model_display_name: String,
}

/// `exa.codeium_common_pb.ModelUsageStats`
#[derive(Clone, PartialEq, Message)]
pub struct Usage {
    /// prompt tokens that were not cache reads
    #[prost(uint64, tag = "2")]
    pub input_tokens: u64,
    /// thinking and response tokens together
    #[prost(uint64, tag = "3")]
    pub output_tokens: u64,
    #[prost(uint64, tag = "4")]
    pub cache_write_tokens: u64,
    #[prost(uint64, tag = "5")]
    pub cache_read_tokens: u64,
    #[prost(uint64, tag = "9")]
    pub thinking_output_tokens: u64,
    #[prost(string, tag = "11")]
    pub response_id: String,
}

/// `exa.chat_pb.ChatToolDefinition`
#[derive(Clone, PartialEq, Message)]
pub struct ToolDefinition {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(string, tag = "2")]
    pub description: String,
    #[prost(string, tag = "3")]
    pub json_schema_string: String,
}

/// `gemini_coder.Step`, one row of `steps`
#[derive(Clone, PartialEq, Message)]
pub struct Step {
    /// `exa.cortex_pb.CortexStepType`, see [`StepType`]
    #[prost(int32, tag = "1")]
    pub r#type: i32,
    #[prost(message, optional, tag = "5")]
    pub metadata: Option<StepMetadata>,
    #[prost(message, optional, tag = "19")]
    pub user_input: Option<UserInput>,
    #[prost(message, optional, tag = "20")]
    pub planner_response: Option<PlannerResponse>,
    #[prost(message, optional, tag = "24")]
    pub error_message: Option<ErrorMessage>,
    #[prost(message, optional, tag = "30")]
    pub checkpoint: Option<Checkpoint>,
    #[prost(message, optional, tag = "114")]
    pub system_message: Option<SystemMessage>,
    #[prost(message, optional, tag = "140")]
    pub generic: Option<Generic>,
}

/// The `exa.cortex_pb.CortexStepType` values this importer turns into messages
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepType {
    UserInput,
    PlannerResponse,
    ErrorMessage,
    Checkpoint,
    SystemMessage,
    /// a tool call and its result
    Generic,
}

impl StepType {
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            14 => Some(Self::UserInput),
            15 => Some(Self::PlannerResponse),
            17 => Some(Self::ErrorMessage),
            23 => Some(Self::Checkpoint),
            101 => Some(Self::SystemMessage),
            132 => Some(Self::Generic),
            _ => None,
        }
    }
}

/// `exa.cortex_pb.CortexStepMetadata`
#[derive(Clone, PartialEq, Message)]
pub struct StepMetadata {
    #[prost(message, optional, tag = "1")]
    pub created_at: Option<Seconds>,
    /// the call a tool step answers
    #[prost(message, optional, tag = "4")]
    pub tool_call: Option<ToolCall>,
    #[prost(message, optional, tag = "8")]
    pub completed_at: Option<Seconds>,
}

/// `exa.codeium_common_pb.ChatToolCall`
#[derive(Clone, PartialEq, Message)]
pub struct ToolCall {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(string, tag = "2")]
    pub name: String,
    #[prost(string, tag = "3")]
    pub arguments_json: String,
    #[prost(string, tag = "4")]
    pub invalid_json_str: String,
}

/// `exa.cortex_pb.CortexStepUserInput`
#[derive(Clone, PartialEq, Message)]
pub struct UserInput {
    #[prost(string, tag = "1")]
    pub query: String,
    #[prost(string, tag = "2")]
    pub user_response: String,
    #[prost(message, repeated, tag = "5")]
    pub images: Vec<ImageData>,
    #[prost(message, repeated, tag = "9")]
    pub media: Vec<Media>,
}

/// `exa.codeium_common_pb.ImageData`
#[derive(Clone, PartialEq, Message)]
pub struct ImageData {
    #[prost(string, tag = "1")]
    pub base64_data: String,
    #[prost(string, tag = "2")]
    pub mime_type: String,
}

/// `exa.codeium_common_pb.Media`
#[derive(Clone, PartialEq, Message)]
pub struct Media {
    #[prost(string, tag = "1")]
    pub mime_type: String,
    #[prost(bytes = "vec", tag = "2")]
    pub inline_data: Vec<u8>,
}

/// `exa.cortex_pb.CortexStepPlannerResponse`, the model's output for one call
#[derive(Clone, PartialEq, Message)]
pub struct PlannerResponse {
    #[prost(string, tag = "1")]
    pub response: String,
    #[prost(string, tag = "3")]
    pub thinking: String,
    #[prost(message, repeated, tag = "7")]
    pub tool_calls: Vec<ToolCall>,
    /// the response after Antigravity's post-processing, used when `response` is empty
    #[prost(string, tag = "8")]
    pub modified_response: String,
    /// `exa.codeium_common_pb.StopReason`
    #[prost(int32, tag = "12")]
    pub stop_reason: i32,
    #[prost(message, repeated, tag = "20")]
    pub media: Vec<Media>,
}

/// `exa.cortex_pb.CortexStepErrorMessage`
#[derive(Clone, PartialEq, Message)]
pub struct ErrorMessage {
    #[prost(message, optional, tag = "3")]
    pub error: Option<ErrorDetails>,
    #[prost(bool, tag = "4")]
    pub should_show_model: bool,
}

/// `exa.cortex_pb.CortexErrorDetails`
#[derive(Clone, PartialEq, Message)]
pub struct ErrorDetails {
    #[prost(string, tag = "1")]
    pub user_error_message: String,
    #[prost(string, tag = "9")]
    pub model_error_message: String,
}

/// `exa.cortex_pb.CortexStepCheckpoint`, a summary that replaces the history before it
#[derive(Clone, PartialEq, Message)]
pub struct Checkpoint {
    #[prost(string, tag = "4")]
    pub user_intent: String,
    #[prost(string, tag = "5")]
    pub session_summary: String,
}

/// `exa.cortex_pb.CortexStepSystemMessage`, such as a finished background task's notice
#[derive(Clone, PartialEq, Message)]
pub struct SystemMessage {
    #[prost(string, tag = "1")]
    pub message: String,
    #[prost(message, repeated, tag = "6")]
    pub media: Vec<Media>,
}

/// `exa.cortex_pb.CortexStepGeneric`
#[derive(Clone, PartialEq, Message)]
pub struct Generic {
    #[prost(message, optional, tag = "2")]
    pub result: Option<GenericResult>,
}

/// `exa.cortex_pb.GenericStepResult`
#[derive(Clone, PartialEq, Message)]
pub struct GenericResult {
    /// the text the model saw, which Antigravity truncates and points at a file when long
    #[prost(string, tag = "1")]
    pub result: String,
    #[prost(message, repeated, tag = "4")]
    pub media: Vec<Media>,
}

/// `exa.cortex_pb.CortexTrajectoryMetadata`, the one row of `trajectory_metadata_blob`
#[derive(Clone, PartialEq, Message)]
pub struct TrajectoryMetadata {
    /// set on a subagent's conversation
    #[prost(string, tag = "5")]
    pub parent_conversation_id: String,
}

/// The name of an `exa.codeium_common_pb.StopReason`, without its prefix
pub fn stop_reason_name(value: i32) -> String {
    let name = match value {
        0 => "UNSPECIFIED",
        1 => "INCOMPLETE",
        2 => "STOP_PATTERN",
        3 => "MAX_TOKENS",
        10 => "FUNCTION_CALL",
        11 => "CONTENT_FILTER",
        13 => "ERROR",
        15 => "OTHER",
        16 => "CLIENT_CANCELED",
        _ => return value.to_string(),
    };

    name.to_owned()
}
