use reliquary_memory::{
    ConfiguredDecisionEndpoint, DecisionEndpoint, ModelSwitchboard, ReliquaryConfig,
};
use serde_json::Value;
use std::path::Path;

#[derive(Clone)]
pub struct JevClient {
    endpoint: ConfiguredDecisionEndpoint,
}

impl JevClient {
    pub fn from_config(path: &Path, model: &str) -> Result<Self, String> {
        let config = ReliquaryConfig::open(path).map_err(|e| e.to_string())?;
        let switchboard =
            ModelSwitchboard::new(config.models, config.credentials).map_err(|e| e.to_string())?;
        let endpoint =
            ConfiguredDecisionEndpoint::from_entity_resolution_decision_switchboard(&switchboard)
                .map_err(|e| e.to_string())?;
        if endpoint.model() != model {
            return Err(format!(
                "configured decision model {} does not match requested {model}",
                endpoint.model()
            ));
        }
        Ok(Self { endpoint })
    }

    pub fn evaluate(&self, state: Value, questions: Value) -> Result<Value, String> {
        self.endpoint
            .evaluate(&state, &questions)
            .map_err(|e| e.to_string())
    }
}

pub fn choice(response: &Value, id: &str) -> Result<(String, Value, Value), String> {
    let answer = response
        .get("answers")
        .and_then(|v| v.get(id))
        .ok_or_else(|| format!("missing answer {id}"))?;
    if answer.get("type").and_then(Value::as_str) != Some("choice") {
        return Err(format!("answer {id} is not choice"));
    }
    let selected = answer
        .get("choice")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("answer {id} missing choice"))?
        .to_owned();
    Ok((
        selected,
        answer.get("probabilities").cloned().unwrap_or(Value::Null),
        answer.get("confidence").cloned().unwrap_or(Value::Null),
    ))
}

pub fn noul(response: &Value, id: &str) -> Result<f64, String> {
    let answer = response
        .get("answers")
        .and_then(|v| v.get(id))
        .ok_or_else(|| format!("missing answer {id}"))?;
    if answer.get("type").and_then(Value::as_str) != Some("noul") {
        return Err(format!("answer {id} is not noul"));
    }
    answer
        .get("noul")
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("answer {id} missing noul probability"))
}
