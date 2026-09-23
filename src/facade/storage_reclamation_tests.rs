use crate::storage_reclamation::relocate_payload;
use crate::{Cva, EntityDraft, ObjectRef, PackedVectors, Phylactery, ScalarType, VectorSchema};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn test_path(name: &str) -> PathBuf {
    let unique = uuid::Uuid::new_v4();
    let dir = std::env::temp_dir().join(format!("continuity-storage-reclaim-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

#[test]
fn rel_reclamation_drops_proven_garbage_and_preserves_published_state() {
    let source = test_path("source.rel");
    let output = source.with_file_name("reclaimed.rel");
    let mut rel = Cva::create(&source).unwrap();
    rel.append_node(
        "published-node".into(),
        "published-conversation".into(),
        None,
        "user".into(),
        1,
        "published content survives reclamation",
    )
    .unwrap();

    let orphan = b"unpublished archive content";
    let orphan_id = crate::archive_store::hash_content(orphan);
    rel.container
        .append(&crate::archive_codec::encode_content(orphan_id, orphan).unwrap())
        .unwrap();
    rel.container
        .append(
            &crate::archive_codec::encode_branch(&crate::Branch {
                id: "staged-only".into(),
                conversation_id: "never-published".into(),
                leaf_node_id: "missing".into(),
                canonical: false,
            })
            .unwrap(),
        )
        .unwrap();
    rel.container
        .append(&crate::conversation_compaction_codec::free_prefix())
        .unwrap();

    let (entity, changed) = rel
        .publish_entity(
            None,
            0,
            EntityDraft {
                canonical_name: "Example".into(),
                aliases: Vec::new(),
                kind: "test".into(),
                summary: "survives physical relocation".into(),
                mutation_id: "entity-reclaim-test".into(),
                created_at_ns: 1,
                updated_at_ns: 1,
            },
        )
        .unwrap();
    assert!(changed);

    let schema = VectorSchema::new(2, ScalarType::F32).unwrap();
    rel.put_packed_vectors(PackedVectors::from_bytes(schema, vec![0_u8; 8]).unwrap())
        .unwrap();
    rel.sync().unwrap();
    let before = fs::metadata(&source).unwrap().len();

    let report = rel.reclaim_storage(&output).unwrap();
    assert!(report.reclaimed_chunks >= 4);
    assert_eq!(report.unpublished_backing_chunks, 1);
    assert_eq!(report.orphan_content_chunks, 1);
    assert_eq!(report.free_compaction_chunks, 1);
    assert_eq!(report.unreferenced_packed_vector_chunks, 1);
    assert!(report.output_file_bytes < before);

    let reopened = Cva::open(&output).unwrap();
    assert_eq!(reopened.stats().nodes, 1);
    assert_eq!(reopened.entity(entity.id).unwrap(), entity);
    assert_eq!(reopened.packed_vector_stats().objects, 0);
}

#[test]
fn phy_reclamation_preserves_identity_while_dropping_unused_matrix() {
    let source = test_path("source.phy");
    let output = source.with_file_name("reclaimed.phy");
    let mut phy = Phylactery::create(&source).unwrap();
    let owner = phy.owner_uuid();
    let schema = VectorSchema::new(2, ScalarType::F32).unwrap();
    phy.put_packed_vectors(PackedVectors::from_bytes(schema, vec![0_u8; 8]).unwrap())
        .unwrap();

    let report = phy.reclaim_storage(&output).unwrap();
    assert_eq!(report.unreferenced_packed_vector_chunks, 1);

    let reopened = Phylactery::open(&output).unwrap();
    assert_eq!(reopened.owner_uuid(), owner);
    assert_eq!(reopened.packed_vector_stats().objects, 0);
}

#[test]
fn relocation_covers_entity_version_records() {
    fn object(offset: u64, len: u64) -> ObjectRef {
        let mut bytes = [0_u8; 16];
        bytes[..8].copy_from_slice(&offset.to_le_bytes());
        bytes[8..].copy_from_slice(&len.to_le_bytes());
        ObjectRef::from_legacy_bytes(bytes)
    }

    let old = object(100, 24);
    let new = object(80, 24);
    let relocated = BTreeMap::from([(old, new)]);
    let entity = crate::entity_codec::EntityVersion {
        global_version: 3,
        entity_version: 2,
        record: old,
    };
    let entity =
        relocate_payload(&crate::entity_codec::encode_version(entity), &relocated).unwrap();
    assert_eq!(
        crate::entity_codec::decode_version(&entity)
            .unwrap()
            .unwrap()
            .record,
        new
    );
}
