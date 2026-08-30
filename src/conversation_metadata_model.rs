#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversationMetadata {
    pub conversation_id: String,
    pub title: Option<String>,
}
