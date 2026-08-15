use crate::{ContainerError, EmbeddingEndpointError};
use std::fmt;

#[derive(Debug)]
pub enum EmbeddingProfileError {
    Container(ContainerError),
    Endpoint(EmbeddingEndpointError),
    CorruptRecord(&'static str),
    MissingFormat,
    ConflictingFormat,
    MissingProfile,
    InvalidProfile(&'static str),
    EndpointMismatch,
    HashCollision,
    SizeOverflow,
}

impl fmt::Display for EmbeddingProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "embedding-profile error: {self:?}")
    }
}

impl std::error::Error for EmbeddingProfileError {}

impl From<ContainerError> for EmbeddingProfileError {
    fn from(value: ContainerError) -> Self {
        Self::Container(value)
    }
}

impl From<EmbeddingEndpointError> for EmbeddingProfileError {
    fn from(value: EmbeddingEndpointError) -> Self {
        Self::Endpoint(value)
    }
}
