use crate::cva_reconcile_conflict_map::memory_replay_error;
use crate::insomnia::completion::{InsomniaCompletion, decode_completion, encode_completion};
use crate::memory_codec::{
    decode_record as decode_memory_record, decode_version as decode_memory_version,
};
use crate::{
    Cva, CvaReconcileConflict, CvaReconcileError, InsomniaAttempt, InsomniaWorkState, Memory,
    MemoryDraft,
};
use std::collections::HashSet;

pub(crate) struct MemoryTail {
    revisions: Vec<Memory>,
    completions: Vec<InsomniaCompletion>,
}

pub(crate) struct MemoryReplayResult {
    pub(crate) revisions: usize,
    pub(crate) duplicate_revisions: usize,
    pub(crate) completions: usize,
}

pub(crate) fn read_memory_tail(
    cva: &mut Cva,
    start_chunk: usize,
) -> Result<MemoryTail, CvaReconcileError> {
    let chunks = cva.container.chunks()?;
    let mut revisions = Vec::new();
    let mut completions = Vec::new();
    let mut seen = HashSet::new();

    for chunk in chunks.iter().skip(start_chunk) {
        let payload = cva.container.read(*chunk)?;
        if let Some(completion) =
            decode_completion(&payload).map_err(CvaReconcileError::InvalidInsomniaCompletion)?
        {
            for record in &completion.records {
                push_revision(cva, record.id, record.revision, &mut seen, &mut revisions)?;
            }
            completions.push(completion);
            continue;
        }
        let Some(version) = decode_memory_version(&payload)? else {
            continue;
        };
        let record_payload = cva.container.read(version.record)?;
        let record = decode_memory_record(&record_payload)?
            .ok_or(CvaReconcileError::InvalidMemoryVersionRecord)?;
        push_revision(cva, record.id, record.revision, &mut seen, &mut revisions)?;
    }

    Ok(MemoryTail {
        revisions,
        completions,
    })
}

pub(crate) fn replay_memory_tail(
    destination: &mut Cva,
    tail: MemoryTail,
) -> Result<MemoryReplayResult, CvaReconcileError> {
    let mut replayed = 0;
    let mut duplicates = 0;
    for memory in tail.revisions {
        let draft = memory_draft(memory.clone());
        let (_, created) = match destination.publish_memory(
            Some(memory.id),
            memory.revision.saturating_sub(1),
            draft,
        ) {
            Ok(value) => value,
            Err(error) => return Err(memory_replay_error(destination, &memory, error)),
        };
        if created {
            replayed += 1;
        } else {
            duplicates += 1;
        }
    }

    let mut completions = 0;
    for completion in tail.completions {
        if replay_completion(destination, completion)? {
            completions += 1;
        }
    }
    Ok(MemoryReplayResult {
        revisions: replayed,
        duplicate_revisions: duplicates,
        completions,
    })
}

fn push_revision(
    cva: &mut Cva,
    id: crate::MemoryId,
    revision: u64,
    seen: &mut HashSet<(crate::MemoryId, u64)>,
    revisions: &mut Vec<Memory>,
) -> Result<(), CvaReconcileError> {
    if seen.insert((id, revision)) {
        revisions.push(cva.memory_revision(id, revision)?);
    }
    Ok(())
}

fn replay_completion(
    destination: &mut Cva,
    completion: InsomniaCompletion,
) -> Result<bool, CvaReconcileError> {
    let existing = destination.insomnia_attempts(completion.episode_id);
    if !existing.is_empty() {
        if existing.len() == 1 && same_completion(&existing[0], &completion) {
            return Ok(false);
        }
        let existing = &existing[0];
        return Err(CvaReconcileError::Conflict(
            CvaReconcileConflict::InsomniaCompletion {
                episode_id: completion.episode_id,
                existing_model: existing.extractor_model.clone(),
                existing_version: existing.extractor_version.clone(),
                incoming_model: completion.extractor_model.clone(),
                incoming_version: completion.extractor_version.clone(),
            },
        ));
    }
    if completion
        .memory_ids
        .iter()
        .any(|id| !destination.memories.contains_memory(*id))
    {
        return Err(CvaReconcileError::MissingCompletionMemory);
    }

    let receipt = InsomniaCompletion {
        episode_id: completion.episode_id,
        attempt: completion.attempt,
        started_at_ns: completion.started_at_ns,
        completed_at_ns: completion.completed_at_ns,
        extractor_model: completion.extractor_model,
        extractor_version: completion.extractor_version,
        rejected_count: completion.rejected_count,
        memory_ids: completion.memory_ids,
        global_version_start: destination.container.next_version_candidate(),
        bodies: Vec::new(),
        records: Vec::new(),
    };
    let payload =
        encode_completion(&receipt).map_err(CvaReconcileError::InvalidInsomniaCompletion)?;
    destination.container.append(&payload)?;
    destination.insomnia.apply_completion(&receipt)?;
    Ok(true)
}

fn same_completion(attempt: &InsomniaAttempt, completion: &InsomniaCompletion) -> bool {
    attempt.state == InsomniaWorkState::Complete
        && attempt.attempt == completion.attempt
        && attempt.started_at_ns == completion.started_at_ns
        && attempt.completed_at_ns == completion.completed_at_ns
        && attempt.extractor_model == completion.extractor_model
        && attempt.extractor_version == completion.extractor_version
        && attempt.memory_ids == completion.memory_ids
        && attempt.rejected_count == completion.rejected_count
}

fn memory_draft(memory: Memory) -> MemoryDraft {
    MemoryDraft {
        category: memory.category,
        memory_type: memory.memory_type,
        authority_kind: memory.authority_kind,
        title: memory.title,
        content: memory.content,
        scope: memory.scope,
        lifecycle_state: memory.lifecycle_state,
        archived: memory.archived,
        superseded_by: memory.superseded_by,
        parent_id: memory.parent_id,
        source_node_id: memory.source_node_id,
        content_source_conversation_id: memory.content_source_conversation_id,
        content_source_node_id: memory.content_source_node_id,
        grounding_source_conversation_id: memory.grounding_source_conversation_id,
        grounding_source_node_id: memory.grounding_source_node_id,
        source_episode_id: memory.source_episode_id,
        mutation_id: memory.mutation_id,
        created_at_ns: memory.created_at_ns,
        updated_at_ns: memory.updated_at_ns,
    }
}
