use super::{MigrationError, op, require_same};
use crate::graph_codec::{decode_batch, decode_mutation, decode_version};
use crate::{Container, GraphRelationChange, Memory, MemoryDraft, Phylactery};
use std::path::Path;

pub(super) fn migrate(
    source_path: &Path,
    output_path: &Path,
    owner_uuid: [u8; 16],
) -> Result<String, MigrationError> {
    let mut source = op(Phylactery::open(source_path))?;
    let revisions: Vec<_> = source
        .memories
        .records()
        .iter()
        .map(|record| (record.id, record.revision))
        .collect();
    let graph = graph_transactions(&mut source.container)?;
    let profiles = source.compatibility_profiles.profiles();
    let packed_infos = source.packed_vectors.infos();
    let memory_vector_infos = source.memory_vectors.infos();
    let dream_cooldowns = source.dream_cooldown_records();
    let dream_pairs = source.dream_pair_records();

    let mut output = op(Phylactery::create_with_uuid(output_path, owner_uuid))?;
    for profile in profiles {
        op(output
            .compatibility_profiles
            .put(&mut output.container, profile))?;
    }
    for (id, revision) in revisions {
        let memory = op(source.memory_revision(id, revision))?;
        let transaction_time_ns = source.transaction_time_ns(memory.global_version);
        output
            .container
            .set_next_transaction_time_override(transaction_time_ns);
        let result =
            output.publish_memory(Some(id), revision.saturating_sub(1), memory_draft(memory));
        output.container.clear_next_transaction_time_override();
        op(result)?;
    }
    for (transaction, transaction_time_ns) in graph {
        let graph_version = output.graph_version();
        output
            .container
            .set_next_transaction_time_override(transaction_time_ns);
        let result = output.set_memory_relations(&transaction, graph_version);
        output.container.clear_next_transaction_time_override();
        op(result)?;
    }
    for (id, state) in dream_cooldowns {
        let processed_at_ns = match state.processed_at_ns {
            Some(value) => value,
            None => op(output.memory(id))?.updated_at_ns,
        };
        op(output.mark_dream_processed(id, state.epoch, processed_at_ns))?;
    }
    for (left, right) in dream_pairs {
        op(output.mark_dream_pair_evaluated(left, right))?;
    }
    copy_vectors(&mut source, &mut output, packed_infos, memory_vector_infos)?;
    op(output.sync())?;
    let owner_id = output
        .owner_id()
        .ok_or_else(|| MigrationError::Operation("migrated PHY has no owner ID".into()))?;
    drop(output);
    op(Phylactery::open(output_path))?;
    Ok(owner_id)
}

fn copy_vectors(
    source: &mut Phylactery,
    output: &mut Phylactery,
    packed_infos: Vec<crate::PackedVectorInfo>,
    memory_vector_infos: Vec<crate::MemoryVectorInfo>,
) -> Result<(), MigrationError> {
    for info in packed_infos {
        let packed = op(source.packed_vectors.get(&mut source.container, info.id))?;
        require_same(
            info.id,
            op(output.packed_vectors.put(&mut output.container, packed))?,
            "packed vector",
        )?;
    }
    for info in memory_vector_infos {
        let set = op(source.memory_vectors.get(&mut source.container, info.id))?;
        let id = op(output.memory_vectors.put(
            &mut output.container,
            &output.memories,
            &output.compatibility_profiles,
            &output.packed_vectors,
            set.compatibility_profile_id,
            set.packed_vector_id,
            set.memory_body_ids,
        ))?;
        require_same(info.id, id, "Memory Vector")?;
    }
    Ok(())
}

fn graph_transactions(
    container: &mut Container,
) -> Result<Vec<(Vec<GraphRelationChange>, Option<i64>)>, MigrationError> {
    let chunks = op(container.chunks())?;
    let mut transactions = Vec::new();
    for chunk in chunks {
        let payload = op(container.read(chunk))?;
        let Some(version) = op(decode_version(&payload))? else {
            continue;
        };
        let mutation_payload = op(container.read(version.mutation))?;
        let changes = if let Some(batch) = op(decode_batch(&mutation_payload))? {
            batch
                .into_iter()
                .map(|mutation| GraphRelationChange {
                    source: mutation.source,
                    target: mutation.target,
                    kind: mutation.kind,
                    active: mutation.active,
                })
                .collect()
        } else if let Some(mutation) = op(decode_mutation(&mutation_payload))? {
            vec![GraphRelationChange {
                source: mutation.source,
                target: mutation.target,
                kind: mutation.kind,
                active: mutation.active,
            }]
        } else {
            return Err(MigrationError::Operation(
                "graph version points at a non-graph mutation".into(),
            ));
        };
        transactions.push((
            changes,
            container.transaction_time_ns(version.global_version),
        ));
    }
    Ok(transactions)
}

fn memory_draft(memory: Memory) -> MemoryDraft {
    MemoryDraft {
        category: memory.category,
        memory_type: memory.memory_type,
        authority_kind: memory.authority_kind,
        temporal_status: memory.temporal_status,
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
        source_time_ns: memory.source_time_ns,
        mutation_id: memory.mutation_id,
        created_at_ns: memory.created_at_ns,
        updated_at_ns: memory.updated_at_ns,
    }
}
