use crate::{
    GeneralEndpoint, GeneralEndpointError, ModelProvider, ModelRequestAuth, ModelSwitchboard,
};
use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::{Value, json};
use std::thread::sleep;
use std::time::Duration;

const REQUEST_TIMEOUT_SECS: u64 = 180;
const MAX_REQUEST_ATTEMPTS: usize = 5;

#[derive(Clone)]
pub struct OpenAiReadyGeneralEndpoint {
    client: Client,
    url: String,
    model: String,
    auth: ModelRequestAuth,
    structured_mode: StructuredMode,
}

#[derive(Clone, Copy)]
enum StructuredMode {
    ResponseFormat,
    ForcedTool,
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
        Self::from_route(route, auth, StructuredMode::ResponseFormat)
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
        Self::from_route(route, auth, StructuredMode::ResponseFormat)
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
        Self::from_route(route, auth, StructuredMode::ForcedTool)
    }

    fn from_route(
        route: &crate::GeneralModelEndpoint,
        auth: ModelRequestAuth,
        structured_mode: StructuredMode,
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
            structured_mode,
        })
    }

    fn send_with_retry(&self, payload: &[u8]) -> Result<Vec<u8>, GeneralEndpointError> {
        for attempt in 0..MAX_REQUEST_ATTEMPTS {
            let response = self
                .client
                .post(&self.url)
                .header(AUTHORIZATION, self.auth.authorization_header())
                .header(CONTENT_TYPE, "application/json")
                .body(payload.to_vec())
                .send();
            let response = match response {
                Ok(response) => response,
                Err(error) if attempt + 1 < MAX_REQUEST_ATTEMPTS && retryable_error(&error) => {
                    sleep(retry_delay(attempt));
                    continue;
                }
                Err(error) => return Err(GeneralEndpointError::Failure(error.to_string())),
            };
            let status = response.status();
            let bytes = response
                .bytes()
                .map_err(|error| GeneralEndpointError::Failure(error.to_string()))?
                .to_vec();
            if status.is_success() {
                return Ok(bytes);
            }
            if attempt + 1 < MAX_REQUEST_ATTEMPTS && retryable_status(status.as_u16()) {
                sleep(retry_delay(attempt));
                continue;
            }
            return Err(GeneralEndpointError::Failure(format!(
                "HTTP {status}: {}",
                truncate(&String::from_utf8_lossy(&bytes), 1024)
            )));
        }
        Err(GeneralEndpointError::Failure(
            "request retries exhausted".into(),
        ))
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
        let body = match self.structured_mode {
            StructuredMode::ResponseFormat => json!({
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
            }),
            StructuredMode::ForcedTool => json!({
                "model": self.model,
                "temperature": 0,
                "messages": [
                    {"role": "system", "content": system_prompt},
                    {"role": "user", "content": user_payload}
                ],
                "tools": [{
                    "type": "function",
                    "function": {
                        "name": schema_name,
                        "description": "Return the required structured result.",
                        "parameters": schema,
                        "strict": true
                    }
                }],
                "tool_choice": {
                    "type": "function",
                    "function": {"name": schema_name}
                },
                "parallel_tool_calls": false
            }),
        };
        let payload = serde_json::to_vec(&body)
            .map_err(|error| GeneralEndpointError::Failure(error.to_string()))?;
        let bytes = self.send_with_retry(&payload)?;
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|error| GeneralEndpointError::Failure(error.to_string()))?;
        match self.structured_mode {
            StructuredMode::ResponseFormat => {
                let content = value
                    .pointer("/choices/0/message/content")
                    .and_then(Value::as_str)
                    .ok_or(GeneralEndpointError::InvalidResponse(
                        "missing choices[0].message.content",
                    ))?;
                parse_structured_content(content)
            }
            StructuredMode::ForcedTool => parse_forced_tool_result(&value, schema_name),
        }
    }
}

pub(crate) fn parse_forced_tool_result(
    response: &Value,
    expected_name: &str,
) -> Result<Value, GeneralEndpointError> {
    let calls = response
        .pointer("/choices/0/message/tool_calls")
        .and_then(Value::as_array)
        .ok_or(GeneralEndpointError::InvalidResponse(
            "missing choices[0].message.tool_calls",
        ))?;
    if calls.len() != 1 {
        return Err(GeneralEndpointError::InvalidResponse(
            "forced structured response must contain exactly one tool call",
        ));
    }
    let function = calls[0]
        .get("function")
        .ok_or(GeneralEndpointError::InvalidResponse(
            "tool call is missing function",
        ))?;
    let name = function.get("name").and_then(Value::as_str).ok_or(
        GeneralEndpointError::InvalidResponse("tool call function is missing name"),
    )?;
    if name != expected_name {
        return Err(GeneralEndpointError::InvalidResponse(
            "tool call function name does not match requested schema",
        ));
    }
    let arguments = function.get("arguments").and_then(Value::as_str).ok_or(
        GeneralEndpointError::InvalidResponse("tool call function is missing string arguments"),
    )?;
    serde_json::from_str(arguments).map_err(|error| {
        GeneralEndpointError::Failure(format!(
            "invalid tool-call arguments JSON: {error}; preview={:?}",
            truncate(arguments, 512)
        ))
    })
}

pub(crate) fn parse_structured_content(content: &str) -> Result<Value, GeneralEndpointError> {
    let trimmed = content.trim();
    let candidate = if trimmed.starts_with("```") {
        let first_newline = trimmed.find('\n');
        let closing = trimmed.rfind("```");
        match (first_newline, closing) {
            (Some(first), Some(last)) if last > first => trimmed[first + 1..last].trim(),
            _ => trimmed,
        }
    } else {
        trimmed
    };
    serde_json::from_str(candidate).map_err(|error| {
        GeneralEndpointError::Failure(format!(
            "invalid structured JSON: {error}; preview={:?}",
            truncate(content, 512)
        ))
    })
}

fn retryable_error(error: &reqwest::Error) -> bool {
    error.is_connect() || error.is_timeout()
}

fn retryable_status(status: u16) -> bool {
    matches!(status, 408 | 409 | 429 | 500 | 502 | 503 | 504)
}

fn retry_delay(attempt: usize) -> Duration {
    Duration::from_secs(2_u64.saturating_pow(attempt as u32).min(30))
}

fn truncate(value: &str, max: usize) -> &str {
    value.get(..value.len().min(max)).unwrap_or(value)
}
