use crate::dream_duplicate_index::{DuplicateIndex, DuplicateTemporalKey};
use crate::graph_store::GraphStore;
use crate::memory_store::MemoryStore;
use crate::{
    Container, DreamPublicationError, DreamPublicationOutcome, GraphRelationKind, Memory, MemoryId,
};
use std::collections::{HashMap, HashSet};

pub(crate) fn publish_duplicate_pair<R>(
    container: &mut Container,
    memories: &MemoryStore,
    graph: &mut GraphStore,
    duplicate_index: &mut DuplicateIndex,
    a: MemoryId,
    b: MemoryId,
    expected_graph_version: u64,
    source_time: &R,
) -> Result<DreamPublicationOutcome, DreamPublicationError>
where
    R: Fn(&Memory) -> Option<i64>,
{
    if expected_graph_version != graph.graph_version() {
        return Err(crate::GraphError::RevisionConflict {
            expected: expected_graph_version,
            actual: graph.graph_version(),
        }
        .into());
    }
    ensure_duplicate_index(container, memories, graph, duplicate_index, source_time)?;
    let a_key = duplicate_key(container, memories, a, source_time)?;
    let b_key = duplicate_key(container, memories, b, source_time)?;
    let relations = graph.active_relations();
    let mut next_index = duplicate_index.clone();
    let mut changes = next_index.plan_union(a, a_key, b, b_key, &relations);
    for relation in relations.iter().filter(|relation| {
        same_pair(a, b, relation.source, relation.target) && is_nonduplicate_primary(relation.kind)
    }) {
        changes.push(crate::GraphRelationChange {
            source: relation.source,
            target: relation.target,
            kind: relation.kind,
            active: false,
        });
    }
    changes.sort_by_key(|change| {
        (
            change.source.0,
            change.target.0,
            change.kind.code(),
            change.active,
        )
    });
    if changes.is_empty() {
        return Ok(DreamPublicationOutcome::NoChange);
    }
    let published = graph.set_relations(container, memories, &changes, expected_graph_version)?;
    next_index.set_graph_version(graph.graph_version());
    *duplicate_index = next_index;
    Ok(DreamPublicationOutcome::Published(published))
}

fn ensure_duplicate_index<R>(
    container: &mut Container,
    memories: &MemoryStore,
    graph: &GraphStore,
    duplicate_index: &mut DuplicateIndex,
    source_time: &R,
) -> Result<(), DreamPublicationError>
where
    R: Fn(&Memory) -> Option<i64>,
{
    let graph_version = graph.graph_version();
    if duplicate_index.graph_version() == Some(graph_version) {
        return Ok(());
    }
    let relations = graph.active_relations();
    let ids: HashSet<_> = relations
        .iter()
        .filter(|relation| relation.kind == GraphRelationKind::DuplicateOf && relation.active)
        .flat_map(|relation| [relation.source, relation.target])
        .collect();
    let mut keys = HashMap::with_capacity(ids.len());
    for memory_id in ids {
        keys.insert(
            memory_id,
            duplicate_key(container, memories, memory_id, source_time)?,
        );
    }
    *duplicate_index = DuplicateIndex::rebuild(graph_version, &relations, &keys);
    Ok(())
}

fn duplicate_key<R>(
    container: &mut Container,
    memories: &MemoryStore,
    memory_id: MemoryId,
    source_time: &R,
) -> Result<DuplicateTemporalKey, DreamPublicationError>
where
    R: Fn(&Memory) -> Option<i64>,
{
    let memory = memories.memory(container, memory_id)?;
    let timestamp_ns =
        source_time(&memory).ok_or(DreamPublicationError::MissingSourceTimestamp(memory_id))?;
    Ok(DuplicateTemporalKey::new(timestamp_ns, memory_id))
}

fn same_pair(a: MemoryId, b: MemoryId, source: MemoryId, target: MemoryId) -> bool {
    (source == a && target == b) || (source == b && target == a)
}

fn is_nonduplicate_primary(kind: GraphRelationKind) -> bool {
    matches!(
        kind,
        GraphRelationKind::Topical
            | GraphRelationKind::Factual
            | GraphRelationKind::Causal
            | GraphRelationKind::Recurrent
            | GraphRelationKind::Supersedes
    )
}
