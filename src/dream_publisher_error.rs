use crate::GraphError;
use std::fmt;

#[derive(Debug)]
pub enum DreamPublicationError {
    Graph(GraphError),
    InvalidClassification,
    MissingVerification,
    VerificationMismatch,
}

impl fmt::Display for DreamPublicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Graph(error) => write!(f, "{error}"),
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
            _ => None,
        }
    }
}

impl From<GraphError> for DreamPublicationError {
    fn from(value: GraphError) -> Self {
        Self::Graph(value)
    }
}
