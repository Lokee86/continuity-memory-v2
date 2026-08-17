use super::source_validation::validate_candidate_sources;
use super::{InsomniaProcessError, InsomniaProcessResult};
use crate::insomnia::candidate::hex;
use crate::insomnia::completion::{InsomniaCompletion, encode_completion};
use crate::insomnia::store::InsomniaStore;
use crate::memory_store::MemoryStore;
use crate::{
    Archive, Container, Episode, InsomniaExtraction, InsomniaRejection, InsomniaWork, MemoryDraft,
};

pub(crate) struct PreparedApplication {
    pub(crate) drafts: Vec<MemoryDraft>,
    pub(crate) rejected: Vec<InsomniaRejection>,
    pub(crate) model: String,
    pub(crate) contract_version: String,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_application(
    archive: &Archive,
    container: &mut Container,
    episode: &Episode,
    turns: &[crate::ResolvedTurn],
    extraction: InsomniaExtraction,
    scope: &str,
    completed_at_ns: i64,
) -> Result<PreparedApplication, InsomniaProcessError> {
    let contract_version = extraction.contract_version;
    let mut rejected = extraction.rejected;
    let mut drafts = Vec::with_capacity(extraction.candidates.len());
    for candidate in extraction.candidates {
        if let Err(reason) = validate_candidate_sources(
            archive,
            container,
            &candidate,
            &episode.conversation_id,
            turns,
            &extraction.evidence_turns,
        ) {
            rejected.push(InsomniaRejection {
                candidate_key: Some(candidate.key.clone()),
                reason,
            });
            continue;
        }
        drafts.push(MemoryDraft {
            category: candidate.category,
            memory_type: candidate.memory_type,
            title: candidate.title,
            content: candidate.content,
            scope: scope.trim().to_owned(),
            lifecycle_state: "extracted".into(),
            archived: false,
            superseded_by: None,
            parent_id: None,
            source_node_id: Some(candidate.source_node_id),
            content_source_conversation_id: candidate.authority_source_conversation_id,
            content_source_node_id: candidate.authority_source_node_id,
            grounding_source_conversation_id: candidate.grounding_source_conversation_id,
            grounding_source_node_id: candidate.grounding_source_node_id,
            source_episode_id: Some(episode.id),
            mutation_id: format!("insomnia:{}:{}", hex(&episode.id.0), candidate.key),
            created_at_ns: episode.source_through_ns,
            updated_at_ns: completed_at_ns.max(episode.source_through_ns),
        });
    }
    Ok(PreparedApplication {
        drafts,
        rejected,
        model: extraction.model,
        contract_version,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn commit_application(
    container: &mut Container,
    memories: &mut MemoryStore,
    insomnia: &mut InsomniaStore,
    claim: &InsomniaWork,
    prepared: PreparedApplication,
    started_at_ns: i64,
    completed_at_ns: i64,
) -> Result<InsomniaProcessResult, InsomniaProcessError> {
    let token = claim
        .lease_token
        .ok_or(InsomniaProcessError::InvalidClaim)?;
    insomnia.active_claim(claim.episode_id, token, completed_at_ns)?;
    let batch = memories.stage_grouped_insomnia(container, prepared.drafts)?;
    let existing = batch.existing;
    let mut memory_ids: Vec<_> = batch.records.iter().map(|record| record.id).collect();
    memory_ids.extend(existing.iter().map(|memory| memory.id));
    let completion = InsomniaCompletion {
        episode_id: claim.episode_id,
        attempt: claim.attempt_count,
        started_at_ns,
        completed_at_ns,
        extractor_model: prepared.model,
        extractor_version: prepared.contract_version,
        rejected_count: prepared.rejected.len() as u32,
        memory_ids,
        global_version_start: batch.global_version_start,
        bodies: batch.bodies,
        records: batch.records,
    };
    let payload = encode_completion(&completion)
        .map_err(|_| crate::InsomniaError::InvalidField("completion record"))?;
    let completion_chunk = container
        .append(&payload)
        .map_err(crate::InsomniaError::from)?;
    container
        .commit_embedded_version_range(completion.global_version_start, completion.records.len())
        .map_err(crate::InsomniaError::from)?;
    container.sync().map_err(crate::InsomniaError::from)?;
    memories.apply_grouped_bodies(completion_chunk, &completion.bodies)?;
    let created = memories.apply_grouped_records(container, &completion.records)?;
    insomnia.apply_completion(&completion)?;
    Ok(InsomniaProcessResult {
        created,
        existing,
        rejected: prepared.rejected,
    })
}
