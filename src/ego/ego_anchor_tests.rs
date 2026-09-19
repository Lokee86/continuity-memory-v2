use super::test_path;
use crate::{Cva, EgoAnchorPriority, Phylactery};
use std::fs;

#[test]
fn anchors_persist_update_and_tombstone_in_both_owner_kinds() {
    let phy_path = test_path("anchors.phy");
    let mut phy = Phylactery::create(&phy_path).unwrap();
    let anchor = phy
        .create_ego_anchor(
            EgoAnchorPriority::High,
            "Keep this globally salient.".into(),
        )
        .unwrap();
    let (anchor, changed) = phy
        .update_ego_anchor(
            anchor.id,
            anchor.revision,
            EgoAnchorPriority::Normal,
            "Keep this globally salient, but normally.".into(),
        )
        .unwrap();
    assert!(changed);
    assert_eq!(anchor.revision, 2);
    phy.sync().unwrap();
    drop(phy);

    let mut phy = Phylactery::open(&phy_path).unwrap();
    assert_eq!(phy.ego_anchors().len(), 1);
    assert!(phy.delete_ego_anchor(anchor.id, 2).unwrap());
    phy.sync().unwrap();
    drop(phy);
    assert!(
        Phylactery::open(&phy_path)
            .unwrap()
            .ego_anchors()
            .is_empty()
    );

    let rel_path = test_path("anchors.rel");
    let mut rel = Cva::create(&rel_path).unwrap();
    let anchor = rel
        .create_ego_anchor(EgoAnchorPriority::Low, "REL-local standing context.".into())
        .unwrap();
    rel.sync().unwrap();
    drop(rel);

    let reopened = Cva::open(&rel_path).unwrap();
    assert_eq!(reopened.ego_anchors(), vec![anchor]);
}

#[test]
fn divergent_reconciliation_fails_closed_when_ego_state_exists() {
    let dir =
        std::env::temp_dir().join(format!("reliquary-ego-reconcile-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&dir).unwrap();
    let left = dir.join("left.rel");
    let right = dir.join("right.rel");
    let output = dir.join("merged.rel");

    let mut base = Cva::create(&left).unwrap();
    base.create_ego_anchor(EgoAnchorPriority::Normal, "Preserve this Anchor.".into())
        .unwrap();
    base.sync().unwrap();
    drop(base);
    fs::copy(&left, &right).unwrap();

    for (path, id, content) in [
        (&left, "left-node", "left change"),
        (&right, "right-node", "right change"),
    ] {
        let mut rel = Cva::open(path).unwrap();
        rel.append_node(
            id.into(),
            format!("{id}-conversation"),
            None,
            "user".into(),
            1,
            content,
        )
        .unwrap();
        rel.sync().unwrap();
    }

    assert!(matches!(
        Cva::reconcile(&left, &right, &output),
        Err(crate::CvaReconcileError::UnsupportedSemanticOwner(
            "Ego state in divergent reconciliation"
        ))
    ));
    assert!(!output.exists());
}
