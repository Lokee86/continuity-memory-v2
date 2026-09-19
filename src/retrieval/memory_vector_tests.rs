use crate::archive_history_codec::encode_archive_format;
use crate::compatibility_profile_codec::encode_format as encode_profile_format;
use crate::insomnia::codec::encode_format as encode_insomnia_format;
use crate::memory_codec::encode_format as encode_memory_format;
use crate::packed_vector_codec::encode_format as encode_packed_format;
use crate::{
    Cva, CvaError, MemoryDraft, MemoryError, MemoryVectorError, SimulatedEmbeddingEndpoint,
    VectorNormalization,
};
use std::fs;
use std::path::PathBuf;

fn test_path(name: &str) -> PathBuf {
    let unique = uuid::Uuid::new_v4();
    let dir = std::env::temp_dir().join(format!("reliquary-memory-vectors-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn draft(mutation_id: &str, lifecycle: &str, title: &str, content: &str) -> MemoryDraft {
    MemoryDraft {
        category: "preference".into(),
        memory_type: "project".into(),
        authority_kind: "unknown".into(),
        temporal_status: "unknown".into(),
        title: title.into(),
        content: content.into(),
        scope: "private".into(),
        lifecycle_state: lifecycle.into(),
        archived: false,
        superseded_by: None,
        parent_id: None,
        source_node_id: None,
        content_source_conversation_id: None,
        content_source_node_id: None,
        grounding_source_conversation_id: None,
        grounding_source_node_id: None,
        source_episode_id: None,
        source_time_ns: None,
        mutation_id: mutation_id.into(),
        created_at_ns: 10,
        updated_at_ns: 10,
    }
}

#[test]
fn memory_body_is_embedded_once_per_profile_across_metadata_revisions() {
    let path = test_path("reuse.cva");
    let mut cva = Cva::create(&path).unwrap();
    let (memory, _) = cva
        .publish_memory(
            None,
            0,
            draft(
                "m1",
                "extracted",
                "Preferred editor",
                "The user prefers Helix.",
            ),
        )
        .unwrap();
    let body_id = cva.memory_body_id(memory.id).unwrap();
    let endpoint = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 7);
    let profile = cva.establish_compatibility_profile(&endpoint).unwrap();
    let global_before_vectors = cva.container.latest_version();
    let memory_version_before_vectors = cva.memory_version();

    let first = cva
        .build_missing_memory_vectors(profile.id, &endpoint)
        .unwrap();
    assert_eq!(cva.container.latest_version(), global_before_vectors);
    assert_eq!(cva.memory_version(), memory_version_before_vectors);
    assert_eq!(first.embedded, 1);
    assert_eq!(first.already_present, 0);
    let first_set_id = first.created_set.unwrap();
    assert!(cva.memory_vector_location(profile.id, body_id).is_some());
    let first_set = cva.memory_vectors(first_set_id).unwrap();
    assert_eq!(
        cva.put_memory_vectors(
            profile.id,
            first_set.packed_vector_id,
            first_set.memory_body_ids.clone(),
        )
        .unwrap(),
        first_set_id
    );
    let packed_after_first = cva.packed_vector_stats();

    let mut revised = draft(
        "m1-metadata",
        "reinforced",
        "Preferred editor",
        "The user prefers Helix.",
    );
    revised.updated_at_ns = 20;
    revised.category = "decision".into();
    cva.publish_memory(Some(memory.id), 1, revised).unwrap();
    assert_eq!(cva.memory_body_id(memory.id).unwrap(), body_id);

    let second = cva
        .build_missing_memory_vectors(profile.id, &endpoint)
        .unwrap();
    assert_eq!(second.embedded, 0);
    assert_eq!(second.already_present, 1);
    assert_eq!(second.created_set, None);
    assert_eq!(cva.packed_vector_stats(), packed_after_first);
}

#[test]
fn semantic_memory_content_cannot_mutate_in_place() {
    let path = test_path("immutable-body.cva");
    let mut cva = Cva::create(&path).unwrap();
    let (memory, _) = cva
        .publish_memory(None, 0, draft("m1", "extracted", "Rule", "Keep A."))
        .unwrap();
    let changed = draft("m1-changed", "reinforced", "Rule", "Keep B.");
    assert!(matches!(
        cva.publish_memory(Some(memory.id), 1, changed),
        Err(MemoryError::SemanticMutation)
    ));
}

#[test]
fn a_new_profile_gets_a_distinct_embedding_for_the_same_memory_body() {
    let path = test_path("profiles.cva");
    let mut cva = Cva::create(&path).unwrap();
    let (memory, _) = cva
        .publish_memory(None, 0, draft("m1", "extracted", "Rule", "Keep A."))
        .unwrap();
    let body_id = cva.memory_body_id(memory.id).unwrap();
    let endpoint_a = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 7);
    let endpoint_b = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 9001);
    let profile_a = cva.establish_compatibility_profile(&endpoint_a).unwrap();
    let profile_b = cva.establish_compatibility_profile(&endpoint_b).unwrap();
    assert_ne!(profile_a.id, profile_b.id);

    cva.build_missing_memory_vectors(profile_a.id, &endpoint_a)
        .unwrap();
    cva.build_missing_memory_vectors(profile_b.id, &endpoint_b)
        .unwrap();
    assert!(cva.memory_vector_location(profile_a.id, body_id).is_some());
    assert!(cva.memory_vector_location(profile_b.id, body_id).is_some());
    assert_eq!(cva.memory_vector_stats().bindings, 2);
}

#[test]
fn memory_vector_store_requires_its_format_marker() {
    let path = test_path("missing-format.cva");
    let mut container = crate::Container::create(&path).unwrap();
    container.append(&encode_archive_format()).unwrap();
    container.append(&encode_memory_format()).unwrap();
    container.append(&encode_insomnia_format()).unwrap();
    container.append(&encode_packed_format()).unwrap();
    container.append(&encode_profile_format()).unwrap();
    container.sync().unwrap();
    drop(container);
    assert!(matches!(
        Cva::open(path),
        Err(CvaError::MemoryVectors(MemoryVectorError::MissingFormat))
    ));
}

#[test]
fn memory_vector_bindings_reopen_with_the_cva() {
    let path = test_path("reopen.cva");
    let mut cva = Cva::create(&path).unwrap();
    let (memory, _) = cva
        .publish_memory(None, 0, draft("m1", "extracted", "Rule", "Keep A."))
        .unwrap();
    let body_id = cva.memory_body_id(memory.id).unwrap();
    let endpoint = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 7);
    let profile = cva.establish_compatibility_profile(&endpoint).unwrap();
    let result = cva
        .build_missing_memory_vectors(profile.id, &endpoint)
        .unwrap();
    let set_id = result.created_set.unwrap();
    cva.sync().unwrap();
    drop(cva);

    let mut reopened = Cva::open(&path).unwrap();
    assert_eq!(reopened.memory_vector_stats().bindings, 1);
    let location = reopened
        .memory_vector_location(profile.id, body_id)
        .unwrap();
    assert_eq!(location.set_id, set_id);
    let set = reopened.memory_vectors(set_id).unwrap();
    assert_eq!(set.memory_body_ids, vec![body_id]);
}
