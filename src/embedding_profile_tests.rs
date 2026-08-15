use crate::{
    Cva, EmbeddingEndpointDescriptor, EmbeddingProfileError, SimulatedEmbeddingEndpoint,
    VectorNormalization,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn test_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("continuity-profiles-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn endpoint(seed: u64) -> SimulatedEmbeddingEndpoint {
    SimulatedEmbeddingEndpoint::new(
        EmbeddingEndpointDescriptor {
            provider: "simulated".into(),
            model: "test-embed".into(),
            revision: "r1".into(),
            dimensions: 16,
            normalization: VectorNormalization::L2,
        },
        seed,
    )
}

#[test]
fn profile_round_trips_and_is_clock_neutral() {
    let path = test_path("roundtrip.cva");
    let mut cva = Cva::create(&path).unwrap();
    let global_before = cva.container.latest_version();
    let profile = cva.create_embedding_profile(&endpoint(7)).unwrap();
    assert_eq!(cva.container.latest_version(), global_before);
    assert_eq!(cva.embedding_profile_stats().profiles, 1);
    cva.sync().unwrap();
    drop(cva);

    let reopened = Cva::open(path).unwrap();
    assert_eq!(reopened.embedding_profile(profile.id).unwrap(), profile);
}

#[test]
fn identical_endpoint_deduplicates_profile() {
    let path = test_path("dedupe.cva");
    let mut cva = Cva::create(path).unwrap();
    let first = cva.create_embedding_profile(&endpoint(9)).unwrap();
    let second = cva.create_embedding_profile(&endpoint(9)).unwrap();
    assert_eq!(first.id, second.id);
    assert_eq!(cva.embedding_profile_stats().profiles, 1);
}

#[test]
fn behavior_fingerprint_distinguishes_same_advertised_model() {
    let path = test_path("fingerprint.cva");
    let mut cva = Cva::create(path).unwrap();
    let first = cva.create_embedding_profile(&endpoint(1)).unwrap();
    let second = cva.create_embedding_profile(&endpoint(2)).unwrap();
    assert_ne!(first.id, second.id);
    assert_ne!(first.behavior_fingerprint, second.behavior_fingerprint);
    assert_eq!(cva.embedding_profile_stats().profiles, 2);
}

#[test]
fn endpoint_verification_rejects_behavior_change() {
    let path = test_path("verify.cva");
    let mut cva = Cva::create(path).unwrap();
    let profile = cva.create_embedding_profile(&endpoint(11)).unwrap();
    cva.verify_embedding_endpoint(profile.id, &endpoint(11))
        .unwrap();
    assert!(matches!(
        cva.verify_embedding_endpoint(profile.id, &endpoint(12)),
        Err(EmbeddingProfileError::EndpointMismatch)
    ));
}
