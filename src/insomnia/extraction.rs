use super::evidence::{EvidenceRequest, parse_evidence_requests, resolve_evidence};
use super::ledger;
use super::metadata;
use super::synthesis;
use crate::{Cva, Episode, GeneralEndpoint, GeneralEndpointError, ResolvedTurn};
use serde_json::{Value, json};
use std::fmt;
use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InsomniaCandidate {
    pub key: String,
    pub authority_kind: String,
    pub category: String,
    pub memory_type: String,
    pub title: String,
    pub content: String,
    pub source_node_id: String,
    pub source_quote: String,
    pub authority_source_conversation_id: Option<String>,
    pub authority_source_node_id: Option<String>,
    pub authority_source_quote: Option<String>,
    pub grounding_source_conversation_id: Option<String>,
    pub grounding_source_node_id: Option<String>,
    pub grounding_source_quote: Option<String>,
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

pub(crate) enum InsomniaExtractionStage {
    Complete(InsomniaExtraction),
    Evidence(InsomniaEvidenceRound),
}

pub(crate) struct InsomniaEvidenceRound {
    episode_payload: Value,
    initial_ledger: Value,
    pub(crate) requests: Vec<EvidenceRequest>,
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
    metadata_endpoint: Option<Arc<dyn GeneralEndpoint>>,
}

impl<E: GeneralEndpoint> InsomniaExtractor<E> {
    pub fn new(endpoint: E) -> Self {
        Self {
            endpoint,
            metadata_endpoint: None,
        }
    }

    pub fn with_metadata_endpoint<M>(mut self, endpoint: M) -> Self
    where
        M: GeneralEndpoint + 'static,
    {
        self.metadata_endpoint = Some(Arc::new(endpoint));
        self
    }

    pub fn model(&self) -> &str {
        self.endpoint.model()
    }

    pub fn extract(
        &self,
        episode: &Episode,
        turns: &[ResolvedTurn],
    ) -> Result<InsomniaExtraction, InsomniaExtractionError> {
        match self.start(episode, turns)? {
            InsomniaExtractionStage::Complete(extraction) => Ok(extraction),
            InsomniaExtractionStage::Evidence(_) => Err(InsomniaExtractionError::InvalidOutput(
                "archive evidence was requested but no evidence archive was supplied".into(),
            )),
        }
    }

    pub(crate) fn extract_with_evidence(
        &self,
        cva: &mut Cva,
        episode: &Episode,
        turns: &[ResolvedTurn],
    ) -> Result<InsomniaExtraction, InsomniaExtractionError> {
        match self.start(episode, turns)? {
            InsomniaExtractionStage::Complete(extraction) => Ok(extraction),
            InsomniaExtractionStage::Evidence(round) => {
                let (results, evidence_turns) = resolve_evidence(cva, episode, &round.requests)?;
                self.finish_evidence(episode, turns, round, results, evidence_turns)
            }
        }
    }

    pub(crate) fn start(
        &self,
        episode: &Episode,
        turns: &[ResolvedTurn],
    ) -> Result<InsomniaExtractionStage, InsomniaExtractionError> {
        let episode_payload = encode_episode_value(episode, turns)?;
        let initial_ledger = self.complete_ledger(&episode_payload, turns, &[], true)?;
        let requests = parse_evidence_requests(&initial_ledger)?;
        if requests.is_empty() {
            let entries = ledger::parse(&initial_ledger, turns, &[])?;
            return self
                .synthesize(episode, turns, &episode_payload, entries, Vec::new())
                .map(InsomniaExtractionStage::Complete);
        }
        Ok(InsomniaExtractionStage::Evidence(InsomniaEvidenceRound {
            episode_payload,
            initial_ledger,
            requests,
        }))
    }

    pub(crate) fn finish_evidence(
        &self,
        episode: &Episode,
        turns: &[ResolvedTurn],
        round: InsomniaEvidenceRound,
        results: Vec<InsomniaEvidenceResult>,
        evidence_turns: Vec<InsomniaEvidenceTurn>,
    ) -> Result<InsomniaExtraction, InsomniaExtractionError> {
        let evidence_results: Vec<_> = results.iter().map(evidence_result_json).collect();
        let evidence_payload_turns: Vec<_> =
            evidence_turns.iter().map(evidence_turn_json).collect();
        let payload = json!({
            "authoritative_episode": round.episode_payload,
            "initial_authority_disposition_ledger": round.initial_ledger,
            "read_only_archive_evidence": {
                "results": evidence_results,
                "turns": evidence_payload_turns
            },
            "instruction": "Return the final authority/disposition ledger now. evidence_requests MUST be empty; no second evidence round is allowed. Archive evidence may resolve a referent through grounding_source_node_id or supply earlier assistant-authored content explicitly adopted/retained through authority_source_node_id, but archive evidence cannot supply user authority."
        });
        let final_ledger = self.complete_ledger(&payload, turns, &evidence_turns, false)?;
        if !parse_evidence_requests(&final_ledger)?.is_empty() {
            return Err(InsomniaExtractionError::InvalidOutput(
                "model requested more than one archive-evidence round".into(),
            ));
        }
        let entries = ledger::parse(&final_ledger, turns, &evidence_turns)?;
        self.synthesize(
            episode,
            turns,
            &payload["authoritative_episode"],
            entries,
            evidence_turns,
        )
    }

    fn complete_ledger(
        &self,
        payload: &Value,
        turns: &[ResolvedTurn],
        evidence_turns: &[InsomniaEvidenceTurn],
        allow_evidence_requests: bool,
    ) -> Result<Value, InsomniaExtractionError> {
        let encoded = serde_json::to_string(payload)
            .map_err(|error| InsomniaExtractionError::InvalidOutput(error.to_string()))?;
        let mut result = self.endpoint.complete_json(
            ledger::LEDGER_SYSTEM_PROMPT,
            &encoded,
            "insomnia_authority_disposition_ledger",
            &ledger::schema(turns, evidence_turns, allow_evidence_requests),
        )?;
        let missing = ledger::sanitize_turn_keys(&mut result, turns)?;
        if missing.is_empty() {
            return Ok(result);
        }

        let repair_payload = json!({
            "authoritative_input": payload,
            "partial_ledger": result,
            "missing_user_turn_ids": missing,
            "instruction": "Repair only the missing user-turn ledger entries listed in missing_user_turn_ids. Return exactly those turn keys and no others. Preserve the original authority rules. Do not revise clauses already present in partial_ledger."
        });
        let repair_payload = serde_json::to_string(&repair_payload)
            .map_err(|error| InsomniaExtractionError::InvalidOutput(error.to_string()))?;
        let repair_prompt = format!(
            "{}\n\nREPAIR MODE: The previous structured result omitted required user-turn keys. Return clauses only for the explicitly listed missing user turns. Do not repeat or modify already-present turns.",
            ledger::LEDGER_SYSTEM_PROMPT
        );
        let repair = self.endpoint.complete_json(
            &repair_prompt,
            &repair_payload,
            "insomnia_authority_disposition_ledger_repair",
            &ledger::repair_schema(turns, evidence_turns, &missing, allow_evidence_requests),
        )?;
        ledger::merge_repair(&mut result, repair, &missing, allow_evidence_requests)?;
        Ok(result)
    }

    fn synthesize(
        &self,
        episode: &Episode,
        turns: &[ResolvedTurn],
        episode_payload: &Value,
        entries: Vec<ledger::LedgerEntry>,
        evidence_turns: Vec<InsomniaEvidenceTurn>,
    ) -> Result<InsomniaExtraction, InsomniaExtractionError> {
        let mut groups = synthesis::build_groups(&entries)?;
        if groups.is_empty() {
            return Ok(InsomniaExtraction {
                model: self.endpoint.model().to_owned(),
                candidates: Vec::new(),
                rejected: Vec::new(),
                evidence_turns,
            });
        }
        if let Some(metadata_endpoint) = &self.metadata_endpoint {
            metadata::classify(
                metadata_endpoint.as_ref(),
                episode_payload,
                turns,
                &evidence_turns,
                &mut groups,
            )?;
        }
        let payload = synthesis::payload(episode_payload, &groups);
        let payload = serde_json::to_string(&payload)
            .map_err(|error| InsomniaExtractionError::InvalidOutput(error.to_string()))?;
        let wording = self.endpoint.complete_json(
            synthesis::SYNTHESIS_SYSTEM_PROMPT,
            &payload,
            "insomnia_memory_wording",
            &synthesis::schema(&groups),
        )?;
        let (candidates, rejected) =
            synthesis::materialize(episode, turns, &evidence_turns, &groups, &wording)?;
        Ok(InsomniaExtraction {
            model: self.endpoint.model().to_owned(),
            candidates,
            rejected,
            evidence_turns,
        })
    }
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
        .map(|turn| json!({"conversation_id": turn.conversation_id, "node_id": turn.node_id}))
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
