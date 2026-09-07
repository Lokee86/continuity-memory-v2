use crate::{
    Cva, FragmentConfig, SimulatedEmbeddingEndpoint, VectorGenerationError, VectorNormalization,
};
use std::fs;
use std::path::PathBuf;

fn test_path(name: &str) -> PathBuf {
    let unique = uuid::Uuid::new_v4();
    let dir = std::env::temp_dir().join(format!("continuity-generations-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn endpoint(seed: u64) -> SimulatedEmbeddingEndpoint {
    SimulatedEmbeddingEndpoint::new(12, VectorNormalization::L2, seed)
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
    let profile = cva.establish_compatibility_profile(&endpoint(1)).unwrap();
    let generation = cva
        .build_archive_vector_generation(profile.id, &endpoint(1))
        .unwrap();
    assert_eq!(generation.compatibility_profile_id, profile.id);
    assert_eq!(generation.source_archive_version, source);
    assert_eq!(generation.vector_version, 1);
    assert_eq!(cva.current_vector_generation(profile.id), Some(generation));
    assert!(generation.global_version > cva.record_versions().last().unwrap().global_version);
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
    let profile = cva.establish_compatibility_profile(&endpoint(2)).unwrap();
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
fn newer_archive_cut_supersedes_compatibility_profile_generation() {
    let path = test_path("supersede.cva");
    let mut cva = Cva::create(path).unwrap();
    append_turns(&mut cva, 0, 8);
    materialize(&mut cva, 7);
    let profile = cva.establish_compatibility_profile(&endpoint(3)).unwrap();
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
fn compatibility_profiles_keep_independent_active_generations() {
    let path = test_path("profiles.cva");
    let mut cva = Cva::create(path).unwrap();
    append_turns(&mut cva, 0, 8);
    materialize(&mut cva, 7);
    let first_endpoint = endpoint(40);
    let second_endpoint = endpoint(50);
    let first_profile = cva
        .establish_compatibility_profile(&first_endpoint)
        .unwrap();
    let second_profile = cva
        .establish_compatibility_profile(&second_endpoint)
        .unwrap();
    let first = cva
        .build_archive_vector_generation(first_profile.id, &first_endpoint)
        .unwrap();
    let second = cva
        .build_archive_vector_generation(second_profile.id, &second_endpoint)
        .unwrap();
    assert_ne!(
        first.compatibility_profile_id,
        second.compatibility_profile_id
    );
    assert_eq!(cva.current_vector_generation(first_profile.id), Some(first));
    assert_eq!(
        cva.current_vector_generation(second_profile.id),
        Some(second)
    );
    assert_eq!(cva.vector_generation_stats().active_profiles, 2);
}

#[test]
fn compatible_endpoint_drift_can_build_under_existing_profile() {
    let path = test_path("compatible-drift.cva");
    let mut cva = Cva::create(path).unwrap();
    append_turns(&mut cva, 0, 8);
    materialize(&mut cva, 7);
    let profile = cva.establish_compatibility_profile(&endpoint(4)).unwrap();
    let drifted = endpoint(4).with_drift(0.00001);
    let generation = cva
        .build_archive_vector_generation(profile.id, &drifted)
        .unwrap();
    assert_eq!(generation.compatibility_profile_id, profile.id);
}

#[test]
fn incompatible_endpoint_cannot_build_under_profile() {
    let path = test_path("mismatch.cva");
    let mut cva = Cva::create(path).unwrap();
    append_turns(&mut cva, 0, 8);
    materialize(&mut cva, 7);
    let profile = cva.establish_compatibility_profile(&endpoint(4)).unwrap();
    assert!(matches!(
        cva.build_archive_vector_generation(profile.id, &endpoint(5)),
        Err(VectorGenerationError::IncompatibleEndpoint)
    ));
    assert_eq!(cva.vector_version(), 0);
}
