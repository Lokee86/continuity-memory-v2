use crate::memory_codec::{decode_record, encode_record};
use crate::memory_model::MemoryRecord;
use crate::{MemoryBodyId, MemoryId};

#[test]
fn memory_record_v3_round_trips_authority_kind() {
    let record = record("direct");
    let decoded = decode_record(&encode_record(&record).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(decoded.authority_kind, "direct");
    assert_eq!(decoded.category, record.category);
    assert_eq!(decoded.memory_type, record.memory_type);
}

#[test]
fn legacy_memory_record_v2_reopens_with_unknown_authority() {
    let record = record("direct");
    let mut bytes = encode_record(&record).unwrap();
    bytes[..8].copy_from_slice(b"CVAMEMR2");

    let mut cursor = 100;
    skip_string(&bytes, &mut cursor);
    skip_string(&bytes, &mut cursor);
    let authority_start = cursor;
    skip_string(&bytes, &mut cursor);
    bytes.drain(authority_start..cursor);

    let decoded = decode_record(&bytes).unwrap().unwrap();
    assert_eq!(decoded.authority_kind, "unknown");
    assert_eq!(decoded.category, record.category);
    assert_eq!(decoded.memory_type, record.memory_type);
    assert_eq!(decoded.mutation_id, record.mutation_id);
}

fn skip_string(bytes: &[u8], cursor: &mut usize) {
    let len = u32::from_le_bytes(bytes[*cursor..*cursor + 4].try_into().unwrap()) as usize;
    *cursor += 4 + len;
}

fn record(authority_kind: &str) -> MemoryRecord {
    MemoryRecord {
        id: MemoryId([1; 32]),
        revision: 1,
        body_id: MemoryBodyId([2; 32]),
        category: "decision".into(),
        memory_type: "project".into(),
        authority_kind: authority_kind.into(),
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
        mutation_id: "codec-test".into(),
        created_at_ns: 1,
        updated_at_ns: 2,
        global_version: 3,
        memory_version: 4,
    }
}
