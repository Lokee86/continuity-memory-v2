use crate::GeneralEndpointError;
use std::fmt;

#[derive(Debug)]
pub enum DreamClassificationError {
    Endpoint(GeneralEndpointError),
    InvalidPair,
    InvalidOutput(String),
}

impl fmt::Display for DreamClassificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Endpoint(error) => write!(f, "{error}"),
            Self::InvalidPair => write!(f, "Dream classification requires two distinct Memories"),
            Self::InvalidOutput(message) => write!(f, "invalid Dream classification: {message}"),
        }
    }
}

impl std::error::Error for DreamClassificationError {}

impl From<GeneralEndpointError> for DreamClassificationError {
    fn from(value: GeneralEndpointError) -> Self {
        Self::Endpoint(value)
    }
}
