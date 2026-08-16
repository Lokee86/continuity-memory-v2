use super::source_validation::validate_candidate_sources;
use super::{InsomniaProcessError, InsomniaProcessResult};
use crate::cva_memory_publish::publish_memory_parts;
use crate::insomnia::candidate::hex;
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

pub(crate) fn publish_draft(
    archive: &Archive,
    container: &mut Container,
    memories: &mut MemoryStore,
    draft: MemoryDraft,
    result: &mut InsomniaProcessResult,
) -> Result<(), InsomniaProcessError> {
    let (memory, created) = publish_memory_parts(archive, memories, container, None, 0, draft)?;
    if created {
        result.created.push(memory);
    } else {
        result.existing.push(memory);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn finish_application(
    container: &mut Container,
    memories: &MemoryStore,
    insomnia: &mut InsomniaStore,
    claim: &InsomniaWork,
    started_at_ns: i64,
    completed_at_ns: i64,
    model: String,
    contract_version: String,
    result: &InsomniaProcessResult,
) -> Result<(), InsomniaProcessError> {
    let memory_ids: Vec<_> = result
        .created
        .iter()
        .chain(&result.existing)
        .map(|memory| memory.id)
        .collect();
    if memory_ids.iter().any(|id| !memories.contains_memory(*id)) {
        return Err(crate::InsomniaError::InvalidTransition.into());
    }
    insomnia.complete(
        container,
        claim.episode_id,
        claim.lease_token.unwrap(),
        started_at_ns,
        completed_at_ns,
        model,
        contract_version,
        memory_ids,
        result.rejected.len() as u32,
    )?;
    Ok(())
}
