use crate::runtime_host_test_support::{one_worker, test_path};
use crate::{
    Cva, EchoEvent, EchoEventKind, EpisodePolicy, InteractionRuntime, Phylactery,
    ReliquaryRuntimeHost, ReliquaryRuntimeRoutes, RuntimeKnowledgeProvenanceResolution,
    RuntimeSemanticOwnerKind,
};

fn project(name: &str) -> (Cva, String) {
    let cva = Cva::create_project(test_path(name)).unwrap();
    let owner_id = cva.owner_id().unwrap();
    (cva, owner_id)
}

#[test]
fn visible_semantic_owners_follow_dependency_closure_plus_phylactery() {
    let (root, root_id) = project("semantic-root.prj.rel");
    let (mut middle, middle_id) = project("semantic-middle.prj.rel");
    middle
        .set_rel_metadata(None, vec![root_id.clone()])
        .unwrap();
    let (mut leaf, leaf_id) = project("semantic-leaf.prj.rel");
    leaf.set_rel_metadata(None, vec![middle_id.clone()])
        .unwrap();
    let (sibling, sibling_id) = project("semantic-sibling.prj.rel");
    let phylactery = Phylactery::create(test_path("semantic-user.phy")).unwrap();
    let phy_id = phylactery.owner_id().unwrap();

    let mut host = ReliquaryRuntimeHost::new(
        ReliquaryRuntimeRoutes::default(),
        one_worker(),
        EpisodePolicy::default(),
    );
    host.mount_rel(InteractionRuntime::new(leaf)).unwrap();
    host.mount_rel(InteractionRuntime::new(sibling)).unwrap();
    host.mount_rel(InteractionRuntime::new(root)).unwrap();
    host.mount_rel(InteractionRuntime::new(middle)).unwrap();
    host.attach_phylactery(phylactery).unwrap();
    host.set_active_rel(&leaf_id).unwrap();

    let visible = host.visible_semantic_owners().unwrap();
    assert_eq!(
        visible
            .iter()
            .map(|owner| owner.owner_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            root_id.as_str(),
            middle_id.as_str(),
            leaf_id.as_str(),
            phy_id.as_str(),
        ]
    );
    assert_eq!(visible[0].kind, RuntimeSemanticOwnerKind::Reliquary);
    assert_eq!(visible[3].kind, RuntimeSemanticOwnerKind::Phylactery);

    assert_eq!(
        host.semantic_owner(&sibling_id).unwrap().kind,
        RuntimeSemanticOwnerKind::Reliquary
    );
    let error = host
        .visible_semantic_owner(&sibling_id)
        .unwrap_err()
        .to_string();
    assert!(error.contains("not visible from the active REL"));
}

#[test]
fn generic_knowledge_access_dispatches_by_owner_kind() {
    let (rel, rel_id) = project("semantic-knowledge.prj.rel");
    let phylactery = Phylactery::create(test_path("semantic-knowledge.phy")).unwrap();
    let phy_id = phylactery.owner_id().unwrap();

    let mut host = ReliquaryRuntimeHost::new(
        ReliquaryRuntimeRoutes::default(),
        one_worker(),
        EpisodePolicy::default(),
    );
    host.mount_rel(InteractionRuntime::new(rel)).unwrap();
    host.attach_phylactery(phylactery).unwrap();
    host.set_active_rel(&rel_id).unwrap();

    let rel_memory = host
        .create_knowledge_memory_for_owner(
            &rel_id,
            "REL fact".into(),
            "Project-owned knowledge".into(),
            "knowledge".into(),
            "fact".into(),
        )
        .unwrap();
    let phy_memory = host
        .create_knowledge_memory_for_owner(
            &phy_id,
            "PHY fact".into(),
            "User-owned knowledge".into(),
            "knowledge".into(),
            "fact".into(),
        )
        .unwrap();

    let rel_state = host.read_knowledge_for_owner(&rel_id).unwrap();
    let phy_state = host.read_knowledge_for_owner(&phy_id).unwrap();
    assert!(rel_state.0.iter().any(|memory| memory.id == rel_memory.id));
    assert!(phy_state.0.iter().any(|memory| memory.id == phy_memory.id));

    let resolution = host
        .resolve_knowledge_provenance_for_owner(&phy_id, phy_memory.id)
        .unwrap();
    assert!(matches!(
        resolution,
        RuntimeKnowledgeProvenanceResolution::None {
            memory_owner_id,
            provenance: None,
        } if memory_owner_id == phy_id
    ));

    let error = host
        .read_memory_provenance_for_owner(&phy_id, phy_memory.id)
        .unwrap_err()
        .to_string();
    assert!(error.contains("do not have Reliquary Archive provenance"));
}

#[test]
fn echo_resolution_is_owner_qualified_and_path_bounded() {
    let (mut rel, rel_id) = project("semantic-echo.prj.rel");
    rel.append_node(
        "u1".into(),
        "conversation".into(),
        None,
        "user".into(),
        1,
        "Question",
    )
    .unwrap();
    rel.append_node(
        "a1".into(),
        "conversation".into(),
        Some("u1".into()),
        "assistant".into(),
        2,
        "Answer",
    )
    .unwrap();
    rel.put_echo_event(EchoEvent {
        conversation_id: "conversation".into(),
        message_id: "a1".into(),
        sequence: 0,
        timestamp_ns: 3,
        model_round: Some(1),
        kind: EchoEventKind::ReasoningSummary,
        correlation_id: None,
        name: None,
        content: "Reasoned answer".into(),
    })
    .unwrap();
    rel.sync().unwrap();

    let mut host = ReliquaryRuntimeHost::new(
        ReliquaryRuntimeRoutes::default(),
        one_worker(),
        EpisodePolicy::default(),
    );
    host.mount_rel(InteractionRuntime::new(rel)).unwrap();
    host.set_active_rel(&rel_id).unwrap();

    let source = host
        .read_echo_source_for(&rel_id, "conversation", "u1", "a1")
        .unwrap();
    assert_eq!(source.owner_id, rel_id);
    assert_eq!(source.turns.len(), 1);
    assert_eq!(source.turns[0].message_id, "a1");
    assert_eq!(source.turns[0].events[0].content, "Reasoned answer");

    let error = host
        .read_echo_source_for(&source.owner_id, "conversation", "missing", "a1")
        .unwrap_err()
        .to_string();
    assert!(error.contains("start is not on the returned transcript path"));
}
