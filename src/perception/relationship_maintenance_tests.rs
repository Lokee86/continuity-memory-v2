use crate::relationship_test_support::{draft, entity, temp_path};
use crate::{Cva, EntityId, EntityRef, RelationshipParticipant};
use std::fs;

#[test]
fn entity_merge_retargets_local_relationship_participants() {
    let path = temp_path("relationship-merge.rel");
    let mut rel = Cva::create_project(&path).unwrap();
    let owner_id = rel.owner_id().unwrap();
    let survivor = entity(&mut rel, "Canonical", 1);
    let retired = entity(&mut rel, "Alias", 2);
    let (relationship, _) = rel
        .publish_relationship(
            None,
            0,
            draft(
                vec![
                    RelationshipParticipant {
                        entity: EntityRef {
                            owner_id: owner_id.clone(),
                            entity_id: retired,
                        },
                        role: Some("member".into()),
                    },
                    RelationshipParticipant {
                        entity: EntityRef {
                            owner_id: "phy-cccccccc-cccc-cccc-cccc-cccccccccccc".into(),
                            entity_id: EntityId([3; 32]),
                        },
                        role: Some("counterparty".into()),
                    },
                ],
                "retarget-me",
                "before merge",
            ),
        )
        .unwrap();

    let outcome = rel.merge_entities(survivor, retired, 10).unwrap();
    assert_eq!(outcome.relationships_retargeted, 1);

    let revised = rel.relationship(relationship.id).unwrap();
    assert_eq!(revised.revision, 2);
    assert!(revised.participants.iter().any(|participant| {
        participant.entity.owner_id == owner_id && participant.entity.entity_id == survivor
    }));
    assert!(
        !revised
            .participants
            .iter()
            .any(|participant| participant.entity.entity_id == retired)
    );

    let stale = rel.publish_relationship(
        None,
        0,
        draft(
            vec![RelationshipParticipant {
                entity: EntityRef {
                    owner_id,
                    entity_id: retired,
                },
                role: None,
            }],
            "retired-participant",
            "must reject stale identity",
        ),
    );
    assert!(matches!(
        stale,
        Err(crate::RelationshipError::MissingLocalEntity(id)) if id == retired
    ));
}

#[test]
fn history_replay_preserves_pre_merge_relationship_revisions() {
    let path = temp_path("relationship-history-replay.rel");
    let mut rel = Cva::create_project(&path).unwrap();
    let owner_id = rel.owner_id().unwrap();
    let survivor = entity(&mut rel, "Canonical History", 1);
    let retired = entity(&mut rel, "Retired History", 2);
    rel.merge_entities(survivor, retired, 10).unwrap();

    let first = rel
        .replay_relationship_revision(
            None,
            0,
            draft(
                vec![RelationshipParticipant {
                    entity: EntityRef {
                        owner_id: owner_id.clone(),
                        entity_id: retired,
                    },
                    role: Some("member".into()),
                }],
                "history-before-merge",
                "historical retired identity",
            ),
        )
        .unwrap()
        .0;
    rel.replay_relationship_revision(
        Some(first.id),
        1,
        draft(
            vec![RelationshipParticipant {
                entity: EntityRef {
                    owner_id,
                    entity_id: survivor,
                },
                role: Some("member".into()),
            }],
            "history-after-merge",
            "current survivor identity",
        ),
    )
    .unwrap();

    rel.sync().unwrap();
    drop(rel);

    let reopened = Cva::open(&path).unwrap();
    assert_eq!(reopened.relationship(first.id).unwrap().revision, 2);
    assert_eq!(reopened.relationship_stats().revisions, 2);
}

#[test]
fn divergent_reconcile_replays_relationship_revisions() {
    let base = temp_path("relationship-reconcile-base.rel");
    let left = temp_path("relationship-reconcile-left.rel");
    let right = temp_path("relationship-reconcile-right.rel");
    let output = temp_path("relationship-reconcile-output.rel");

    let mut rel = Cva::create_project(&base).unwrap();
    let entity_id = entity(&mut rel, "Shared", 1);
    let owner_id = rel.owner_id().unwrap();
    rel.sync().unwrap();
    drop(rel);
    fs::copy(&base, &left).unwrap();
    fs::copy(&base, &right).unwrap();

    let participant = RelationshipParticipant {
        entity: EntityRef {
            owner_id,
            entity_id,
        },
        role: None,
    };
    let mut left_rel = Cva::open(&left).unwrap();
    left_rel
        .publish_relationship(
            None,
            0,
            draft(vec![participant.clone()], "left-relationship", "left"),
        )
        .unwrap();
    left_rel.sync().unwrap();
    drop(left_rel);

    let mut right_rel = Cva::open(&right).unwrap();
    right_rel
        .publish_relationship(
            None,
            0,
            draft(vec![participant], "right-relationship", "right"),
        )
        .unwrap();
    right_rel.sync().unwrap();
    drop(right_rel);

    let result = Cva::reconcile(&left, &right, &output).unwrap();
    assert_eq!(result.replayed_relationship_revisions, 1);
    assert_eq!(result.duplicate_relationship_revisions, 0);
    assert_eq!(Cva::open(&output).unwrap().relationships().len(), 2);
}

#[test]
fn reclamation_preserves_relationship_records() {
    let source = temp_path("relationship-reclaim-source.rel");
    let output = temp_path("relationship-reclaim-output.rel");
    let mut rel = Cva::create_project(&source).unwrap();
    let entity_id = entity(&mut rel, "Stored", 1);
    let owner_id = rel.owner_id().unwrap();
    let (relationship, _) = rel
        .publish_relationship(
            None,
            0,
            draft(
                vec![RelationshipParticipant {
                    entity: EntityRef {
                        owner_id,
                        entity_id,
                    },
                    role: None,
                }],
                "reclaim-relationship",
                "preserve me",
            ),
        )
        .unwrap();

    rel.reclaim_storage(&output).unwrap();
    let reopened = Cva::open(&output).unwrap();
    assert_eq!(
        reopened.relationship(relationship.id).unwrap(),
        relationship
    );
}
