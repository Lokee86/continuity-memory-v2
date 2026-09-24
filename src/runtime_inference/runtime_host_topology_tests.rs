use crate::runtime_host_test_support::{one_worker, test_path};
use crate::{
    Cva, EpisodePolicy, InteractionRuntime, Phylactery, ReliquaryRuntimeHost,
    ReliquaryRuntimeRoutes,
};

fn project(name: &str) -> (Cva, String) {
    let cva = Cva::create_project(test_path(name)).unwrap();
    let owner_id = cva.owner_id().unwrap();
    (cva, owner_id)
}

#[test]
fn graph_host_resolves_dependency_closure_and_excludes_siblings() {
    let (root, root_id) = project("root.prj.rel");
    let (mut middle, middle_id) = project("middle.prj.rel");
    middle
        .set_rel_metadata(Some("Middle".into()), vec![root_id.clone()])
        .unwrap();

    let (mut leaf, leaf_id) = project("leaf.prj.rel");
    leaf.set_rel_metadata(Some("Leaf".into()), vec![middle_id.clone()])
        .unwrap();

    let (sibling, sibling_id) = project("sibling.prj.rel");
    let mut host = ReliquaryRuntimeHost::new(
        ReliquaryRuntimeRoutes::default(),
        one_worker(),
        EpisodePolicy::default(),
    );

    host.mount_rel(InteractionRuntime::new(leaf)).unwrap();
    let missing = host.set_active_rel(&leaf_id).unwrap_err().to_string();
    assert!(missing.contains("not mounted"));

    host.mount_rel(InteractionRuntime::new(sibling)).unwrap();
    host.mount_rel(InteractionRuntime::new(root)).unwrap();
    host.mount_rel(InteractionRuntime::new(middle)).unwrap();

    assert!(host.set_active_rel(&leaf_id).unwrap());
    assert_eq!(host.active_rel_id().as_deref(), Some(leaf_id.as_str()));
    assert_eq!(
        host.active_dependency_closure().unwrap(),
        vec![root_id.clone(), middle_id.clone(), leaf_id.clone()]
    );
    assert!(
        !host
            .active_dependency_closure()
            .unwrap()
            .contains(&sibling_id)
    );
    assert_eq!(
        host.rel_metadata().unwrap().type_label.as_deref(),
        Some("Leaf")
    );
}

#[test]
fn graph_host_rejects_mounted_cycles_before_metadata_publication() {
    let (root, root_id) = project("cycle-root.prj.rel");
    let (mut middle, middle_id) = project("cycle-middle.prj.rel");
    middle
        .set_rel_metadata(None, vec![root_id.clone()])
        .unwrap();
    let (mut leaf, leaf_id) = project("cycle-leaf.prj.rel");
    leaf.set_rel_metadata(None, vec![middle_id.clone()])
        .unwrap();

    let mut host = ReliquaryRuntimeHost::new(
        ReliquaryRuntimeRoutes::default(),
        one_worker(),
        EpisodePolicy::default(),
    );
    host.mount_rel(InteractionRuntime::new(root)).unwrap();
    host.mount_rel(InteractionRuntime::new(middle)).unwrap();
    host.mount_rel(InteractionRuntime::new(leaf)).unwrap();

    let error = host
        .set_rel_metadata_for(&root_id, None, vec![leaf_id.clone()])
        .unwrap_err()
        .to_string();
    assert!(error.contains("cycle"));
    assert!(
        host.rel_metadata_for(&root_id)
            .unwrap()
            .dependencies
            .is_empty()
    );

    assert!(host.set_active_rel(&leaf_id).unwrap());
    assert_eq!(
        host.active_dependency_closure().unwrap(),
        vec![root_id, middle_id, leaf_id]
    );
}

#[test]
fn active_switch_keeps_one_host_owned_phylactery() {
    let (left, left_id) = project("phy-left.prj.rel");
    let (right, right_id) = project("phy-right.prj.rel");
    let phylactery = Phylactery::create(test_path("shared.phy")).unwrap();
    let phy_id = phylactery.owner_id().unwrap();

    let mut host = ReliquaryRuntimeHost::new(
        ReliquaryRuntimeRoutes::default(),
        one_worker(),
        EpisodePolicy::default(),
    );
    host.mount_rel(InteractionRuntime::new(left)).unwrap();
    host.mount_rel(InteractionRuntime::new(right)).unwrap();
    host.attach_phylactery(phylactery).unwrap();

    assert!(host.set_active_rel(&left_id).unwrap());
    assert_eq!(
        host.phylactery_owner_id().unwrap().as_deref(),
        Some(phy_id.as_str())
    );

    assert!(host.set_active_rel(&right_id).unwrap());
    assert_eq!(
        host.phylactery_owner_id().unwrap().as_deref(),
        Some(phy_id.as_str())
    );
    assert!(host.has_phylactery().unwrap());

    let detached = host.detach_phylactery().unwrap().unwrap();
    assert_eq!(detached.owner_id().as_deref(), Some(phy_id.as_str()));
}

#[test]
fn unmount_stops_only_the_selected_owner_execution() {
    let (left, left_id) = project("unmount-left.prj.rel");
    let (right, right_id) = project("unmount-right.prj.rel");

    let mut host = ReliquaryRuntimeHost::new(
        ReliquaryRuntimeRoutes::default(),
        one_worker(),
        EpisodePolicy::default(),
    );
    host.mount_rel(InteractionRuntime::new(left)).unwrap();
    host.mount_rel(InteractionRuntime::new(right)).unwrap();
    host.set_active_rel(&right_id).unwrap();

    let left = host.unmount_rel(&left_id).unwrap();
    assert_eq!(left.owner_id().as_deref(), Some(left_id.as_str()));
    assert_eq!(host.mounted_rel_ids(), vec![right_id.clone()]);
    assert_eq!(host.active_rel_id().as_deref(), Some(right_id.as_str()));

    let right = host.unmount_rel(&right_id).unwrap();
    assert_eq!(right.owner_id().as_deref(), Some(right_id.as_str()));
    assert!(host.mounted_rel_ids().is_empty());
    assert_eq!(host.active_rel_id(), None);
}
