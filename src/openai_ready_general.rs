use crate::{
    GeneralEndpoint, GeneralEndpointError, ModelProvider, ModelRequestAuth, ModelSwitchboard,
};
use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::{Value, json};
use std::time::Duration;

const REQUEST_TIMEOUT_SECS: u64 = 180;

#[derive(Clone)]
pub struct OpenAiReadyGeneralEndpoint {
    client: Client,
    url: String,
    model: String,
    auth: ModelRequestAuth,
}

impl OpenAiReadyGeneralEndpoint {
    pub fn from_switchboard(switchboard: &ModelSwitchboard) -> Result<Self, GeneralEndpointError> {
        let route = switchboard
            .general()
            .ok_or(GeneralEndpointError::InvalidConfiguration(
                "general route is not configured",
            ))?;
        let auth = switchboard
            .general_auth()
            .ok_or(GeneralEndpointError::InvalidConfiguration(
                "general auth is missing",
            ))?;
        Self::from_route(route, auth)
    }

    pub fn from_insomnia_switchboard(
        switchboard: &ModelSwitchboard,
    ) -> Result<Self, GeneralEndpointError> {
        let route = switchboard
            .insomnia()
            .ok_or(GeneralEndpointError::InvalidConfiguration(
                "insomnia and general routes are not configured",
            ))?;
        let auth =
            switchboard
                .insomnia_auth()
                .ok_or(GeneralEndpointError::InvalidConfiguration(
                    "insomnia auth is missing",
                ))?;
        Self::from_route(route, auth)
    }

    pub fn from_dream_switchboard(
        switchboard: &ModelSwitchboard,
    ) -> Result<Self, GeneralEndpointError> {
        let route = switchboard
            .dream()
            .ok_or(GeneralEndpointError::InvalidConfiguration(
                "dream and general routes are not configured",
            ))?;
        let auth = switchboard
            .dream_auth()
            .ok_or(GeneralEndpointError::InvalidConfiguration(
                "dream auth is missing",
            ))?;
        Self::from_route(route, auth)
    }

    fn from_route(
        route: &crate::GeneralModelEndpoint,
        auth: ModelRequestAuth,
    ) -> Result<Self, GeneralEndpointError> {
        if route.provider != ModelProvider::OpenAiReady {
            return Err(GeneralEndpointError::InvalidConfiguration(
                "general route is not OpenAI-ready",
            ));
        }
        let url = route
            .url
            .clone()
            .ok_or(GeneralEndpointError::InvalidConfiguration(
                "general URL is missing",
            ))?;
        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(|error| GeneralEndpointError::Failure(error.to_string()))?;
        Ok(Self {
            client,
            url,
            model: route.model.clone(),
            auth,
        })
    }
}

impl GeneralEndpoint for OpenAiReadyGeneralEndpoint {
    fn model(&self) -> &str {
        &self.model
    }

    fn complete_json(
        &self,
        system_prompt: &str,
        user_payload: &str,
        schema_name: &str,
        schema: &Value,
    ) -> Result<Value, GeneralEndpointError> {
        let body = json!({
            "model": self.model,
            "temperature": 0,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_payload}
            ],
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": schema_name,
                    "strict": true,
                    "schema": schema
                }
            }
        });
        let payload = serde_json::to_vec(&body)
            .map_err(|error| GeneralEndpointError::Failure(error.to_string()))?;
        let response = self
            .client
            .post(&self.url)
            .header(AUTHORIZATION, self.auth.authorization_header())
            .header(CONTENT_TYPE, "application/json")
            .body(payload)
            .send()
            .map_err(|error| GeneralEndpointError::Failure(error.to_string()))?;
        let status = response.status();
        let bytes = response
            .bytes()
            .map_err(|error| GeneralEndpointError::Failure(error.to_string()))?;
        if !status.is_success() {
            return Err(GeneralEndpointError::Failure(format!(
                "HTTP {status}: {}",
                truncate(&String::from_utf8_lossy(&bytes), 1024)
            )));
        }
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|error| GeneralEndpointError::Failure(error.to_string()))?;
        let content = value
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .ok_or(GeneralEndpointError::InvalidResponse(
                "missing choices[0].message.content",
            ))?;
        serde_json::from_str(content).map_err(|error| {
            GeneralEndpointError::Failure(format!("invalid structured JSON: {error}"))
        })
    }
}

fn truncate(value: &str, max: usize) -> &str {
    value.get(..value.len().min(max)).unwrap_or(value)
}
