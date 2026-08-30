use crate::compatibility_vector::cosine;
use crate::graph_store::GraphStore;
use crate::memory_retrieval_index::build_subcentroids;
use crate::memory_retrieval_vectors::load_memory_vectors;
use crate::memory_store::MemoryStore;
use crate::memory_vector_store::MemoryVectorStore;
use crate::packed_vector_store::PackedVectorStore;
use crate::{
    COMMUNITY_ALGORITHM_VERSION, CommunityId, CommunitySnapshot, CompatibilityProfileId, Container,
    MemoryId, MemoryRetrievalError, MemoryRetrievalIndex,
};
use std::collections::HashMap;

pub(crate) fn build_memory_retrieval_index(
    container: &mut Container,
    memories: &MemoryStore,
    graph: &GraphStore,
    snapshot: Option<&CommunitySnapshot>,
    memory_vectors: &MemoryVectorStore,
    packed_vectors: &PackedVectorStore,
    profile_id: CompatibilityProfileId,
    subcentroids_per_community: usize,
) -> Result<MemoryRetrievalIndex, MemoryRetrievalError> {
    if subcentroids_per_community == 0 {
        return Err(MemoryRetrievalError::InvalidConfig);
    }
    let by_body = load_memory_vectors(
        container,
        memories,
        memory_vectors,
        packed_vectors,
        profile_id,
    )?;
    let mut searchable = std::collections::HashSet::new();
    let mut vectors = HashMap::new();
    for id in memories.current_ids() {
        let memory = memories.memory(container, id)?;
        if memory.archived {
            continue;
        }
        searchable.insert(id);
        let body_id = memories.current_body_id(id)?;
        if let Some(vector) = by_body.get(&body_id) {
            if cosine(vector, vector).is_none() {
                return Err(MemoryRetrievalError::CorruptVector("invalid stored vector"));
            }
            vectors.insert(id, vector.clone());
        }
    }
    let dimensions = vectors
        .values()
        .next()
        .map(|vector| vector.len())
        .ok_or(MemoryRetrievalError::EmptyPopulation)?;
    if vectors.values().any(|vector| vector.len() != dimensions) {
        return Err(MemoryRetrievalError::CorruptVector("vector dimensions"));
    }

    let current_snapshot = snapshot.filter(|snapshot| {
        snapshot.derived_graph_version == graph.graph_version()
            && snapshot.algorithm_version == COMMUNITY_ALGORITHM_VERSION
    });
    let memberships = current_snapshot.map(membership_map).unwrap_or_default();
    let subcentroids = current_snapshot
        .map(|snapshot| build_subcentroids(snapshot, &vectors, subcentroids_per_community))
        .unwrap_or_default();

    Ok(MemoryRetrievalIndex {
        compatibility_profile_id: profile_id,
        memory_version: memories.memory_version(),
        graph_version: graph.graph_version(),
        community_generation: snapshot.map(|snapshot| snapshot.generation),
        vector_bindings: memory_vectors.binding_count(profile_id),
        subcentroids_per_community,
        dimensions,
        indexed_memories: vectors.len(),
        routing_vectors: subcentroids.len(),
        vectors,
        searchable,
        memberships,
        subcentroids,
    })
}

fn membership_map(snapshot: &CommunitySnapshot) -> HashMap<MemoryId, CommunityId> {
    snapshot
        .communities
        .iter()
        .flat_map(|community| {
            community
                .members
                .iter()
                .map(move |member| (*member, community.id))
        })
        .collect()
}
