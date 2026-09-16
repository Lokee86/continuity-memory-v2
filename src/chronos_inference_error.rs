use crate::GeneralEndpointError;

#[derive(Debug)]
pub enum TemporalInferenceError {
    Endpoint(GeneralEndpointError),
    InvalidOutput(String),
}

impl std::fmt::Display for TemporalInferenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Endpoint(error) => write!(f, "{error}"),
            Self::InvalidOutput(message) => write!(f, "invalid Chronos inference: {message}"),
        }
    }
}

impl std::error::Error for TemporalInferenceError {}

impl From<GeneralEndpointError> for TemporalInferenceError {
    fn from(value: GeneralEndpointError) -> Self {
        Self::Endpoint(value)
    }
}
