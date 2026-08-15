use crate::{
    Cva, EmbeddingEndpointDescriptor, EmbeddingProfileError, FragmentConfig,
    SimulatedEmbeddingEndpoint, VectorGenerationError, VectorNormalization,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn test_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("continuity-generations-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn endpoint(seed: u64) -> SimulatedEmbeddingEndpoint {
    SimulatedEmbeddingEndpoint::new(
        EmbeddingEndpointDescriptor {
            provider: "simulated".into(),
            model: "archive-test".into(),
            revision: "r1".into(),
            dimensions: 12,
            normalization: VectorNormalization::L2,
        },
        seed,
    )
}

fn append_turns(cva: &mut Cva, start: usize, end: usize) {
    for index in start..end {
        cva.append_node(
            format!("n{index}"),
            "c1".into(),
            (index > 0).then(|| format!("n{}", index - 1)),
            if index % 2 == 0 { "user" } else { "assistant" }.into(),
            index as i64,
            &format!("turn {index}"),
        )
        .unwrap();
    }
}

fn materialize(cva: &mut Cva, leaf: usize) {
    cva.materialize_path_fragments("c1", &format!("n{leaf}"), FragmentConfig::default(), false)
        .unwrap();
}

#[test]
fn simulated_endpoint_builds_and_reopens_active_generation() {
    let path = test_path("roundtrip.cva");
    let mut cva = Cva::create(&path).unwrap();
    append_turns(&mut cva, 0, 8);
    materialize(&mut cva, 7);
    let source = cva.archive_version();
    let profile = cva.create_embedding_profile(&endpoint(1)).unwrap();
    let generation = cva
        .build_archive_vector_generation(profile.id, &endpoint(1))
        .unwrap();
    assert_eq!(generation.source_archive_version, source);
    assert_eq!(generation.vector_version, 1);
    assert_eq!(cva.vector_version(), 1);
    assert_eq!(cva.packed_vector_stats().objects, 1);
    assert_eq!(cva.archive_vector_stats().objects, 1);
    assert_eq!(cva.current_vector_generation(profile.id), Some(generation));
    assert!(generation.global_version > cva.record_versions().last().unwrap().global_version);
    cva.append_node(
        "n8".into(),
        "c1".into(),
        Some("n7".into()),
        "user".into(),
        8,
        "post-generation turn",
    )
    .unwrap();
    assert!(cva.record_versions().last().unwrap().global_version > generation.global_version);
    assert_eq!(cva.archive_version(), source + 1);
    assert_eq!(cva.vector_version(), 1);
    cva.sync().unwrap();
    drop(cva);

    let reopened = Cva::open(path).unwrap();
    assert_eq!(
        reopened.current_vector_generation(profile.id),
        Some(generation)
    );
    assert_eq!(
        reopened.vector_generation(generation.id).unwrap(),
        generation
    );
}

#[test]
fn repeated_build_is_idempotent() {
    let path = test_path("idempotent.cva");
    let mut cva = Cva::create(path).unwrap();
    append_turns(&mut cva, 0, 8);
    materialize(&mut cva, 7);
    let profile = cva.create_embedding_profile(&endpoint(2)).unwrap();
    let first = cva
        .build_archive_vector_generation(profile.id, &endpoint(2))
        .unwrap();
    let second = cva
        .build_archive_vector_generation(profile.id, &endpoint(2))
        .unwrap();
    assert_eq!(first, second);
    assert_eq!(cva.vector_version(), 1);
    assert_eq!(cva.vector_generation_stats().generations, 1);
}

#[test]
fn newer_archive_cut_supersedes_profile_generation() {
    let path = test_path("supersede.cva");
    let mut cva = Cva::create(path).unwrap();
    append_turns(&mut cva, 0, 8);
    materialize(&mut cva, 7);
    let profile = cva.create_embedding_profile(&endpoint(3)).unwrap();
    let first = cva
        .build_archive_vector_generation(profile.id, &endpoint(3))
        .unwrap();

    append_turns(&mut cva, 8, 14);
    materialize(&mut cva, 13);
    let second = cva
        .build_archive_vector_generation(profile.id, &endpoint(3))
        .unwrap();
    assert!(second.source_archive_version > first.source_archive_version);
    assert_eq!(second.vector_version, 2);
    assert_eq!(cva.current_vector_generation(profile.id), Some(second));
    assert_eq!(cva.vector_generation_at(profile.id, 1), Some(first));
}

#[test]
fn profiles_keep_independent_active_generations() {
    let path = test_path("profiles.cva");
    let mut cva = Cva::create(path).unwrap();
    append_turns(&mut cva, 0, 8);
    materialize(&mut cva, 7);
    let first_profile = cva.create_embedding_profile(&endpoint(4)).unwrap();
    let second_profile = cva.create_embedding_profile(&endpoint(5)).unwrap();
    let first = cva
        .build_archive_vector_generation(first_profile.id, &endpoint(4))
        .unwrap();
    let second = cva
        .build_archive_vector_generation(second_profile.id, &endpoint(5))
        .unwrap();
    assert_ne!(first.archive_vector_id, second.archive_vector_id);
    assert_eq!(cva.current_vector_generation(first_profile.id), Some(first));
    assert_eq!(
        cva.current_vector_generation(second_profile.id),
        Some(second)
    );
    assert_eq!(cva.vector_generation_stats().active_profiles, 2);
}

#[test]
fn build_rejects_endpoint_that_no_longer_matches_profile() {
    let path = test_path("mismatch.cva");
    let mut cva = Cva::create(path).unwrap();
    append_turns(&mut cva, 0, 8);
    materialize(&mut cva, 7);
    let profile = cva.create_embedding_profile(&endpoint(6)).unwrap();
    assert!(matches!(
        cva.build_archive_vector_generation(profile.id, &endpoint(7)),
        Err(VectorGenerationError::Profile(
            EmbeddingProfileError::EndpointMismatch
        ))
    ));
    assert_eq!(cva.vector_version(), 0);
}
