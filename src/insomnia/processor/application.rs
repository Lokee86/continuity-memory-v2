use super::source_validation::validate_candidate_sources;
use super::{InsomniaProcessError, InsomniaProcessResult};
use crate::insomnia::candidate::hex;
use crate::insomnia::completion::{InsomniaCompletion, encode_completion};
use crate::insomnia::store::InsomniaStore;
use crate::memory_model::memory_id;
use crate::memory_store::MemoryStore;
use crate::{
    Archive, Container, Episode, INSOMNIA_EXTRACTOR_CONTRACT_VERSION, InsomniaExtraction,
    InsomniaOwnership, InsomniaRejection, InsomniaWork, Memory, MemoryDraft, MemoryRef,
    MemorySourceRef, Phylactery,
};

pub(crate) struct PreparedApplication {
    pub(crate) project_drafts: Vec<MemoryDraft>,
    pub(crate) user_drafts: Vec<PreparedUserMemory>,
    pub(crate) rejected: Vec<InsomniaRejection>,
    pub(crate) model: String,
}

pub(crate) struct PreparedUserMemory {
    pub(crate) draft: MemoryDraft,
    pub(crate) source_ref: MemorySourceRef,
}

pub(crate) struct UserPublication {
    pub(crate) created: Vec<Memory>,
    pub(crate) existing: Vec<Memory>,
    pub(crate) refs: Vec<MemoryRef>,
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
    let mut rejected = extraction.rejected;
    let mut project_drafts = Vec::with_capacity(extraction.candidates.len());
    let mut user_drafts = Vec::new();
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
        let source_time_ns = candidate_source_time_ns(archive, &candidate, turns)?;
        let mut draft = MemoryDraft {
            category: candidate.category,
            memory_type: candidate.memory_type,
            authority_kind: candidate.authority_kind,
            temporal_status: candidate.temporal_status,
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
            source_time_ns: Some(source_time_ns),
            mutation_id: format!("insomnia:{}:{}", hex(&episode.id.0), candidate.key),
            created_at_ns: episode.source_through_ns,
            updated_at_ns: completed_at_ns.max(episode.source_through_ns),
        };
        match candidate.ownership {
            InsomniaOwnership::Project => project_drafts.push(draft),
            InsomniaOwnership::User => {
                let owner_id = container.owner_id().ok_or_else(|| {
                    InsomniaProcessError::InvalidCandidate(
                        "source Reliquary has no durable owner ID; migrate it before routing user Memory provenance"
                            .into(),
                    )
                })?;
                let source_ref = MemorySourceRef {
                    owner_id,
                    source_episode_id: episode.id,
                    source_node_id: draft.source_node_id.clone().ok_or_else(|| {
                        InsomniaProcessError::InvalidCandidate(
                            "user Memory source node is unavailable".into(),
                        )
                    })?,
                    content_source_conversation_id: draft.content_source_conversation_id.clone(),
                    content_source_node_id: draft.content_source_node_id.clone(),
                    grounding_source_conversation_id: draft
                        .grounding_source_conversation_id
                        .clone(),
                    grounding_source_node_id: draft.grounding_source_node_id.clone(),
                };
                draft.source_node_id = None;
                draft.content_source_conversation_id = None;
                draft.content_source_node_id = None;
                draft.grounding_source_conversation_id = None;
                draft.grounding_source_node_id = None;
                draft.source_episode_id = None;
                user_drafts.push(PreparedUserMemory { draft, source_ref });
            }
        }
    }
    Ok(PreparedApplication {
        project_drafts,
        user_drafts,
        rejected,
        model: extraction.model,
    })
}

