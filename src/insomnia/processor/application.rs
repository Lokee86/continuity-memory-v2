use super::prepared::{PreparedApplication, PreparedMemory, PreparedUserMemory, UserPublication};
use super::source_validation::{candidate_source_time_ns, validate_candidate_sources};
use super::{InsomniaProcessError, InsomniaProcessResult};
use crate::insomnia::candidate::hex;
use crate::insomnia::completion::{InsomniaCompletion, encode_completion};
use crate::insomnia::store::InsomniaStore;
use crate::memory_model::{memory_body_id, memory_id};
use crate::memory_store::MemoryStore;
use crate::{
    Archive, Container, Episode, INSOMNIA_EXTRACTOR_CONTRACT_VERSION, InsomniaExtraction,
    InsomniaOwnership, InsomniaRejection, InsomniaWork, Memory, MemoryDraft, MemoryRef,
    MemoryRoutingMetadata, MemorySourceRef, Phylactery,
};

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
        let temporal = crate::insomnia::temporal::assess_draft(&draft);
        let routing_metadata = candidate
            .routing_metadata
            .map(|routing| MemoryRoutingMetadata {
                memory_id: memory_id(&draft.mutation_id),
                body_id: memory_body_id(&draft.title, &draft.content),
                entity_mentions: routing.entity_mentions,
                lexical_terms: routing.lexical_terms,
            });
        match candidate.ownership {
            InsomniaOwnership::Project => project_drafts.push(PreparedMemory {
                draft,
                temporal,
                temporal_inference: None,
                routing_metadata,
            }),
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
                user_drafts.push(PreparedUserMemory {
                    memory: PreparedMemory {
                        draft,
                        temporal,
                        temporal_inference: None,
                        routing_metadata,
                    },
                    source_ref,
                });
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
        let temporal_inference = prepared.memory.bound_temporal_inference();
        let draft = &prepared.memory.draft;
        debug_assert_eq!(
            prepared.memory.temporal.analysis.source_timestamp_ns,
            draft.source_time_ns
        );
        let (memory, was_created) = match phylactery
            .publish_memory_with_source_ref_and_temporal_inference(
                None,
                0,
                draft.clone(),
                prepared.source_ref.clone(),
                temporal_inference,
            ) {
            Ok(value) => value,
            Err(crate::MemoryError::MutationConflict) => {
                let id = memory_id(&draft.mutation_id);
                let memory = phylactery.memory(id)?;
                if !same_routed_user_semantics(&memory, draft, &prepared.source_ref) {
                    return Err(crate::MemoryError::MutationConflict.into());
                }
                (memory, false)
            }
            Err(error) => return Err(error.into()),
        };
        if let Some(metadata) = prepared.memory.routing_metadata.clone() {
            phylactery
                .memories
                .put_routing_metadata(&mut phylactery.container, metadata)?;
        }
        let memory = phylactery.memory(memory.id)?;
        if was_created {
            created.push(memory.clone());
        } else {
            existing.push(memory.clone());
        }
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
    let project_drafts = prepared
        .project_drafts
        .into_iter()
        .map(|prepared| {
            debug_assert_eq!(
                prepared.temporal.analysis.source_timestamp_ns,
                prepared.draft.source_time_ns
            );
            let temporal_inference = prepared.bound_temporal_inference();
            (
                prepared.draft,
                temporal_inference,
                prepared.routing_metadata,
            )
        })
        .collect();
    let batch = memories.stage_grouped_insomnia(container, project_drafts)?;
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
        routing_metadata: batch.routing_metadata,
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
    let mut created = memories.apply_grouped_records(container, &completion.records)?;
    memories.apply_grouped_routing_metadata(&completion.routing_metadata)?;
    for memory in &mut created {
        *memory = memories.memory(container, memory.id)?;
    }
    insomnia.apply_completion(&completion)?;
    Ok(InsomniaProcessResult {
        created,
        existing,
        user_created: user_publication.created,
        user_existing: user_publication.existing,
        rejected: prepared.rejected,
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
