use super::candidate::{parse_candidates, validate_candidates};
use super::contract::{INSOMNIA_SYSTEM_PROMPT, insomnia_schema};
use crate::{Episode, GeneralEndpoint, GeneralEndpointError, ResolvedTurn};
use serde_json::json;
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
pub struct InsomniaExtraction {
    pub model: String,
    pub candidates: Vec<InsomniaCandidate>,
    pub rejected: Vec<InsomniaRejection>,
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
        if turns.is_empty() {
            return Err(InsomniaExtractionError::InvalidOutput(
                "episode contains no turns".into(),
            ));
        }
        let payload = encode_episode_payload(episode, turns)?;
        let value = self.endpoint.complete_json(
            INSOMNIA_SYSTEM_PROMPT,
            &payload,
            "insomnia_memory_extraction",
            &insomnia_schema(),
        )?;
        let raw = parse_candidates(&value)?;
        let (candidates, rejected) = validate_candidates(episode.id, turns, raw);
        Ok(InsomniaExtraction {
            model: self.endpoint.model().to_owned(),
            candidates,
            rejected,
        })
    }
}

fn encode_episode_payload(
    episode: &Episode,
    turns: &[ResolvedTurn],
) -> Result<String, InsomniaExtractionError> {
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
    serde_json::to_string(&json!({
        "episode_id": super::candidate::hex(&episode.id.0),
        "conversation_id": episode.conversation_id,
        "start_node_id": episode.start_node_id,
        "end_node_id": episode.end_node_id,
        "turns": turns,
    }))
    .map_err(|error| InsomniaExtractionError::InvalidOutput(error.to_string()))
}
