use crate::{CommunityError, GeneralEndpointError, MemoryError, MemoryRetrievalError};
use std::fmt;

#[derive(Debug)]
pub enum DreamCommunityNamingError {
    InvalidConfig(&'static str),
    MissingCurrentSnapshot,
    Community(CommunityError),
    Retrieval(MemoryRetrievalError),
    Memory(MemoryError),
    Inference(GeneralEndpointError),
    InvalidOutput(String),
}

impl DreamCommunityNamingError {
    pub fn is_backpressure(&self) -> bool {
        matches!(self, Self::Inference(error) if error.is_backpressure())
    }
}

impl fmt::Display for DreamCommunityNamingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(field) => {
                write!(f, "invalid Dream community naming config: {field}")
            }
            Self::MissingCurrentSnapshot => {
                write!(
                    f,
                    "Dream community naming requires a current community snapshot"
                )
            }
            Self::Community(error) => write!(f, "{error}"),
            Self::Retrieval(error) => write!(f, "{error}"),
            Self::Memory(error) => write!(f, "{error}"),
            Self::Inference(error) => write!(f, "{error}"),
            Self::InvalidOutput(message) => {
                write!(f, "invalid Dream community naming output: {message}")
            }
        }
    }
}

impl std::error::Error for DreamCommunityNamingError {}

impl From<CommunityError> for DreamCommunityNamingError {
    fn from(value: CommunityError) -> Self {
        Self::Community(value)
    }
}

impl From<MemoryRetrievalError> for DreamCommunityNamingError {
    fn from(value: MemoryRetrievalError) -> Self {
        Self::Retrieval(value)
    }
}

impl From<MemoryError> for DreamCommunityNamingError {
    fn from(value: MemoryError) -> Self {
        Self::Memory(value)
    }
}

impl From<GeneralEndpointError> for DreamCommunityNamingError {
    fn from(value: GeneralEndpointError) -> Self {
        Self::Inference(value)
    }
}
