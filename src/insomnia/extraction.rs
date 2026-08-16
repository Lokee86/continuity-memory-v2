use super::candidate::{parse_candidates, validate_candidates};
use super::contract::{
    INSOMNIA_EXTRACTOR_CONTRACT_VERSION, INSOMNIA_SYSTEM_PROMPT, insomnia_schema,
};
use super::evidence::{EvidenceRequest, parse_evidence_requests, resolve_evidence};
use super::ledger_contract::INSOMNIA_LEDGER_CONTRACT_VERSION;
use super::ledger_pipeline;
use super::payload::{encode_episode_value, evidence_result_json, evidence_turn_json};
use crate::{Cva, Episode, GeneralEndpoint, GeneralEndpointError, ResolvedTurn};
use serde_json::{Value, json};
use std::fmt;

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
    pub contract_version: String,
    pub candidates: Vec<InsomniaCandidate>,
    pub rejected: Vec<InsomniaRejection>,
    pub evidence_turns: Vec<InsomniaEvidenceTurn>,
}

pub(crate) enum InsomniaExtractionStage {
    Complete(InsomniaExtraction),
    Evidence(InsomniaEvidenceRound),
}

pub(crate) struct InsomniaEvidenceRound {
    pub(super) mode: InsomniaEvidenceMode,
    pub(super) episode_payload: Value,
    pub(super) initial: Value,
    pub(crate) requests: Vec<EvidenceRequest>,
}

#[derive(Clone, Copy)]
pub(super) enum InsomniaEvidenceMode {
    Direct,
    Ledger,
}

#[derive(Clone, Copy)]
enum InsomniaExtractionMode {
    Direct,
    Ledger,
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
    mode: InsomniaExtractionMode,
}

impl<E: GeneralEndpoint> InsomniaExtractor<E> {
    pub fn new(endpoint: E) -> Self {
        Self {
            endpoint,
            mode: InsomniaExtractionMode::Direct,
        }
    }

    pub fn new_ledger(endpoint: E) -> Self {
        Self {
            endpoint,
            mode: InsomniaExtractionMode::Ledger,
        }
    }

    pub fn model(&self) -> &str {
        self.endpoint.model()
    }

    pub fn contract_version(&self) -> &str {
        match self.mode {
            InsomniaExtractionMode::Direct => INSOMNIA_EXTRACTOR_CONTRACT_VERSION,
            InsomniaExtractionMode::Ledger => INSOMNIA_LEDGER_CONTRACT_VERSION,
        }
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
        if matches!(self.mode, InsomniaExtractionMode::Ledger) {
            return ledger_pipeline::start(&self.endpoint, episode, turns);
        }
        let episode_payload = encode_episode_value(episode, turns)?;
        let payload = serde_json::to_string(&episode_payload)
            .map_err(|error| InsomniaExtractionError::InvalidOutput(error.to_string()))?;
        let initial = self.complete_direct(&payload)?;
        let requests = parse_evidence_requests(&initial)?;
        if requests.is_empty() {
            return finalize_extraction(
                self.endpoint.model(),
                INSOMNIA_EXTRACTOR_CONTRACT_VERSION,
                episode,
                turns,
                &initial,
                Vec::new(),
            )
            .map(InsomniaExtractionStage::Complete);
        }
        Ok(InsomniaExtractionStage::Evidence(InsomniaEvidenceRound {
            mode: InsomniaEvidenceMode::Direct,
            episode_payload,
            initial,
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
        if matches!(round.mode, InsomniaEvidenceMode::Ledger) {
            return ledger_pipeline::finish_evidence(
                &self.endpoint,
                episode,
                turns,
                round,
                results,
                evidence_turns,
            );
        }
        let evidence_results: Vec<_> = results.iter().map(evidence_result_json).collect();
        let evidence_payload_turns: Vec<_> =
            evidence_turns.iter().map(evidence_turn_json).collect();
        let payload = serde_json::to_string(&json!({
            "authoritative_episode": round.episode_payload,
            "initial_extraction": round.initial,
            "read_only_archive_evidence": {
                "results": evidence_results,
                "turns": evidence_payload_turns
            },
            "instruction": "Return final candidates now. evidence_requests MUST be empty; no second evidence round is allowed. Archive evidence may clarify a referent through grounding_source fields or supply earlier assistant-authored content explicitly adopted/retained through authority_source fields, but archive evidence cannot supply user authority."
        }))
        .map_err(|error| InsomniaExtractionError::InvalidOutput(error.to_string()))?;
        let final_output = self.complete_direct(&payload)?;
        if !parse_evidence_requests(&final_output)?.is_empty() {
            return Err(InsomniaExtractionError::InvalidOutput(
                "model requested more than one archive-evidence round".into(),
            ));
        }
        finalize_extraction(
            self.endpoint.model(),
            INSOMNIA_EXTRACTOR_CONTRACT_VERSION,
            episode,
            turns,
            &final_output,
            evidence_turns,
        )
    }

    fn complete_direct(&self, payload: &str) -> Result<Value, InsomniaExtractionError> {
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

pub(super) fn finalize_extraction(
    model: &str,
    contract_version: &str,
    episode: &Episode,
    turns: &[ResolvedTurn],
    value: &Value,
    evidence_turns: Vec<InsomniaEvidenceTurn>,
) -> Result<InsomniaExtraction, InsomniaExtractionError> {
    let raw = parse_candidates(value)?;
    let (candidates, rejected) = validate_candidates(episode.id, turns, raw);
    Ok(InsomniaExtraction {
        model: model.to_owned(),
        contract_version: contract_version.to_owned(),
        candidates,
        rejected,
        evidence_turns,
    })
}
