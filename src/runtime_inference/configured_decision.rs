use crate::{DecisionEndpoint, DecisionEndpointError, ModelRequestAuth, ModelSwitchboard};
use reqwest::blocking::Client;
use serde_json::{Value, json};
use std::time::Duration;

const REQUEST_TIMEOUT_SECS: u64 = 30;

#[derive(Clone)]
pub struct ConfiguredDecisionEndpoint {
    client: Client,
    model: String,
    url: String,
    auth: ModelRequestAuth,
}

impl ConfiguredDecisionEndpoint {
    pub fn from_entity_resolution_decision_switchboard(
        switchboard: &ModelSwitchboard,
    ) -> Result<Self, DecisionEndpointError> {
        let endpoint = switchboard.entity_resolution_decision().ok_or(
            DecisionEndpointError::InvalidConfiguration(
                "Entity resolution decision route is not configured",
            ),
        )?;
        let auth = switchboard.entity_resolution_decision_auth().ok_or(
            DecisionEndpointError::InvalidConfiguration(
                "Entity resolution decision auth is not configured",
            ),
        )?;
        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(|error| DecisionEndpointError::Failure(error.to_string()))?;
        Ok(Self {
            client,
            model: endpoint.model.clone(),
            url: endpoint.url.clone(),
            auth,
        })
    }
}

impl DecisionEndpoint for ConfiguredDecisionEndpoint {
    fn model(&self) -> &str {
        &self.model
    }

    fn evaluate(&self, state: &Value, questions: &Value) -> Result<Value, DecisionEndpointError> {
        let response = self
            .client
            .post(&self.url)
            .header("Authorization", self.auth.authorization_header())
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": self.model,
                "state": state,
                "questions": questions,
            }))
            .send()
            .map_err(|error| DecisionEndpointError::Failure(error.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .map_err(|error| DecisionEndpointError::Failure(error.to_string()))?;
        if !status.is_success() {
            return Err(DecisionEndpointError::Failure(format!(
                "HTTP {status}: {body}"
            )));
        }
        serde_json::from_str(&body).map_err(|error| {
            DecisionEndpointError::Failure(format!("response JSON failed: {error}: {body}"))
        })
    }
}
