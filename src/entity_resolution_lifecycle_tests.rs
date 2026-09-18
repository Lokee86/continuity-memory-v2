use crate::entity_resolution_test_support::{
    archive_rel_memory, rel_with_three_mentions, supersede_rel_memory,
};
use crate::{
    DEFAULT_ENTITY_RESOLUTION_DORMANT_TTL_NS, DEFAULT_ENTITY_RESOLUTION_PENDING_TTL_NS, EntityId,
    EntityResolutionReason, MemoryEntityResolutionStatus,
};

#[test]
fn pending_compacts_to_dormant_then_purges() {
    let (mut rel, _, keys, _) = rel_with_three_mentions();
    let key = keys[0];
    let started = 1_000_000_000_i64;
    rel.put_entity_unresolved(
        key,
        0,
        vec![EntityId([1; 32])],
        EntityResolutionReason::Ambiguous,
        [1; 32],
        [2; 32],
        started,
    )
    .unwrap();

    let before = rel
        .compact_entity_resolutions(started + DEFAULT_ENTITY_RESOLUTION_PENDING_TTL_NS - 1)
        .unwrap();
    assert_eq!(before.dormant, 0);
    assert!(matches!(
        rel.entity_resolution(key).unwrap().status,
        MemoryEntityResolutionStatus::Pending(_)
    ));

    let compacted = rel
        .compact_entity_resolutions(started + DEFAULT_ENTITY_RESOLUTION_PENDING_TTL_NS)
        .unwrap();
    assert_eq!(compacted.dormant, 1);
    let dormant_at = started + DEFAULT_ENTITY_RESOLUTION_PENDING_TTL_NS;
    assert!(matches!(
        rel.entity_resolution(key).unwrap().status,
        MemoryEntityResolutionStatus::Dormant(_)
    ));

    assert_eq!(
        rel.compact_entity_resolutions(dormant_at + DEFAULT_ENTITY_RESOLUTION_DORMANT_TTL_NS - 1)
            .unwrap()
            .purged,
        0
    );
    assert_eq!(
        rel.compact_entity_resolutions(dormant_at + DEFAULT_ENTITY_RESOLUTION_DORMANT_TTL_NS)
            .unwrap()
            .purged,
        1
    );
    assert!(rel.entity_resolution(key).is_none());
    assert!(rel.entity_resolution_retry_needed(key, [1; 32], [2; 32]));
}

#[test]
fn negative_elapsed_time_does_not_compact() {
    let (mut rel, _, keys, _) = rel_with_three_mentions();
    rel.put_entity_unresolved(
        keys[0],
        0,
        vec![],
        EntityResolutionReason::InsufficientEvidence,
        [1; 32],
        [2; 32],
        500,
    )
    .unwrap();
    let result = rel.compact_entity_resolutions(100).unwrap();
    assert_eq!(result.dormant, 0);
    assert!(matches!(
        rel.entity_resolution(keys[0]).unwrap().status,
        MemoryEntityResolutionStatus::Pending(_)
    ));
}

#[test]
fn inactive_memory_purges_nonterminal_noise_but_keeps_resolved() {
    let (mut rel, memory_id, keys, _) = rel_with_three_mentions();
    rel.put_entity_resolved(
        keys[0],
        0,
        EntityId([1; 32]),
        EntityResolutionReason::ContextMatch,
        10,
    )
    .unwrap();
    rel.put_entity_unresolved(
        keys[1],
        0,
        vec![],
        EntityResolutionReason::Ambiguous,
        [2; 32],
        [3; 32],
        10,
    )
    .unwrap();
    rel.put_entity_rejected(keys[2], 0, EntityResolutionReason::GenericRole, 10)
        .unwrap();

    archive_rel_memory(&mut rel, memory_id);
    let result = rel.compact_entity_resolutions(20).unwrap();
    assert_eq!(result.purged, 1);
    assert_eq!(result.rejected_purged, 1);
    assert!(matches!(
        rel.entity_resolution(keys[0]).unwrap().status,
        MemoryEntityResolutionStatus::Resolved { .. }
    ));
    assert!(rel.entity_resolution(keys[1]).is_none());
    assert!(rel.entity_resolution(keys[2]).is_none());
}

#[test]
fn superseded_memory_purges_pending_state() {
    let (mut rel, memory_id, keys, _) = rel_with_three_mentions();
    rel.put_entity_unresolved(
        keys[0],
        0,
        vec![],
        EntityResolutionReason::Ambiguous,
        [1; 32],
        [2; 32],
        10,
    )
    .unwrap();

    supersede_rel_memory(&mut rel, memory_id);
    assert_eq!(rel.compact_entity_resolutions(20).unwrap().purged, 1);
    assert!(rel.entity_resolution(keys[0]).is_none());
}

#[test]
fn dormant_state_wakes_only_on_changed_evidence() {
    let (mut rel, _, keys, _) = rel_with_three_mentions();
    let key = keys[0];
    rel.put_entity_unresolved(
        key,
        0,
        vec![EntityId([1; 32])],
        EntityResolutionReason::Ambiguous,
        [1; 32],
        [2; 32],
        0,
    )
    .unwrap();
    rel.compact_entity_resolutions(DEFAULT_ENTITY_RESOLUTION_PENDING_TTL_NS)
        .unwrap();

    let revision = rel.entity_resolution(key).unwrap().revision;
    assert!(!rel.entity_resolution_retry_needed(key, [1; 32], [2; 32]));
    assert!(
        !rel.put_entity_unresolved(
            key,
            revision,
            vec![EntityId([1; 32])],
            EntityResolutionReason::Ambiguous,
            [1; 32],
            [2; 32],
            DEFAULT_ENTITY_RESOLUTION_PENDING_TTL_NS + 1,
        )
        .unwrap()
    );

    rel.put_entity_unresolved(
        key,
        revision,
        vec![EntityId([1; 32]), EntityId([2; 32])],
        EntityResolutionReason::Ambiguous,
        [9; 32],
        [2; 32],
        DEFAULT_ENTITY_RESOLUTION_PENDING_TTL_NS + 1,
    )
    .unwrap();
    assert!(matches!(
        rel.entity_resolution(key).unwrap().status,
        MemoryEntityResolutionStatus::Pending(_)
    ));
}
