use crate::{GraphError, MemoryError, MemoryId};
use std::fmt;

#[derive(Debug)]
pub enum DreamPublicationError {
    Graph(GraphError),
    Memory(MemoryError),
    MissingSourceTimestamp(MemoryId),
    InvalidClassification,
    MissingVerification,
    VerificationMismatch,
}

impl fmt::Display for DreamPublicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Graph(error) => write!(f, "{error}"),
            Self::Memory(error) => write!(f, "{error}"),
            Self::MissingSourceTimestamp(id) => write!(
                f,
                "duplicate Memory {:02x?} has no authoritative source timestamp",
                id.0
            ),
            Self::InvalidClassification => {
                f.write_str("Dream classification is not valid for Graph publication")
            }
            Self::MissingVerification => {
                f.write_str("Dream relation requires an accepted verification result")
            }
            Self::VerificationMismatch => {
                f.write_str("Dream verification does not match the classification being published")
            }
        }
    }
}

impl std::error::Error for DreamPublicationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Graph(error) => Some(error),
            Self::Memory(error) => Some(error),
            _ => None,
        }
    }
}

impl From<GraphError> for DreamPublicationError {
    fn from(value: GraphError) -> Self {
        Self::Graph(value)
    }
}

impl From<MemoryError> for DreamPublicationError {
    fn from(value: MemoryError) -> Self {
        Self::Memory(value)
    }
}
