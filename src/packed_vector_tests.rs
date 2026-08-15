use crate::archive_history_codec::encode_archive_format;
use crate::{
    Branch, Container, Cva, CvaError, PackedVectorError, PackedVectors, ScalarType, VectorSchema,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn test_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("continuity-packed-vectors-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

#[test]
fn packed_vectors_round_trip_inside_same_cva_as_archive() {
    let path = test_path("roundtrip.cva");
    let mut cva = Cva::create(&path).unwrap();
    cva.append_node("n1".into(), "c1".into(), None, "user".into(), 1, "hello")
        .unwrap();
    cva.append_branch(Branch {
        id: "main".into(),
        conversation_id: "c1".into(),
        leaf_node_id: "n1".into(),
        canonical: true,
    })
    .unwrap();

    let schema = VectorSchema::new(3, ScalarType::F16).unwrap();
    let packed =
        PackedVectors::from_bytes(schema, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]).unwrap();
    let id = cva.put_packed_vectors(packed.clone()).unwrap();
    assert_eq!(cva.archive_version(), 2);
    assert_eq!(cva.packed_vector_stats().objects, 1);
    assert_eq!(cva.packed_vector_stats().rows, 2);
    cva.sync().unwrap();
    drop(cva);

    let mut reopened = Cva::open(&path).unwrap();
    assert_eq!(reopened.stats().nodes, 1);
    assert_eq!(reopened.stats().branches, 1);
    assert_eq!(reopened.packed_vector_stats().objects, 1);
    assert_eq!(reopened.packed_vectors(id).unwrap(), packed);
}

#[test]
fn identical_packed_matrix_is_content_addressed_once() {
    let path = test_path("dedupe.cva");
    let mut cva = Cva::create(path).unwrap();
    let schema = VectorSchema::new(4, ScalarType::I8).unwrap();
    let packed = PackedVectors::from_bytes(schema, vec![1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
    let first = cva.put_packed_vectors(packed.clone()).unwrap();
    let second = cva.put_packed_vectors(packed).unwrap();
    assert_eq!(first, second);
    assert_eq!(cva.packed_vector_stats().objects, 1);
}

#[test]
fn raw_packed_vectors_do_not_advance_semantic_clocks() {
    let path = test_path("clocks.cva");
    let mut cva = Cva::create(path).unwrap();
    let global_before = cva.container.latest_version();
    let archive_before = cva.archive_version();
    let schema = VectorSchema::new(512, ScalarType::I8).unwrap();
    let packed = PackedVectors::from_bytes(schema, vec![0; 512]).unwrap();
    cva.put_packed_vectors(packed).unwrap();
    assert_eq!(cva.container.latest_version(), global_before);
    assert_eq!(cva.archive_version(), archive_before);
}

#[test]
fn supports_large_float64_rows_without_special_cases() {
    let path = test_path("float64.cva");
    let mut cva = Cva::create(path).unwrap();
    let schema = VectorSchema::new(4096, ScalarType::F64).unwrap();
    let packed = PackedVectors::from_bytes(schema, vec![0; 4096 * 8]).unwrap();
    let id = cva.put_packed_vectors(packed).unwrap();
    let info = cva
        .packed_vector_infos()
        .into_iter()
        .find(|info| info.id == id)
        .unwrap();
    assert_eq!(info.schema, schema);
    assert_eq!(info.count, 1);
    assert_eq!(info.byte_len, 32_768);
}

#[test]
fn packed_vector_store_requires_its_format_marker() {
    let path = test_path("missing-format.cva");
    let mut container = Container::create(&path).unwrap();
    container.append(&encode_archive_format()).unwrap();
    container.sync().unwrap();
    drop(container);
    assert!(matches!(
        Cva::open(path),
        Err(CvaError::PackedVectors(PackedVectorError::MissingFormat))
    ));
}

#[test]
fn corrupt_packed_vector_bytes_are_rejected_on_reopen() {
    let path = test_path("corrupt.cva");
    let mut cva = Cva::create(&path).unwrap();
    let schema = VectorSchema::new(4, ScalarType::I8).unwrap();
    cva.put_packed_vectors(PackedVectors::from_bytes(schema, vec![1, 2, 3, 4]).unwrap())
        .unwrap();
    cva.sync().unwrap();
    drop(cva);

    let mut bytes = fs::read(&path).unwrap();
    let magic = b"CVAPVEC1";
    let object = bytes
        .windows(magic.len())
        .position(|window| window == magic)
        .unwrap();
    let matrix = object + 64;
    bytes[matrix] ^= 0xff;
    fs::write(&path, bytes).unwrap();

    assert!(matches!(
        Cva::open(path),
        Err(CvaError::PackedVectors(PackedVectorError::HashCollision))
    ));
}
