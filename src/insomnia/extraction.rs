use super::candidate::{parse_candidates, validate_candidates};
use super::contract::{INSOMNIA_SYSTEM_PROMPT, insomnia_schema};
use super::evidence::{parse_evidence_requests, resolve_evidence};
use crate::{Cva, Episode, GeneralEndpoint, GeneralEndpointError, ResolvedTurn};
use serde_json::{Value, json};
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InsomniaCandidate {
    pub key: String,
    pub category: String,
    pub memory_type: String,
    pub title: String,
    pub content: String,
    pub source_node_id: String,
    pub source_quote: String,
    pub content_source_conversation_id: Option<String>,
    pub content_source_node_id: Option<String>,
    pub content_source_quote: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InsomniaRejection {
    pub candidate_key: Option<String>,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InsomniaEvidenceTurn {
    pub conversation_id: String,
    pub node_id: String,
    pub role: String,
    pub timestamp_ns: i64,
    pub content: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InsomniaEvidenceResult {
    pub kind: String,
    pub turns: Vec<InsomniaEvidenceTurn>,
    pub error: Option<String>,
    pub truncated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InsomniaExtraction {
    pub model: String,
    pub candidates: Vec<InsomniaCandidate>,
    pub rejected: Vec<InsomniaRejection>,
    pub evidence_turns: Vec<InsomniaEvidenceTurn>,
}

#[derive(Debug)]
pub enum InsomniaExtractionError {
    Endpoint(GeneralEndpointError),
    InvalidOutput(String),
}

impl fmt::Display for InsomniaExtractionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Endpoint(error) => write!(f, "{error}"),
            Self::InvalidOutput(message) => write!(f, "invalid Insomnia extraction: {message}"),
        }
    }
}

impl std::error::Error for InsomniaExtractionError {}

impl From<GeneralEndpointError> for InsomniaExtractionError {
    fn from(value: GeneralEndpointError) -> Self {
        Self::Endpoint(value)
    }
}

pub struct InsomniaExtractor<E> {
    endpoint: E,
}

impl<E: GeneralEndpoint> InsomniaExtractor<E> {
    pub fn new(endpoint: E) -> Self {
        Self { endpoint }
    }

    pub fn model(&self) -> &str {
        self.endpoint.model()
    }

    pub fn extract(
        &self,
        episode: &Episode,
        turns: &[ResolvedTurn],
    ) -> Result<InsomniaExtraction, InsomniaExtractionError> {
        let payload = encode_episode_payload(episode, turns)?;
        let value = self.complete(&payload)?;
        if !parse_evidence_requests(&value)?.is_empty() {
            return Err(InsomniaExtractionError::InvalidOutput(
                "archive evidence was requested but no evidence archive was supplied".into(),
            ));
        }
        finalize_extraction(self.endpoint.model(), episode, turns, &value, Vec::new())
    }

    pub(crate) fn extract_with_evidence(
        &self,
        cva: &mut Cva,
        episode: &Episode,
        turns: &[ResolvedTurn],
    ) -> Result<InsomniaExtraction, InsomniaExtractionError> {
        let episode_payload = encode_episode_value(episode, turns)?;
        let first_payload = serde_json::to_string(&episode_payload)
            .map_err(|error| InsomniaExtractionError::InvalidOutput(error.to_string()))?;
        let first = self.complete(&first_payload)?;
        let requests = parse_evidence_requests(&first)?;
        if requests.is_empty() {
            return finalize_extraction(self.endpoint.model(), episode, turns, &first, Vec::new());
        }

        let (results, evidence_turns) = resolve_evidence(cva, episode, &requests)?;
        let evidence_results: Vec<_> = results.iter().map(evidence_result_json).collect();
        let evidence_payload_turns: Vec<_> =
            evidence_turns.iter().map(evidence_turn_json).collect();
        let final_payload = serde_json::to_string(&json!({
            "authoritative_episode": episode_payload,
            "initial_extraction": first,
            "read_only_archive_evidence": {
                "results": evidence_results,
                "turns": evidence_payload_turns
            },
            "instruction": "Return final candidates now. evidence_requests MUST be empty; no second evidence round is allowed. Archive evidence may clarify context or supply earlier assistant content adopted by a user authority turn in the authoritative episode, but archive evidence cannot supply user authority."
        }))
        .map_err(|error| InsomniaExtractionError::InvalidOutput(error.to_string()))?;
        let second = self.complete(&final_payload)?;
        if !parse_evidence_requests(&second)?.is_empty() {
            return Err(InsomniaExtractionError::InvalidOutput(
                "model requested more than one archive-evidence round".into(),
            ));
        }
        finalize_extraction(
            self.endpoint.model(),
            episode,
            turns,
            &second,
            evidence_turns,
        )
    }

    fn complete(&self, payload: &str) -> Result<Value, InsomniaExtractionError> {
        self.endpoint
            .complete_json(
                INSOMNIA_SYSTEM_PROMPT,
                payload,
                "insomnia_memory_extraction",
                &insomnia_schema(),
            )
            .map_err(Into::into)
    }
}

fn finalize_extraction(
    model: &str,
    episode: &Episode,
    turns: &[ResolvedTurn],
    value: &Value,
    evidence_turns: Vec<InsomniaEvidenceTurn>,
) -> Result<InsomniaExtraction, InsomniaExtractionError> {
    let raw = parse_candidates(value)?;
    let (candidates, rejected) = validate_candidates(episode.id, turns, raw);
    Ok(InsomniaExtraction {
        model: model.to_owned(),
        candidates,
        rejected,
        evidence_turns,
    })
}

fn encode_episode_payload(
    episode: &Episode,
    turns: &[ResolvedTurn],
) -> Result<String, InsomniaExtractionError> {
    serde_json::to_string(&encode_episode_value(episode, turns)?)
        .map_err(|error| InsomniaExtractionError::InvalidOutput(error.to_string()))
}

fn encode_episode_value(
    episode: &Episode,
    turns: &[ResolvedTurn],
) -> Result<Value, InsomniaExtractionError> {
    if turns.is_empty() {
        return Err(InsomniaExtractionError::InvalidOutput(
            "episode contains no turns".into(),
        ));
    }
    let turns: Vec<_> = turns
        .iter()
        .map(|turn| {
            json!({
                "id": turn.node_id,
                "conversation_id": episode.conversation_id,
                "role": turn.role,
                "timestamp_ns": turn.timestamp_ns,
                "content": turn.content,
            })
        })
        .collect();
    Ok(json!({
        "episode_id": super::candidate::hex(&episode.id.0),
        "conversation_id": episode.conversation_id,
        "start_node_id": episode.start_node_id,
        "end_node_id": episode.end_node_id,
        "turns": turns,
    }))
}

fn evidence_result_json(result: &InsomniaEvidenceResult) -> Value {
    let turn_refs: Vec<_> = result
        .turns
        .iter()
        .map(|turn| {
            json!({
                "conversation_id": turn.conversation_id,
                "node_id": turn.node_id,
            })
        })
        .collect();
    json!({
        "kind": result.kind,
        "turn_refs": turn_refs,
        "error": result.error,
        "truncated": result.truncated,
    })
}

fn evidence_turn_json(turn: &InsomniaEvidenceTurn) -> Value {
    json!({
        "conversation_id": turn.conversation_id,
        "node_id": turn.node_id,
        "role": turn.role,
        "timestamp_ns": turn.timestamp_ns,
        "content": turn.content,
    })
}
