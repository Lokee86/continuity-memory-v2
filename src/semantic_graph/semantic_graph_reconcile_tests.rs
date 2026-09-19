use crate::cva_reconcile_graph_test_support::{create_workspace, diverge_archive, test_dir};
use crate::{Cva, EntityDraft, GraphRelationOrigin, SemanticGraphRelationKind};
use std::fs;

#[test]
fn reconcile_replays_right_only_entity_association() {
    let dir = test_dir();
    let left = dir.join("left.cva");
    let right = dir.join("right.cva");
    let output = dir.join("merged.cva");
    let ids = create_workspace(&left, &["a"]);
    fs::copy(&left, &right).unwrap();

    let mut left_cva = Cva::open(&left).unwrap();
    diverge_archive(&mut left_cva, "left");
    left_cva.sync().unwrap();

    let mut right_cva = Cva::open(&right).unwrap();
    diverge_archive(&mut right_cva, "right");
    let (entity, _) = right_cva
        .publish_entity(
            None,
            0,
            EntityDraft {
                canonical_name: "Reliquary".into(),
                aliases: vec!["REL".into()],
                kind: "project".into(),
                summary: "Project referent.".into(),
                mutation_id: "reconcile-entity".into(),
                created_at_ns: 1,
                updated_at_ns: 1,
            },
        )
        .unwrap();
    right_cva
        .set_entity_association(ids[0], entity.id, true, right_cva.graph_version())
        .unwrap();
    right_cva.sync().unwrap();

    let result = Cva::reconcile(&left, &right, &output).unwrap();
    assert_eq!(result.replayed_entity_revisions, 1);
    assert_eq!(result.replayed_graph_transactions, 1);
    assert_eq!(result.replayed_graph_mutations, 1);

    let merged = Cva::open(output).unwrap();
    assert_eq!(
        merged.entity(entity.id).unwrap().canonical_name,
        "Reliquary"
    );
    assert_eq!(
        merged.entity_associations_for_memory(ids[0]),
        vec![entity.id]
    );
    let relation = merged
        .semantic_graph_relations()
        .into_iter()
        .find(|relation| relation.kind == SemanticGraphRelationKind::EntityAssociation)
        .unwrap();
    assert_eq!(relation.origin, GraphRelationOrigin::Perception);
}
