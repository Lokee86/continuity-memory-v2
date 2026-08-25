#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ContentId(pub [u8; 32]);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Node {
    pub id: String,
    pub conversation_id: String,
    pub parent_id: Option<String>,
    pub role: String,
    pub timestamp_ns: i64,
    pub content_id: ContentId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Branch {
    pub id: String,
    pub conversation_id: String,
    pub leaf_node_id: String,
    pub canonical: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedTurn {
    pub node_id: String,
    pub role: String,
    pub timestamp_ns: i64,
    pub content: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversationSummary {
    pub conversation_id: String,
    pub leaf_node_ids: Vec<String>,
    pub turn_count: usize,
    pub latest_timestamp_ns: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArchiveStats {
    pub content_objects: usize,
    pub nodes: usize,
    pub branches: usize,
    pub fragments: usize,
    pub episodes: usize,
    pub files: usize,
    pub source_attachments: usize,
    pub file_memory_links: usize,
}
