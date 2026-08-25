use continuity_memory::{
    Cva, DreamMemoryContext, GraphRelation, MemoryDraft, MemoryId, PackedVectors, ScalarType,
    SimulatedEmbeddingEndpoint, VectorNormalization, VectorSchema,
};
use std::error::Error;
use std::fs;
use std::path::Path;

pub const AUG20_NS: i64 = 1_787_184_000_000_000_000;
pub const AUG22_NS: i64 = 1_787_356_800_000_000_000;
pub const AUG24_NS: i64 = 1_787_529_600_000_000_000;
pub const AUG25_NS: i64 = 1_787_616_000_000_000_000;

pub fn reset_cva(path: &Path) -> Result<Cva, Box<dyn Error>> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(Cva::create(path)?)
}

pub fn memory(
    cva: &mut Cva,
    mutation_id: &str,
    title: &str,
    content: &str,
    timestamp_ns: i64,
    lifecycle_state: &str,
) -> Result<MemoryId, Box<dyn Error>> {
    let node_id = format!("node-{mutation_id}");
    let conversation_id = format!("dream-write-{mutation_id}");
    cva.append_node(
        node_id.clone(),
        conversation_id.clone(),
        None,
        "user".into(),
        timestamp_ns,
        content,
    )?;
    let draft = MemoryDraft {
        category: "validation".into(),
        memory_type: "fact".into(),
        authority_kind: "unknown".into(),
        title: title.into(),
        content: content.into(),
        scope: "private".into(),
        lifecycle_state: lifecycle_state.into(),
        archived: lifecycle_state == "archived",
        superseded_by: None,
        parent_id: None,
        source_node_id: None,
        content_source_conversation_id: Some(conversation_id),
        content_source_node_id: Some(node_id),
        grounding_source_conversation_id: None,
        grounding_source_node_id: None,
        source_episode_id: None,
        mutation_id: mutation_id.into(),
        created_at_ns: 999,
        updated_at_ns: 999,
    };
    Ok(cva.publish_memory(None, 0, draft)?.0.id)
}

pub fn install_vectors(
    cva: &mut Cva,
    ids: &[MemoryId],
    vectors: &[&[f32]],
) -> Result<continuity_memory::CompatibilityProfileId, Box<dyn Error>> {
    if ids.len() != vectors.len() || ids.is_empty() {
        return Err("vector fixture must contain matching non-empty ids/vectors".into());
    }
    let dimensions = u32::try_from(vectors[0].len())?;
    let endpoint = SimulatedEmbeddingEndpoint::new(dimensions, VectorNormalization::L2, 91);
    let profile = cva.establish_compatibility_profile(&endpoint)?;
    let mut bytes = Vec::new();
    for vector in vectors {
        if vector.len() != dimensions as usize {
            return Err("vector fixture dimension mismatch".into());
        }
        for value in *vector {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    let schema = VectorSchema::new(dimensions, ScalarType::F32)?;
    let packed_id = cva.put_packed_vectors(PackedVectors::from_bytes(schema, bytes)?)?;
    let body_ids = ids
        .iter()
        .map(|id| cva.memory_body_id(*id))
        .collect::<Result<Vec<_>, _>>()?;
    cva.put_memory_vectors(profile.id, packed_id, body_ids)?;
    Ok(profile.id)
}

pub fn context(cva: &mut Cva, id: MemoryId) -> Result<DreamMemoryContext, Box<dyn Error>> {
    let memory = cva.memory(id)?;
    let body_id = cva.memory_body_id(id)?;
    let temporal = cva.dream_temporal_analysis(id)?;
    let graph_relations = cva
        .graph_relations()
        .into_iter()
        .filter(|relation| relation.source == id || relation.target == id)
        .collect::<Vec<GraphRelation>>();
    Ok(DreamMemoryContext {
        memory,
        body_id,
        source_timestamp_ns: temporal.source_timestamp_ns,
        graph_relations,
        temporal,
    })
}

pub fn short(id: MemoryId) -> String {
    id.0.iter()
        .take(4)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
