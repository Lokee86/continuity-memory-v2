use crate::vector_generation_codec::{GenerationPayload, encode_generation};
use crate::vector_generation_validation::vector_generation_id;
use crate::{
    Cva, CvaError, EmbeddingEndpointDescriptor, FragmentConfig, PackedVectors, ScalarType,
    SimulatedEmbeddingEndpoint, VectorGenerationError, VectorNormalization, VectorSchema,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn test_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("continuity-generation-validation-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn endpoint() -> SimulatedEmbeddingEndpoint {
    SimulatedEmbeddingEndpoint::new(
        EmbeddingEndpointDescriptor {
            provider: "simulated".into(),
            model: "validation-test".into(),
            revision: "r1".into(),
            dimensions: 12,
            normalization: VectorNormalization::L2,
        },
        10,
    )
}

fn populate(cva: &mut Cva) {
    for index in 0..8 {
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
    cva.materialize_path_fragments("c1", "n7", FragmentConfig::default(), false)
        .unwrap();
}

#[test]
fn unversioned_generation_payload_is_inert_on_reopen() {
    let path = test_path("inert.cva");
    let mut cva = Cva::create(&path).unwrap();
    populate(&mut cva);
    let profile = cva.create_embedding_profile(&endpoint()).unwrap();
    let fragment_id = cva.fragments()[0].id;
    let schema = VectorSchema::new(profile.dimensions, ScalarType::F32).unwrap();
    let packed =
        PackedVectors::from_bytes(schema, vec![0; profile.dimensions as usize * 4]).unwrap();
    let packed_id = cva.put_packed_vectors(packed).unwrap();
    let archive_vector_id = cva
        .put_archive_vectors(packed_id, vec![fragment_id])
        .unwrap();
    let source_archive_version = cva.archive_version();
    let id = vector_generation_id(profile.id, archive_vector_id, source_archive_version);
    cva.container
        .append(&encode_generation(GenerationPayload {
            id,
            profile_id: profile.id,
            archive_vector_id,
            source_archive_version,
        }))
        .unwrap();
    cva.sync().unwrap();
    drop(cva);

    let reopened = Cva::open(path).unwrap();
    assert_eq!(reopened.vector_version(), 0);
    assert!(reopened.current_vector_generation(profile.id).is_none());
}

#[test]
fn publication_rejects_source_before_mapped_fragments() {
    let path = test_path("source-cut.cva");
    let mut cva = Cva::create(path).unwrap();
    populate(&mut cva);
    let profile = cva.create_embedding_profile(&endpoint()).unwrap();
    let fragment_id = cva.fragments()[0].id;
    let schema = VectorSchema::new(profile.dimensions, ScalarType::F32).unwrap();
    let packed =
        PackedVectors::from_bytes(schema, vec![0; profile.dimensions as usize * 4]).unwrap();
    let packed_id = cva.put_packed_vectors(packed).unwrap();
    let archive_vector_id = cva
        .put_archive_vectors(packed_id, vec![fragment_id])
        .unwrap();
    assert!(matches!(
        cva.publish_vector_generation(profile.id, archive_vector_id, cva.archive_version() - 1,),
        Err(VectorGenerationError::SourceArchiveVersion)
    ));
}

#[test]
fn reopen_rejects_global_version_claimed_by_archive_and_vectors() {
    let path = test_path("global-conflict.cva");
    let mut cva = Cva::create(&path).unwrap();
    populate(&mut cva);
    let archive_global = cva.record_versions().last().unwrap().global_version;
    let profile = cva.create_embedding_profile(&endpoint()).unwrap();
    cva.build_archive_vector_generation(profile.id, &endpoint())
        .unwrap();
    cva.sync().unwrap();
    drop(cva);

    let mut bytes = fs::read(&path).unwrap();
    let record = bytes
        .windows(8)
        .position(|window| window == b"CVAVGRC1")
        .unwrap();
    bytes[record + 8..record + 16].copy_from_slice(&archive_global.to_le_bytes());
    fs::write(&path, bytes).unwrap();

    assert!(matches!(
        Cva::open(path),
        Err(CvaError::SemanticGlobalVersionConflict(version)) if version == archive_global
    ));
}

#[test]
fn publication_rejects_profile_matrix_dimension_mismatch() {
    let path = test_path("dimensions.cva");
    let mut cva = Cva::create(path).unwrap();
    populate(&mut cva);
    let profile = cva.create_embedding_profile(&endpoint()).unwrap();
    let fragment_id = cva.fragments()[0].id;
    let schema = VectorSchema::new(4, ScalarType::F32).unwrap();
    let packed = PackedVectors::from_bytes(schema, vec![0; 16]).unwrap();
    let packed_id = cva.put_packed_vectors(packed).unwrap();
    let archive_vector_id = cva
        .put_archive_vectors(packed_id, vec![fragment_id])
        .unwrap();
    assert!(matches!(
        cva.publish_vector_generation(profile.id, archive_vector_id, cva.archive_version()),
        Err(VectorGenerationError::DimensionMismatch)
    ));
}
