use super::contract::MAX_INSOMNIA_CANDIDATES;
use super::extraction::{InsomniaCandidate, InsomniaExtractionError, InsomniaRejection};
use crate::{EpisodeId, ResolvedTurn};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashSet;

#[derive(Clone)]
pub(super) struct RawCandidate {
    authority_kind: String,
    category: String,
    memory_type: String,
    title: String,
    content: String,
    source_node_id: String,
    source_quote: String,
    content_source_conversation_id: String,
    content_source_node_id: String,
    content_source_quote: String,
}

pub(super) fn parse_candidates(
    value: &Value,
) -> Result<Vec<RawCandidate>, InsomniaExtractionError> {
    let candidates = value
        .get("candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| InsomniaExtractionError::InvalidOutput("missing candidates array".into()))?;
    if candidates.len() > MAX_INSOMNIA_CANDIDATES {
        return Err(InsomniaExtractionError::InvalidOutput(format!(
            "{} candidates exceeds maximum {}",
            candidates.len(),
            MAX_INSOMNIA_CANDIDATES
        )));
    }
    candidates.iter().map(parse_candidate).collect()
}

fn parse_candidate(value: &Value) -> Result<RawCandidate, InsomniaExtractionError> {
    Ok(RawCandidate {
        authority_kind: required_string(value, "authority_kind")?,
        category: required_string(value, "category")?,
        memory_type: required_string(value, "type")?,
        title: required_string(value, "title")?,
        content: required_string(value, "content")?,
        source_node_id: required_string(value, "source_node_id")?,
        source_quote: required_string(value, "source_quote")?,
        content_source_conversation_id: required_string(value, "content_source_conversation_id")?,
        content_source_node_id: required_string(value, "content_source_node_id")?,
        content_source_quote: required_string(value, "content_source_quote")?,
    })
}

fn required_string(value: &Value, field: &str) -> Result<String, InsomniaExtractionError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            InsomniaExtractionError::InvalidOutput(format!("missing string field {field}"))
        })
}

pub(super) fn validate_candidates(
    episode_id: EpisodeId,
    turns: &[ResolvedTurn],
    raw: Vec<RawCandidate>,
) -> (Vec<InsomniaCandidate>, Vec<InsomniaRejection>) {
    const AUTHORITY_KINDS: &[&str] = &["direct", "correction", "adoption", "retention"];
    const CATEGORIES: &[&str] = &[
        "fact",
        "preference",
        "decision",
        "instruction",
        "relationship",
        "constraint",
        "correction",
        "commitment",
    ];
    const TYPES: &[&str] = &[
        "identity",
        "education",
        "employment",
        "location",
        "possession",
        "health",
        "finance",
        "schedule",
        "communication",
        "project",
        "process",
        "product",
        "relationship",
        "other",
    ];
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    let mut seen = HashSet::new();
    for raw in raw {
        let authority_kind = canonical_label(&raw.authority_kind);
        let category = canonical_label(&raw.category);
        let memory_type = canonical_label(&raw.memory_type);
        let source_node_id = raw.source_node_id.trim().to_owned();
        let source_quote = raw.source_quote.trim().to_owned();
        let source = turns.iter().find(|turn| turn.node_id == source_node_id);
        let mut reason = None;
        if !AUTHORITY_KINDS.contains(&authority_kind.as_str()) {
            reason = Some("authority kind is not recognized".to_owned());
        } else if !CATEGORIES.contains(&category.as_str()) {
            reason = Some("category is not recognized".to_owned());
        } else if !TYPES.contains(&memory_type.as_str()) {
            reason = Some("type is not recognized".to_owned());
        } else if raw.title.trim().is_empty() || raw.content.trim().is_empty() {
            reason = Some("title and content are required".to_owned());
        } else if source.is_none() {
            reason = Some("source node is outside the authoritative episode".to_owned());
        } else if source.is_some_and(|turn| turn.role != "user") {
            reason = Some("source node is not a user authority turn".to_owned());
        } else if source_quote.is_empty()
            || source.is_some_and(|turn| !turn.content.contains(&source_quote))
        {
            reason = Some("source quote is not verbatim from the authority turn".to_owned());
        }
        let content_conversation = raw.content_source_conversation_id.trim();
        let content_node = raw.content_source_node_id.trim();
        let content_quote = raw.content_source_quote.trim();
        if reason.is_none()
            && ((content_conversation.is_empty() != content_node.is_empty())
                || (content_node.is_empty() != content_quote.is_empty()))
        {
            reason = Some("content-source fields must be all empty or all present".to_owned());
        }
        let key = candidate_key(
            episode_id,
            &source_node_id,
            &source_quote,
            content_conversation,
            content_node,
            content_quote,
        );
        if reason.is_none() && !seen.insert(key.clone()) {
            reason = Some("duplicate candidate authority anchor".to_owned());
        }
        if let Some(reason) = reason {
            rejected.push(InsomniaRejection {
                candidate_key: Some(key),
                reason,
            });
            continue;
        }
        accepted.push(InsomniaCandidate {
            key,
            authority_kind,
            category,
            memory_type,
            title: raw.title.trim().to_owned(),
            content: raw.content.trim().to_owned(),
            source_node_id,
            source_quote,
            content_source_conversation_id: (!content_conversation.is_empty())
                .then(|| content_conversation.to_owned()),
            content_source_node_id: (!content_node.is_empty()).then(|| content_node.to_owned()),
            content_source_quote: (!content_quote.is_empty()).then(|| content_quote.to_owned()),
        });
    }
    (accepted, rejected)
}

fn canonical_label(value: &str) -> String {
    value.trim().to_lowercase().replace('-', "_")
}

fn candidate_key(
    episode_id: EpisodeId,
    source_node_id: &str,
    source_quote: &str,
    content_conversation_id: &str,
    content_node_id: &str,
    content_quote: &str,
) -> String {
    let mut hash = Sha256::new();
    hash.update(b"continuity-insomnia-candidate\0");
    hash.update(episode_id.0);
    for value in [
        source_node_id,
        source_quote,
        content_conversation_id,
        content_node_id,
        content_quote,
    ] {
        hash.update((value.len() as u64).to_le_bytes());
        hash.update(value.as_bytes());
    }
    format!("claim-{}", hex(&hash.finalize()[..16]))
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 0x0f) as usize] as char);
    }
    result
}
