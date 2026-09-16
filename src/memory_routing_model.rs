use crate::{MemoryBodyId, MemoryId};

pub const MAX_MEMORY_ENTITY_MENTIONS: usize = 64;
pub const MAX_MEMORY_LEXICAL_TERMS: usize = 64;
pub const MAX_MEMORY_ROUTING_TEXT_BYTES: usize = 512;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MemoryTextField {
    Title,
    Content,
}

impl MemoryTextField {
    pub(crate) fn tag(self) -> u8 {
        match self {
            Self::Title => 1,
            Self::Content => 2,
        }
    }

    pub(crate) fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::Title),
            2 => Some(Self::Content),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryEntityMention {
    pub field: MemoryTextField,
    pub start_byte: u32,
    pub end_byte: u32,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryRoutingMetadata {
    pub memory_id: MemoryId,
    pub body_id: MemoryBodyId,
    pub entity_mentions: Vec<MemoryEntityMention>,
    pub lexical_terms: Vec<String>,
}
