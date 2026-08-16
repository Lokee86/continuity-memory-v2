use super::candidate::validate_candidates;
use super::evidence::parse_evidence_requests;
use super::extraction::{
    InsomniaEvidenceMode, InsomniaEvidenceResult, InsomniaEvidenceRound, InsomniaEvidenceTurn,
    InsomniaExtraction, InsomniaExtractionError, InsomniaExtractionStage,
};
use super::ledger::{DispositionLedger, parse_ledger};
use super::ledger_contract::{
    INSOMNIA_DISPOSITION_SYSTEM_PROMPT, INSOMNIA_LEDGER_CONTRACT_VERSION,
    INSOMNIA_SYNTHESIS_SYSTEM_PROMPT, insomnia_ledger_schema, insomnia_synthesis_schema,
};
use super::ledger_synthesis::parse_synthesis;
use super::payload::{encode_episode_value, evidence_result_json, evidence_turn_json};
use crate::{Episode, GeneralEndpoint, ResolvedTurn};
use serde_json::{Value, json};

pub(super) fn start<E: GeneralEndpoint>(
    endpoint: &E,
    episode: &Episode,
    turns: &[ResolvedTurn],
) -> Result<InsomniaExtractionStage, InsomniaExtractionError> {
    let episode_payload = encode_episode_value(episode, turns)?;
    let payload = serde_json::to_string(&episode_payload).map_err(invalid_json)?;
    let initial = complete_ledger(endpoint, &payload)?;
    let requests = parse_evidence_requests(&initial)?;
    if requests.is_empty() {
        return finalize(endpoint, episode, turns, &initial, Vec::new())
            .map(InsomniaExtractionStage::Complete);
    }
    Ok(InsomniaExtractionStage::Evidence(InsomniaEvidenceRound {
        mode: InsomniaEvidenceMode::Ledger,
        episode_payload,
        initial,
        requests,
    }))
}

pub(super) fn finish_evidence<E: GeneralEndpoint>(
    endpoint: &E,
    episode: &Episode,
    turns: &[ResolvedTurn],
    round: InsomniaEvidenceRound,
    results: Vec<InsomniaEvidenceResult>,
    evidence_turns: Vec<InsomniaEvidenceTurn>,
) -> Result<InsomniaExtraction, InsomniaExtractionError> {
    let evidence_results: Vec<_> = results.iter().map(evidence_result_json).collect();
    let evidence_payload_turns: Vec<_> = evidence_turns.iter().map(evidence_turn_json).collect();
    let payload = serde_json::to_string(&json!({
        "authoritative_episode": round.episode_payload,
        "initial_ledger": round.initial,
        "read_only_archive_evidence": {
            "results": evidence_results,
            "turns": evidence_payload_turns
        },
        "instruction": "Return the final complete disposition ledger now. Account for every user turn exactly once. evidence_requests MUST be empty; no second evidence round is allowed. Evidence may fill an adopted assistant authority source or resolve a referent through grounding, but it never supplies user authority."
    }))
    .map_err(invalid_json)?;
    let final_ledger = complete_ledger(endpoint, &payload)?;
    if !parse_evidence_requests(&final_ledger)?.is_empty() {
        return Err(InsomniaExtractionError::InvalidOutput(
            "ledger requested more than one archive-evidence round".into(),
        ));
    }
    finalize(endpoint, episode, turns, &final_ledger, evidence_turns)
}

fn finalize<E: GeneralEndpoint>(
    endpoint: &E,
    episode: &Episode,
    turns: &[ResolvedTurn],
    ledger_value: &Value,
    evidence_turns: Vec<InsomniaEvidenceTurn>,
) -> Result<InsomniaExtraction, InsomniaExtractionError> {
    let ledger = parse_ledger(ledger_value, turns)?;
    if ledger.retained.is_empty() {
        return Ok(InsomniaExtraction {
            model: endpoint.model().to_owned(),
            contract_version: INSOMNIA_LEDGER_CONTRACT_VERSION.into(),
            candidates: Vec::new(),
            rejected: Vec::new(),
            evidence_turns,
        });
    }
    let payload = synthesis_payload(&ledger)?;
    let synthesis = endpoint.complete_json(
        INSOMNIA_SYNTHESIS_SYSTEM_PROMPT,
        &payload,
        "insomnia_memory_synthesis",
        &insomnia_synthesis_schema(),
    )?;
    let raw = parse_synthesis(&synthesis, &ledger)?;
    let (candidates, rejected) = validate_candidates(episode.id, turns, raw);
    Ok(InsomniaExtraction {
        model: endpoint.model().to_owned(),
        contract_version: INSOMNIA_LEDGER_CONTRACT_VERSION.into(),
        candidates,
        rejected,
        evidence_turns,
    })
}

fn complete_ledger<E: GeneralEndpoint>(
    endpoint: &E,
    payload: &str,
) -> Result<Value, InsomniaExtractionError> {
    endpoint
        .complete_json(
            INSOMNIA_DISPOSITION_SYSTEM_PROMPT,
            payload,
            "insomnia_authority_disposition_ledger",
            &insomnia_ledger_schema(),
        )
        .map_err(Into::into)
}

fn synthesis_payload(ledger: &DispositionLedger) -> Result<String, InsomniaExtractionError> {
    let units: Vec<_> = ledger
        .retained
        .iter()
        .map(|unit| {
            json!({
                "ledger_id": unit.id,
                "merge_group": unit.merge_group(),
                "source_node_id": unit.source_node_id,
                "source_quote": unit.source_quote,
                "authority_kind": unit.authority_kind,
                "category": unit.category,
                "type": unit.memory_type,
                "lifecycle": unit.lifecycle,
                "proposition": unit.proposition,
            })
        })
        .collect();
    serde_json::to_string(&json!({"retained_units": units})).map_err(invalid_json)
}

fn invalid_json(error: serde_json::Error) -> InsomniaExtractionError {
    InsomniaExtractionError::InvalidOutput(error.to_string())
}
