use crate::{
    Cva, MemoryDraft, MemoryId, PackedVectors, ScalarType, SimulatedEmbeddingEndpoint,
    VectorNormalization, VectorSchema,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn test_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("continuity-dream-candidates-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

pub(crate) fn memory(
    cva: &mut Cva,
    mutation_id: &str,
    title: &str,
    content: &str,
    timestamp_ns: i64,
    archived: bool,
) -> MemoryId {
    memory_with_created_at(
        cva,
        mutation_id,
        title,
        content,
        timestamp_ns,
        archived,
        999,
    )
}

pub(crate) fn memory_with_created_at(
    cva: &mut Cva,
    mutation_id: &str,
    title: &str,
    content: &str,
    timestamp_ns: i64,
    archived: bool,
    created_at_ns: i64,
) -> MemoryId {
    let node_id = format!("node-{mutation_id}");
    cva.append_node(
        node_id.clone(),
        "c1".into(),
        None,
        "user".into(),
        timestamp_ns,
        content,
    )
    .unwrap();
    let mut draft = draft(mutation_id, title, content, &node_id, archived);
    draft.created_at_ns = created_at_ns;
    draft.updated_at_ns = created_at_ns;
    cva.publish_memory(None, 0, draft).unwrap().0.id
}

pub(crate) fn install_vectors(
    cva: &mut Cva,
    ids: &[MemoryId],
    vectors: &[&[f32]],
) -> crate::CompatibilityProfileId {
    assert_eq!(ids.len(), vectors.len());
    let dimensions = u32::try_from(vectors[0].len()).unwrap();
    let endpoint = SimulatedEmbeddingEndpoint::new(dimensions, VectorNormalization::L2, 41);
    let profile = cva.establish_compatibility_profile(&endpoint).unwrap();
    let mut bytes = Vec::new();
    for vector in vectors {
        assert_eq!(vector.len(), dimensions as usize);
        for value in *vector {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    let schema = VectorSchema::new(dimensions, ScalarType::F32).unwrap();
    let packed_id = cva
        .put_packed_vectors(PackedVectors::from_bytes(schema, bytes).unwrap())
        .unwrap();
    let bodies = ids
        .iter()
        .map(|id| cva.memory_body_id(*id).unwrap())
        .collect();
    cva.put_memory_vectors(profile.id, packed_id, bodies)
        .unwrap();
    profile.id
}

fn draft(
    mutation_id: &str,
    title: &str,
    content: &str,
    node_id: &str,
    archived: bool,
) -> MemoryDraft {
    MemoryDraft {
        category: "project".into(),
        memory_type: "fact".into(),
        title: title.into(),
        content: content.into(),
        scope: "private".into(),
        lifecycle_state: if archived { "archived" } else { "knowledge" }.into(),
        archived,
        superseded_by: None,
        parent_id: None,
        source_node_id: None,
        content_source_conversation_id: Some("c1".into()),
        content_source_node_id: Some(node_id.into()),
        grounding_source_conversation_id: None,
        grounding_source_node_id: None,
        source_episode_id: None,
        mutation_id: mutation_id.into(),
        created_at_ns: 999,
        updated_at_ns: 999,
    }
}
