use serde_json::Value;
use std::fmt;
use std::sync::Mutex;

#[derive(Debug)]
pub enum GeneralEndpointError {
    InvalidConfiguration(&'static str),
    Failure(String),
    InvalidResponse(&'static str),
}

impl fmt::Display for GeneralEndpointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration(message) => {
                write!(f, "general endpoint configuration: {message}")
            }
            Self::Failure(message) => write!(f, "general endpoint failure: {message}"),
            Self::InvalidResponse(message) => write!(f, "general endpoint response: {message}"),
        }
    }
}

impl std::error::Error for GeneralEndpointError {}

pub trait GeneralEndpoint: Send + Sync {
    fn model(&self) -> &str;

    fn complete_json(
        &self,
        system_prompt: &str,
        user_payload: &str,
        schema_name: &str,
        schema: &Value,
    ) -> Result<Value, GeneralEndpointError>;
}

pub struct SimulatedGeneralEndpoint {
    model: String,
    responses: Mutex<Vec<Value>>,
}

impl SimulatedGeneralEndpoint {
    pub fn new(model: impl Into<String>, responses: Vec<Value>) -> Self {
        let mut responses = responses;
        responses.reverse();
        Self {
            model: model.into(),
            responses: Mutex::new(responses),
        }
    }
}

impl GeneralEndpoint for SimulatedGeneralEndpoint {
    fn model(&self) -> &str {
        &self.model
    }

    fn complete_json(
        &self,
        _system_prompt: &str,
        _user_payload: &str,
        _schema_name: &str,
        _schema: &Value,
    ) -> Result<Value, GeneralEndpointError> {
        self.responses
            .lock()
            .map_err(|_| GeneralEndpointError::Failure("simulator lock poisoned".into()))?
            .pop()
            .ok_or_else(|| GeneralEndpointError::Failure("simulator has no response".into()))
    }
}
