use crate::memory_codec::{decode_record, encode_record};
use crate::memory_model::MemoryRecord;
use crate::{
    CHRONOS_INFERENCE_CONTRACT_VERSION, EpisodeId, MemoryBodyId, MemoryId, MemorySourceRef,
    MemoryTemporalInference, TemporalIndicationKind, TemporalInference,
    TemporalInferenceResolution,
};

#[test]
fn memory_record_v8_round_trips_source_reference_principal_and_temporal_status() {
    let mut record = record("adoption");
    record.source_time_ns = Some(42);
    record.source_ref = Some(MemorySourceRef {
        owner_id: "rel-00000000-0000-0000-0000-000000000001".into(),
        principal_id: Some("phy-00000000-0000-0000-0000-000000000002".into()),
        source_episode_id: EpisodeId([7; 32]),
        source_node_id: "u1".into(),
        content_source_conversation_id: Some("c1".into()),
        content_source_node_id: Some("a0".into()),
        grounding_source_conversation_id: None,
        grounding_source_node_id: None,
    });
    record.temporal_inference = Some(MemoryTemporalInference {
        body_id: record.body_id,
        source_time_ns: record.source_time_ns,
        inference: TemporalInference {
            model: "chronos-test".into(),
            contract_version: CHRONOS_INFERENCE_CONTRACT_VERSION.into(),
            resolutions: vec![TemporalInferenceResolution {
                start_byte: 0,
                end_byte: 8,
                kind: TemporalIndicationKind::Recurrence,
                evidence: "biweekly".into(),
                canonical_expression: "every two weeks".into(),
            }],
        },
    });
    let decoded = decode_record(&encode_record(&record).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(decoded.authority_kind, "adoption");
    assert_eq!(decoded.source_time_ns, Some(42));
    assert_eq!(decoded.source_ref, record.source_ref);
    assert_eq!(decoded.temporal_status, "current");
    assert_eq!(decoded.temporal_inference, record.temporal_inference);
    assert_eq!(decoded.category, record.category);
    assert_eq!(decoded.memory_type, record.memory_type);
}

#[test]
fn legacy_memory_record_v7_source_reference_reopens_without_principal() {
    let mut record = record("direct");
    record.source_ref = Some(MemorySourceRef {
        owner_id: "rel-legacy".into(),
        principal_id: None,
        source_episode_id: EpisodeId([6; 32]),
        source_node_id: "u1".into(),
        content_source_conversation_id: None,
        content_source_node_id: None,
        grounding_source_conversation_id: None,
        grounding_source_node_id: None,
    });
    let mut bytes = encode_record(&record).unwrap();
    let mut cursor = 81;
    cursor += 1; // superseded_by = None
    cursor += 1; // parent_id = None
    cursor += 1; // source_episode_id = None
    cursor += 1; // source_time_ns = None
    assert_eq!(bytes[cursor], 1); // source_ref = Some
    cursor += 1;
    skip_string(&bytes, &mut cursor); // source_ref.owner_id
    assert_eq!(bytes[cursor], 0); // V8 principal_id = None
    bytes.remove(cursor);
    bytes[..8].copy_from_slice(b"CVAMEMR7");

    let decoded = decode_record(&bytes).unwrap().unwrap();
    let source_ref = decoded.source_ref.unwrap();
    assert_eq!(source_ref.owner_id, "rel-legacy");
    assert_eq!(source_ref.principal_id, None);
    assert_eq!(source_ref.source_node_id, "u1");
}

#[test]
fn legacy_memory_record_v5_reopens_with_unknown_temporal_status() {
    let record = record("direct");
    let bytes = legacy_v5_bytes(&record);
    let decoded = decode_record(&bytes).unwrap().unwrap();
    assert_eq!(decoded.temporal_status, "unknown");
    assert_eq!(decoded.authority_kind, "direct");
}

#[test]
fn legacy_memory_record_v4_reopens_without_source_reference() {
    let mut record = record("direct");
    record.source_time_ns = Some(42);
    let mut bytes = legacy_v5_bytes(&record);
    bytes[..8].copy_from_slice(b"CVAMEMR4");
    bytes.remove(93);
    let decoded = decode_record(&bytes).unwrap().unwrap();
    assert_eq!(decoded.source_time_ns, Some(42));
    assert!(decoded.source_ref.is_none());
}

#[test]
fn legacy_memory_record_v3_reopens_without_source_time() {
    let record = record("direct");
    let bytes = legacy_v3_bytes(&record);
    let decoded = decode_record(&bytes).unwrap().unwrap();
    assert_eq!(decoded.authority_kind, "direct");
    assert_eq!(decoded.source_time_ns, None);
    assert_eq!(decoded.mutation_id, record.mutation_id);
}

#[test]
fn legacy_memory_record_v2_reopens_with_unknown_authority_and_no_source_time() {
    let record = record("direct");
    let mut bytes = legacy_v3_bytes(&record);
    bytes[..8].copy_from_slice(b"CVAMEMR2");

    let mut cursor = 100;
    skip_string(&bytes, &mut cursor);
    skip_string(&bytes, &mut cursor);
    let authority_start = cursor;
    skip_string(&bytes, &mut cursor);
    bytes.drain(authority_start..cursor);

    let decoded = decode_record(&bytes).unwrap().unwrap();
    assert_eq!(decoded.authority_kind, "unknown");
    assert_eq!(decoded.source_time_ns, None);
    assert_eq!(decoded.category, record.category);
    assert_eq!(decoded.memory_type, record.memory_type);
    assert_eq!(decoded.mutation_id, record.mutation_id);
}

fn legacy_v3_bytes(record: &MemoryRecord) -> Vec<u8> {
    assert_eq!(record.source_time_ns, None);
    assert_eq!(record.source_ref, None);
    let mut bytes = legacy_v5_bytes(record);
    bytes[..8].copy_from_slice(b"CVAMEMR3");
    bytes.remove(84);
    bytes.remove(84);
    bytes
}

fn legacy_v5_bytes(record: &MemoryRecord) -> Vec<u8> {
    assert_eq!(record.source_ref, None);
    let mut bytes = encode_record(record).unwrap();
    assert_eq!(bytes.pop(), Some(0)); // V8 optional temporal inference flag.
    let mut cursor = 102 + usize::from(record.source_time_ns.is_some()) * 8;
    skip_string(&bytes, &mut cursor); // category
    skip_string(&bytes, &mut cursor); // memory type
    skip_string(&bytes, &mut cursor); // authority kind
    skip_string(&bytes, &mut cursor); // scope
    skip_string(&bytes, &mut cursor); // Dream lifecycle
    let temporal_start = cursor;
    skip_string(&bytes, &mut cursor);
    bytes.drain(temporal_start..cursor);
    bytes[..8].copy_from_slice(b"CVAMEMR5");
    bytes
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
        temporal_status: "current".into(),
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
        source_time_ns: None,
        source_ref: None,
        temporal_inference: None,
        mutation_id: "codec-test".into(),
        created_at_ns: 1,
        updated_at_ns: 2,
        global_version: 3,
        memory_version: 4,
    }
}
