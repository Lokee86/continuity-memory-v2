use crate::{Cva, FragmentConfig, MemoryDraft, SimulatedEmbeddingEndpoint, VectorNormalization};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn test_dir() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("continuity-cva-derived-merge-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn append_conversation(cva: &mut Cva, conversation: &str, text: &str) {
    for index in 0..8 {
        cva.append_node(
            format!("{conversation}-{index}"),
            conversation.into(),
            (index > 0).then(|| format!("{conversation}-{}", index - 1)),
            if index % 2 == 0 { "user" } else { "assistant" }.into(),
            index as i64,
            text,
        )
        .unwrap();
    }
    cva.materialize_path_fragments(
        conversation,
        &format!("{conversation}-7"),
        FragmentConfig::default(),
        false,
    )
    .unwrap();
}

fn memory() -> MemoryDraft {
    MemoryDraft {
        category: "fact".into(),
        memory_type: "project".into(),
        authority_kind: "unknown".into(),
        title: "Common memory".into(),
        content: "This memory has a derived vector.".into(),
        scope: "private".into(),
        lifecycle_state: "extracted".into(),
        archived: false,
        superseded_by: None,
        parent_id: None,
        source_node_id: None,
        content_source_conversation_id: None,
        content_source_node_id: None,
        grounding_source_conversation_id: None,
        grounding_source_node_id: None,
        source_episode_id: None,
        mutation_id: "common-memory".into(),
        created_at_ns: 1,
        updated_at_ns: 1,
    }
}

#[test]
fn reconcile_repacks_fragments_and_retires_stale_vector_state() {
    let dir = test_dir();
    let left = dir.join("left.cva");
    let right = dir.join("right.cva");
    let output = dir.join("merged.cva");
    let mut base = Cva::create_project(&left).unwrap();
    append_conversation(&mut base, "common", "commonunique");
    let endpoint = SimulatedEmbeddingEndpoint::new(12, VectorNormalization::L2, 77);
    let profile = base.establish_compatibility_profile(&endpoint).unwrap();
    base.publish_memory(None, 0, memory()).unwrap();
    base.build_missing_memory_vectors(profile.id, &endpoint)
        .unwrap();
    base.build_archive_vector_generation(profile.id, &endpoint)
        .unwrap();
    assert!(base.packed_vector_stats().objects > 0);
    assert!(base.memory_vector_stats().objects > 0);
    assert!(base.archive_vector_stats().objects > 0);
    assert!(base.vector_generation_stats().generations > 0);
    base.sync().unwrap();
    drop(base);
    fs::copy(&left, &right).unwrap();

    let mut left_cva = Cva::open(&left).unwrap();
    append_conversation(&mut left_cva, "left", "leftunique");
    left_cva.sync().unwrap();
    let mut right_cva = Cva::open(&right).unwrap();
    append_conversation(&mut right_cva, "right", "rightunique");
    let right_endpoint = SimulatedEmbeddingEndpoint::new(12, VectorNormalization::L2, 88);
    let right_profile = right_cva
        .establish_compatibility_profile(&right_endpoint)
        .unwrap();
    assert_ne!(right_profile.id, profile.id);
    right_cva.sync().unwrap();
    drop(left_cva);
    drop(right_cva);

    let result = Cva::reconcile(&left, &right, &output).unwrap();
    assert!(result.vector_rebuild_required);
    let mut merged = Cva::open(&output).unwrap();
    assert_eq!(merged.fragments().len(), 3);
    assert_eq!(merged.compatibility_profile(profile.id).unwrap(), profile);
    assert_eq!(
        merged.compatibility_profile(right_profile.id).unwrap(),
        right_profile
    );
    assert_eq!(merged.packed_vector_stats().objects, 0);
    assert_eq!(merged.memory_vector_stats().objects, 0);
    assert_eq!(merged.archive_vector_stats().objects, 0);
    assert_eq!(merged.vector_generation_stats().generations, 0);
    assert!(merged.current_vector_generation(profile.id).is_none());
    assert_eq!(
        merged.lexical_candidates("leftunique", 10).unwrap().len(),
        1
    );
    assert_eq!(
        merged.lexical_candidates("rightunique", 10).unwrap().len(),
        1
    );

    let recovery = merged.rebuild_derived_vectors(&endpoint).unwrap();
    assert_eq!(recovery.compatibility_profile_id, profile.id);
    assert!(recovery.memory_vectors_embedded > 0);
    assert!(recovery.archive_generation_rebuilt);
    merged.sync().unwrap();
    drop(merged);

    let reopened = Cva::open(output).unwrap();
    assert!(reopened.packed_vector_stats().objects > 0);
    assert!(reopened.memory_vector_stats().objects > 0);
    assert!(reopened.archive_vector_stats().objects > 0);
    let generation = reopened.current_vector_generation(profile.id).unwrap();
    assert_eq!(
        generation.source_archive_version,
        reopened.archive_version()
    );
}
