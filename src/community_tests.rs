use crate::community_test_support::{edge, member_sets, path, phy_memory, rel_episode, rel_memory};
use crate::{CommunityError, Cva, GraphRelationKind, Phylactery};
use std::collections::HashSet;
use std::fs;

#[test]
fn leiden_baseline_detects_dense_regions_and_persists() {
    let file = path("communities.prj.rel");
    let mut rel = Cva::create_project(&file).unwrap();
    let episode = rel_episode(&mut rel);
    let a = rel_memory(&mut rel, &episode, "a");
    let b = rel_memory(&mut rel, &episode, "b");
    let c = rel_memory(&mut rel, &episode, "c");
    let d = rel_memory(&mut rel, &episode, "d");
    let e = rel_memory(&mut rel, &episode, "e");
    let f = rel_memory(&mut rel, &episode, "f");

    rel.set_memory_relations(
        &[
            edge(a, b),
            edge(b, c),
            edge(c, a),
            edge(d, e),
            edge(e, f),
            edge(f, d),
            edge(c, d),
        ],
        0,
    )
    .unwrap();

    let snapshot = rel.refresh_communities_leiden().unwrap();
    assert_eq!(snapshot.generation, 1);
    assert_eq!(snapshot.derived_graph_version, 1);
    assert_eq!(snapshot.communities.len(), 2);
    assert_eq!(rel.community_stats().memberships, 6);
    assert!(rel.community_stats().current);

    let expected: HashSet<Vec<[u8; 32]>> = [vec![a.0, b.0, c.0], vec![d.0, e.0, f.0]]
        .into_iter()
        .map(|mut members| {
            members.sort();
            members
        })
        .collect();
    assert_eq!(member_sets(&snapshot), expected);

    rel.sync().unwrap();
    let bytes_before_noop = fs::metadata(&file).unwrap().len();
    let repeated = rel.refresh_communities_leiden().unwrap();
    assert_eq!(repeated, snapshot);
    assert_eq!(fs::metadata(&file).unwrap().len(), bytes_before_noop);
    drop(rel);

    let reopened = Cva::open_project(&file).unwrap();
    assert_eq!(reopened.community_snapshot().unwrap(), snapshot);
    assert!(reopened.community_stats().current);
}

#[test]
fn graph_change_marks_snapshot_stale_until_full_refresh() {
    let file = path("stale.prj.rel");
    let mut rel = Cva::create_project(&file).unwrap();
    let episode = rel_episode(&mut rel);
    let a = rel_memory(&mut rel, &episode, "a");
    let b = rel_memory(&mut rel, &episode, "b");
    let c = rel_memory(&mut rel, &episode, "c");
    rel.set_memory_relations(&[edge(a, b), edge(b, c)], 0)
        .unwrap();
    assert_eq!(rel.refresh_communities_leiden().unwrap().generation, 1);

    rel.set_memory_relation(a, c, GraphRelationKind::Factual, true, 1)
        .unwrap();
    assert!(!rel.community_stats().current);
    let refreshed = rel.refresh_communities_leiden().unwrap();
    assert_eq!(refreshed.generation, 2);
    assert_eq!(refreshed.derived_graph_version, 2);
    assert!(rel.community_stats().current);
}

#[test]
fn phylactery_has_independent_owner_local_communities() {
    let file = path("user.phy");
    let mut phy = Phylactery::create(&file).unwrap();
    let a = phy_memory(&mut phy, "a");
    let b = phy_memory(&mut phy, "b");
    let c = phy_memory(&mut phy, "c");
    let d = phy_memory(&mut phy, "d");
    phy.set_memory_relations(&[edge(a, b), edge(c, d)], 0)
        .unwrap();

    let snapshot = phy.refresh_communities_leiden().unwrap();
    assert_eq!(snapshot.communities.len(), 2);
    assert_eq!(snapshot.derived_graph_version, 1);
    phy.sync().unwrap();
    drop(phy);

    let reopened = Phylactery::open(&file).unwrap();
    assert_eq!(reopened.community_snapshot().unwrap(), snapshot);
    assert!(reopened.community_stats().current);
}

#[test]
fn legacy_owner_cannot_publish_community_identity() {
    let file = path("legacy.cva");
    let mut rel = Cva::create_legacy_cva(&file).unwrap();
    assert!(matches!(
        rel.refresh_communities_leiden(),
        Err(CommunityError::MissingOwnerIdentity)
    ));
}