pub(crate) fn publish_user_application(
    phylactery: &mut Phylactery,
    drafts: &[PreparedUserMemory],
) -> Result<UserPublication, InsomniaProcessError> {
    let owner_id = phylactery.owner_id().ok_or_else(|| {
        InsomniaProcessError::Phylactery(
            "Phylactery has no durable owner ID; migrate it before routed Insomnia processing"
                .into(),
        )
    })?;
    let mut created = Vec::new();
    let mut existing = Vec::new();
    let mut refs = Vec::with_capacity(drafts.len());
    for prepared in drafts {
        let draft = &prepared.draft;
        let memory = match phylactery.publish_memory_with_source_ref(
            None,
            0,
            draft.clone(),
            prepared.source_ref.clone(),
        ) {
            Ok((memory, was_created)) => {
                if was_created {
                    created.push(memory.clone());
                } else {
                    existing.push(memory.clone());
                }
                memory
            }
            Err(crate::MemoryError::MutationConflict) => {
                let id = memory_id(&draft.mutation_id);
                let memory = phylactery.memory(id)?;
                if !same_routed_user_semantics(&memory, draft, &prepared.source_ref) {
                    return Err(crate::MemoryError::MutationConflict.into());
                }
                existing.push(memory.clone());
                memory
            }
            Err(error) => return Err(error.into()),
        };
        refs.push(MemoryRef {
            owner_id: owner_id.clone(),
            memory_id: memory.id,
        });
    }
    phylactery
        .sync()
        .map_err(|error| InsomniaProcessError::Phylactery(error.to_string()))?;
    Ok(UserPublication {
        created,
        existing,
        refs,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn commit_application(
    container: &mut Container,
    memories: &mut MemoryStore,
    insomnia: &mut InsomniaStore,
    claim: &InsomniaWork,
    prepared: PreparedApplication,
    user_publication: Option<UserPublication>,
    started_at_ns: i64,
    completed_at_ns: i64,
) -> Result<InsomniaProcessResult, InsomniaProcessError> {
    let token = claim
        .lease_token
        .ok_or(InsomniaProcessError::InvalidClaim)?;
    insomnia.active_claim(claim.episode_id, token, completed_at_ns)?;
    if !prepared.user_drafts.is_empty() && user_publication.is_none() {
        return Err(InsomniaProcessError::UserRoutingRequired);
    }
    let user_publication = user_publication.unwrap_or(UserPublication {
        created: Vec::new(),
        existing: Vec::new(),
        refs: Vec::new(),
    });
    let batch = memories.stage_grouped_insomnia(container, prepared.project_drafts)?;
    let existing = batch.existing;
    let mut memory_ids: Vec<_> = batch.records.iter().map(|record| record.id).collect();
    memory_ids.extend(existing.iter().map(|memory| memory.id));
    let transaction_time_ns =
        Container::transaction_time_now_ns().map_err(crate::InsomniaError::from)?;
    let completion = InsomniaCompletion {
        episode_id: claim.episode_id,
        attempt: claim.attempt_count,
        started_at_ns,
        completed_at_ns,
        transaction_time_ns,
        extractor_model: prepared.model,
        extractor_version: INSOMNIA_EXTRACTOR_CONTRACT_VERSION.into(),
        rejected_count: prepared.rejected.len() as u32,
        memory_ids,
        external_memory_refs: user_publication.refs.clone(),
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
        .commit_embedded_version_range_at(
            completion.global_version_start,
            completion.records.len(),
            Some(completion.transaction_time_ns),
        )
        .map_err(crate::InsomniaError::from)?;
    container.sync().map_err(crate::InsomniaError::from)?;
    memories.apply_grouped_bodies(completion_chunk, &completion.bodies)?;
    let created = memories.apply_grouped_records(container, &completion.records)?;
    insomnia.apply_completion(&completion)?;
    Ok(InsomniaProcessResult {
        created,
        existing,
        user_created: user_publication.created,
        user_existing: user_publication.existing,
        rejected: prepared.rejected,
    })
}

fn candidate_source_time_ns(
    archive: &Archive,
    candidate: &crate::InsomniaCandidate,
    turns: &[crate::ResolvedTurn],
) -> Result<i64, InsomniaProcessError> {
    if let (Some(conversation_id), Some(node_id)) = (
        candidate.authority_source_conversation_id.as_deref(),
        candidate.authority_source_node_id.as_deref(),
    ) {
        return archive
            .nodes
            .get(conversation_id, node_id)
            .map(|node| node.timestamp_ns)
            .ok_or_else(|| {
                InsomniaProcessError::InvalidCandidate(
                    "authority source timestamp is unavailable".into(),
                )
            });
    }
    turns
        .iter()
        .find(|turn| turn.node_id == candidate.source_node_id)
        .map(|turn| turn.timestamp_ns)
        .ok_or_else(|| {
            InsomniaProcessError::InvalidCandidate("source timestamp is unavailable".into())
        })
}

fn same_routed_user_semantics(
    memory: &Memory,
    draft: &MemoryDraft,
    source_ref: &MemorySourceRef,
) -> bool {
    memory.category == draft.category
        && memory.memory_type == draft.memory_type
        && memory.authority_kind == draft.authority_kind
        && memory.temporal_status == draft.temporal_status
        && memory.scope == draft.scope
        && memory.lifecycle_state == draft.lifecycle_state
        && memory.archived == draft.archived
        && memory.superseded_by == draft.superseded_by
        && memory.parent_id == draft.parent_id
        && memory.source_node_id.is_none()
        && memory.content_source_conversation_id.is_none()
        && memory.content_source_node_id.is_none()
        && memory.grounding_source_conversation_id.is_none()
        && memory.grounding_source_node_id.is_none()
        && memory.source_episode_id.is_none()
        && memory.source_time_ns == draft.source_time_ns
        && memory.source_ref.as_ref() == Some(source_ref)
        && memory.mutation_id == draft.mutation_id
        && memory.created_at_ns == draft.created_at_ns
}
