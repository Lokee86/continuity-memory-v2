use crate::InteractionRole;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InteractionStreamStatus {
    Streaming,
    Interrupted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InteractionStreamRecord {
    pub message_id: String,
    pub session_id: String,
    pub parent_message_id: Option<String>,
    pub role: InteractionRole,
    pub timestamp_ns: i64,
    pub content: String,
    pub status: InteractionStreamStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InteractionTurnStatus {
    Complete,
    Streaming,
    Interrupted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedInteractionTurn {
    pub message_id: String,
    pub role: String,
    pub timestamp_ns: i64,
    pub content: String,
    pub status: InteractionTurnStatus,
}
