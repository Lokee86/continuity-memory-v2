use crate::{MemoryId, SemanticNodeRef};
use std::fmt;

#[derive(Debug)]
pub enum GraphError {
    Container(crate::ContainerError),
    MissingMemory(MemoryId),
    MissingNode(MemoryId),
    UnsupportedSemanticNode(SemanticNodeRef),
    SelfRelation,
    RevisionConflict { expected: u64, actual: u64 },
    NodeIdExhausted,
    GraphVersionExhausted,
    MissingFormat,
    ConflictingFormat,
    CorruptRecord(&'static str),
    InvalidNodeMapping,
    DuplicateRelationChange,
    InvalidGraphVersion,
    UnknownRelationKind(u16),
    Topology(String),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Container(error) => write!(f, "{error}"),
            Self::MissingMemory(id) => write!(f, "graph references missing memory {:02x?}", id.0),
            Self::MissingNode(id) => write!(f, "graph has no node for memory {:02x?}", id.0),
            Self::UnsupportedSemanticNode(node) => write!(
                f,
                "graph semantic node kind {:?} is not yet backed by a semantic owner",
                node.kind
            ),
            Self::SelfRelation => {
                f.write_str("graph relationships cannot target the source memory")
            }
            Self::RevisionConflict { expected, actual } => write!(
                f,
                "graph version conflict: expected {expected}, current version is {actual}"
            ),
            Self::NodeIdExhausted => f.write_str("graph node id space is exhausted"),
            Self::GraphVersionExhausted => f.write_str("graph version space is exhausted"),
            Self::MissingFormat => f.write_str("graph format marker is missing"),
            Self::ConflictingFormat => f.write_str("graph format marker is duplicated"),
            Self::CorruptRecord(label) => write!(f, "corrupt graph {label}"),
            Self::InvalidNodeMapping => f.write_str("graph node mapping is invalid"),
            Self::DuplicateRelationChange => {
                f.write_str("graph relation batch contains the same relationship more than once")
            }
            Self::InvalidGraphVersion => f.write_str("graph version history is invalid"),
            Self::UnknownRelationKind(kind) => write!(f, "unknown graph relation kind {kind}"),
            Self::Topology(message) => write!(f, "graph topology error: {message}"),
        }
    }
}

impl std::error::Error for GraphError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Container(error) => Some(error),
            _ => None,
        }
    }
}

impl From<crate::ContainerError> for GraphError {
    fn from(value: crate::ContainerError) -> Self {
        Self::Container(value)
    }
}
