use reliquary_memory::EchoEventKind;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
pub(crate) struct InputAttachment {
    pub(crate) path: PathBuf,
    pub(crate) filename: Option<String>,
    pub(crate) mime_type: Option<String>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InputEchoKind {
    ReasoningSummary,
    Commentary,
    ReasoningTrace,
    ToolCall,
    ToolResult,
    ActivityStarted,
    ActivityCompleted,
    ActivityFailed,
}

impl From<InputEchoKind> for EchoEventKind {
    fn from(value: InputEchoKind) -> Self {
        match value {
            InputEchoKind::ReasoningSummary => Self::ReasoningSummary,
            InputEchoKind::Commentary => Self::Commentary,
            InputEchoKind::ReasoningTrace => Self::ReasoningTrace,
            InputEchoKind::ToolCall => Self::ToolCall,
            InputEchoKind::ToolResult => Self::ToolResult,
            InputEchoKind::ActivityStarted => Self::ActivityStarted,
            InputEchoKind::ActivityCompleted => Self::ActivityCompleted,
            InputEchoKind::ActivityFailed => Self::ActivityFailed,
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind")]
pub(crate) enum Input {
    #[serde(rename = "node")]
    Node {
        id: String,
        conversation_id: String,
        parent_id: Option<String>,
        role: String,
        timestamp_ns: i64,
        content: String,
        #[serde(default)]
        attachments: Vec<InputAttachment>,
    },
    #[serde(rename = "branch")]
    Branch {
        id: String,
        conversation_id: String,
        leaf_node_id: String,
        canonical: bool,
        #[serde(default)]
        title: Option<String>,
    },
    #[serde(rename = "echo")]
    Echo {
        conversation_id: String,
        message_id: String,
        sequence: u64,
        timestamp_ns: i64,
        #[serde(default)]
        model_round: Option<u32>,
        event_kind: InputEchoKind,
        #[serde(default)]
        correlation_id: Option<String>,
        #[serde(default)]
        name: Option<String>,
        content: String,
    },
}
