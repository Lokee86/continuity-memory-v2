use crate::{COMPATIBILITY_MIN_COSINE, Cva, SimulatedEmbeddingEndpoint, VectorNormalization};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn test_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("continuity-compatibility-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn endpoint(seed: u64) -> SimulatedEmbeddingEndpoint {
    SimulatedEmbeddingEndpoint::new(16, VectorNormalization::L2, seed)
}

#[test]
fn profile_round_trips_and_is_clock_neutral() {
    let path = test_path("roundtrip.cva");
    let mut cva = Cva::create(&path).unwrap();
    cva.append_node(
        "n0".into(),
        "c1".into(),
        None,
        "user".into(),
        0,
        "before profile",
    )
    .unwrap();
    let before_global = cva.record_versions().last().unwrap().global_version;
    let profile = cva.establish_compatibility_profile(&endpoint(1)).unwrap();
    assert_eq!(cva.compatibility_profile_stats().profiles, 1);
    assert_eq!(cva.archive_version(), 1);
    assert_eq!(cva.vector_version(), 0);
    cva.append_node(
        "n1".into(),
        "c1".into(),
        Some("n0".into()),
        "assistant".into(),
        1,
        "after profile",
    )
    .unwrap();
    assert_eq!(
        cva.record_versions().last().unwrap().global_version,
        before_global + 1
    );
    cva.sync().unwrap();
    drop(cva);

    let reopened = Cva::open(path).unwrap();
    assert_eq!(reopened.compatibility_profile(profile.id).unwrap(), profile);
}

#[test]
fn tiny_endpoint_drift_is_compatible_and_reuses_profile() {
    let path = test_path("drift.cva");
    let mut cva = Cva::create(path).unwrap();
    let profile = cva.establish_compatibility_profile(&endpoint(2)).unwrap();
    let drifted = endpoint(2).with_drift(0.00001);
    let report = cva
        .verify_compatibility_endpoint(profile.id, &drifted)
        .unwrap();
    assert!(report.compatible);
    assert!(report.minimum_cosine.unwrap() >= COMPATIBILITY_MIN_COSINE);
    let reused = cva.establish_compatibility_profile(&drifted).unwrap();
    assert_eq!(reused.id, profile.id);
    assert_eq!(cva.compatibility_profile_stats().profiles, 1);
}

#[test]
fn materially_different_endpoint_gets_a_distinct_profile() {
    let path = test_path("distinct.cva");
    let mut cva = Cva::create(path).unwrap();
    let first = cva.establish_compatibility_profile(&endpoint(3)).unwrap();
    let second_endpoint = endpoint(4);
    let report = cva
        .verify_compatibility_endpoint(first.id, &second_endpoint)
        .unwrap();
    assert!(!report.compatible);
    let second = cva
        .establish_compatibility_profile(&second_endpoint)
        .unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(cva.compatibility_profile_stats().profiles, 2);
}

#[test]
fn dimensions_are_part_of_compatibility() {
    let path = test_path("dimensions.cva");
    let mut cva = Cva::create(path).unwrap();
    let profile = cva.establish_compatibility_profile(&endpoint(5)).unwrap();
    let other = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 5);
    let report = cva
        .verify_compatibility_endpoint(profile.id, &other)
        .unwrap();
    assert!(!report.compatible);
    assert_eq!(report.minimum_cosine, None);
}
