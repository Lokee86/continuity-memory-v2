use crate::entity_resolution_test_support::rel_with_three_mentions;
use crate::{
    Cva, EntityDraft, EntityError, EntityId, EntityResolutionReason, MemoryError, Phylactery,
};
use std::{fs, path::PathBuf};

#[test]
fn entity_identity_metadata_and_alias_candidates_survive_reopen() {
    let path = temp_path("entity-owner.rel");
    let mut rel = Cva::create_project(&path).unwrap();

    let (alpha, created) = rel
        .publish_entity(
            None,
            0,
            draft(" Alpha ", vec!["A", " Alpha ", "A"], "project", 1),
        )
        .unwrap();
    assert!(created);
    assert_eq!(alpha.canonical_name, "Alpha");
    assert_eq!(alpha.aliases, vec!["A", "Alpha"]);

    let second_id = EntityId([9; 32]);
    rel.publish_entity(Some(second_id), 0, draft("Alpha", vec!["A"], "service", 1))
        .unwrap();

    let mut revised = draft("Alpha Prime", vec!["Alpha", "A1"], "project", 2);
    revised.created_at_ns = alpha.created_at_ns;
    let (updated, created) = rel
        .publish_entity(Some(alpha.id), alpha.revision, revised)
        .unwrap();
    assert!(created);
    assert_eq!(updated.id, alpha.id);
    assert_eq!(updated.revision, 2);

    let candidates = rel.entity_candidates_for_surface("alpha", 8);
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].id.min(candidates[1].id), candidates[0].id);

    rel.sync().unwrap();
    drop(rel);

    let reopened = Cva::open(&path).unwrap();
    assert_eq!(reopened.entity(alpha.id).unwrap(), updated);
    assert_eq!(reopened.entity_stats().entities, 2);
    assert_eq!(reopened.entity_stats().revisions, 3);
    assert_eq!(reopened.entity_version(), 3);
    assert_eq!(reopened.entity_candidates_for_surface("alpha", 8).len(), 2);
}

#[test]
fn entity_revision_cannot_change_creation_identity() {
    let path = temp_path("entity-revision.rel");
    let mut rel = Cva::create_project(&path).unwrap();
    let (entity, _) = rel
        .publish_entity(None, 0, draft("Alpha", vec![], "project", 1))
        .unwrap();

    let mut changed = draft("Alpha", vec![], "project", 2);
    changed.created_at_ns = entity.created_at_ns + 1;
    assert!(matches!(
        rel.publish_entity(Some(entity.id), 1, changed),
        Err(EntityError::InvalidField("Entity created_at"))
    ));
}

#[test]
fn resolution_rejects_entity_ids_that_do_not_exist() {
    let (mut rel, _, keys, _) = rel_with_three_mentions();
    assert!(matches!(
        rel.put_entity_resolved(
            keys[0],
            0,
            EntityId([99; 32]),
            EntityResolutionReason::ContextMatch,
            10,
        ),
        Err(MemoryError::InvalidField(
            "Entity resolution Entity reference"
        ))
    ));
}

#[test]
fn phylactery_entity_owner_reopens_independently() {
    let path = temp_path("entity-owner.phy");
    let mut phy = Phylactery::create(&path).unwrap();
    let (entity, _) = phy
        .publish_entity(None, 0, draft("Sarah", vec!["S"], "person", 1))
        .unwrap();
    phy.sync().unwrap();
    drop(phy);

    let reopened = Phylactery::open(&path).unwrap();
    assert_eq!(reopened.entity(entity.id).unwrap(), entity);
}

#[test]
fn divergent_reconcile_replays_right_entity_revision() {
    let base = temp_path("entity-base.rel");
    let left = temp_path("entity-left.rel");
    let right = temp_path("entity-right.rel");
    let output = temp_path("entity-merged.rel");

    let rel = Cva::create_project(&base).unwrap();
    rel.sync().unwrap();
    drop(rel);
    fs::copy(&base, &left).unwrap();
    fs::copy(&base, &right).unwrap();

    let mut left_rel = Cva::open(&left).unwrap();
    let (left_entity, _) = left_rel
        .publish_entity(None, 0, draft("Left", vec![], "project", 1))
        .unwrap();
    left_rel.sync().unwrap();
    drop(left_rel);

    let mut right_rel = Cva::open(&right).unwrap();
    let (right_entity, _) = right_rel
        .publish_entity(None, 0, draft("Right", vec![], "project", 1))
        .unwrap();
    right_rel.sync().unwrap();
    drop(right_rel);

    let result = Cva::reconcile(&left, &right, &output).unwrap();
    assert_eq!(result.replayed_entity_revisions, 1);
    assert_eq!(result.duplicate_entity_revisions, 0);

    let merged = Cva::open(&output).unwrap();
    assert_eq!(
        merged.entity(left_entity.id).unwrap().canonical_name,
        "Left"
    );
    assert_eq!(
        merged.entity(right_entity.id).unwrap().canonical_name,
        "Right"
    );
}

fn draft(name: &str, aliases: Vec<&str>, kind: &str, revision: u64) -> EntityDraft {
    EntityDraft {
        canonical_name: name.into(),
        aliases: aliases.into_iter().map(str::to_owned).collect(),
        kind: kind.into(),
        summary: format!("{name} semantic summary."),
        mutation_id: format!("entity-test-{name}-{revision}-{}", uuid::Uuid::new_v4()),
        created_at_ns: 1,
        updated_at_ns: revision as i64,
    }
}

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "reliquary-entity-owner-{}-{name}",
        uuid::Uuid::new_v4()
    ))
}
