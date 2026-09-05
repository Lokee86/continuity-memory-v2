use crate::dream_candidate_test_support::{memory_with_source_time, test_path};
use crate::dream_cooldown::dream_epoch;
use crate::{Cva, DEFAULT_DREAM_REPROCESS_COOLDOWN_NS, MemoryDraft, Phylactery};
use std::fs;

const DAY_NS: i64 = 24 * 60 * 60 * 1_000_000_000;

#[test]
fn provenance_age_selects_one_current_epoch_without_backlog() {
    assert_eq!(DEFAULT_DREAM_REPROCESS_COOLDOWN_NS, 30 * DAY_NS);
    assert_eq!(dream_epoch(0, 45 * DAY_NS), 1);
    assert_eq!(dream_epoch(0, 60 * DAY_NS), 2);
    assert_eq!(dream_epoch(0, 95 * DAY_NS), 3);
}

#[test]
fn initial_processing_satisfies_the_current_provenance_epoch() {
    let path = test_path("dream-cooldown-initial.cva");
    let mut cva = Cva::create(&path).unwrap();
    let id = memory_with_source_time(
        &mut cva,
        "old-import",
        "Imported memory",
        "Historical knowledge",
        45 * DAY_NS,
        0,
        45 * DAY_NS,
    );
    make_extracted(&mut cva, id);

    let epoch = cva.dream_eligible_epoch(id, 45 * DAY_NS).unwrap().unwrap();
    assert_eq!(epoch, 1);
    cva.reconcile_dream_lifecycle(id).unwrap();
    assert!(cva.mark_dream_epoch(id, epoch).unwrap());

    assert_eq!(cva.dream_eligible_epoch(id, 45 * DAY_NS).unwrap(), None);
    assert_eq!(cva.dream_eligible_epoch(id, 59 * DAY_NS).unwrap(), None);
    assert_eq!(cva.dream_eligible_epoch(id, 60 * DAY_NS).unwrap(), Some(2));
}

#[test]
fn cooldown_marker_survives_reopen_and_skips_missed_epochs() {
    let path = test_path("dream-cooldown-persistence.cva");
    let mut cva = Cva::create(&path).unwrap();
    let id = memory_with_source_time(
        &mut cva,
        "persistent",
        "Persistent memory",
        "Historical knowledge",
        45 * DAY_NS,
        0,
        45 * DAY_NS,
    );
    let memory_version = cva.memory_version();
    assert!(cva.mark_dream_epoch(id, 1).unwrap());
    assert_eq!(cva.memory_version(), memory_version);
    cva.sync().unwrap();
    drop(cva);

    let mut cva = Cva::open(&path).unwrap();
    assert_eq!(cva.dream_eligible_epoch(id, 45 * DAY_NS).unwrap(), None);
    assert_eq!(cva.dream_eligible_epoch(id, 95 * DAY_NS).unwrap(), Some(3));
    assert!(cva.mark_dream_epoch(id, 3).unwrap());
    assert_eq!(cva.dream_eligible_epoch(id, 95 * DAY_NS).unwrap(), None);
}

#[test]
fn phylactery_cooldown_state_survives_reopen() {
    let path = test_path("dream-cooldown.phy");
    let mut phy = Phylactery::create(&path).unwrap();
    let draft = MemoryDraft {
        category: "preference".into(),
        memory_type: "user".into(),
        authority_kind: "direct".into(),
        title: "Preference".into(),
        content: "Use concise output.".into(),
        scope: "private".into(),
        lifecycle_state: "knowledge".into(),
        archived: false,
        superseded_by: None,
        parent_id: None,
        source_node_id: None,
        content_source_conversation_id: None,
        content_source_node_id: None,
        grounding_source_conversation_id: None,
        grounding_source_node_id: None,
        source_episode_id: None,
        source_time_ns: Some(0),
        mutation_id: "phy-cooldown".into(),
        created_at_ns: 45 * DAY_NS,
        updated_at_ns: 45 * DAY_NS,
    };
    let id = phy.publish_memory(None, 0, draft).unwrap().0.id;
    assert!(phy.mark_dream_epoch(id, 1).unwrap());
    phy.sync().unwrap();
    drop(phy);

    let mut phy = Phylactery::open(path).unwrap();
    assert_eq!(phy.dream_eligible_epoch(id, 59 * DAY_NS).unwrap(), None);
    assert_eq!(phy.dream_eligible_epoch(id, 60 * DAY_NS).unwrap(), Some(2));
}

#[test]
fn divergent_reconcile_keeps_the_highest_satisfied_epoch() {
    let left = test_path("dream-cooldown-left.cva");
    let right = left.with_file_name("dream-cooldown-right.cva");
    let output = left.with_file_name("dream-cooldown-merged.cva");
    let mut base = Cva::create_project(&left).unwrap();
    let id = memory_with_source_time(
        &mut base,
        "reconcile",
        "Reconciled memory",
        "Shared knowledge",
        45 * DAY_NS,
        0,
        45 * DAY_NS,
    );
    base.sync().unwrap();
    drop(base);
    fs::copy(&left, &right).unwrap();

    for (path, epoch, node) in [(&left, 1, "left-node"), (&right, 3, "right-node")] {
        let mut cva = Cva::open(path).unwrap();
        cva.mark_dream_epoch(id, epoch).unwrap();
        cva.append_node(
            node.into(),
            format!("{node}-conversation"),
            None,
            "user".into(),
            46 * DAY_NS,
            node,
        )
        .unwrap();
        cva.sync().unwrap();
    }

    Cva::reconcile(&left, &right, &output).unwrap();
    let mut merged = Cva::open(output).unwrap();
    assert_eq!(
        merged
            .dream_cooldown_records()
            .into_iter()
            .find(|(memory_id, _)| *memory_id == id)
            .map(|(_, epoch)| epoch),
        Some(3)
    );
    assert_eq!(merged.dream_eligible_epoch(id, 95 * DAY_NS).unwrap(), None);
    assert_eq!(
        merged.dream_eligible_epoch(id, 120 * DAY_NS).unwrap(),
        Some(4)
    );
}

#[test]
fn legacy_processed_memory_uses_updated_time_as_non_flooding_baseline() {
    let path = test_path("dream-cooldown-legacy.cva");
    let mut cva = Cva::create(path).unwrap();
    let id = memory_with_source_time(
        &mut cva,
        "legacy",
        "Legacy memory",
        "Already processed before cooldown markers existed",
        45 * DAY_NS,
        0,
        45 * DAY_NS,
    );

    assert_eq!(cva.dream_eligible_epoch(id, 45 * DAY_NS).unwrap(), None);
    assert_eq!(cva.dream_eligible_epoch(id, 59 * DAY_NS).unwrap(), None);
    assert_eq!(cva.dream_eligible_epoch(id, 60 * DAY_NS).unwrap(), Some(2));
}

fn make_extracted(cva: &mut Cva, id: crate::MemoryId) {
    let memory = cva.memory(id).unwrap();
    let draft = MemoryDraft {
        category: memory.category.clone(),
        memory_type: memory.memory_type.clone(),
        authority_kind: memory.authority_kind.clone(),
        title: memory.title.clone(),
        content: memory.content.clone(),
        scope: memory.scope.clone(),
        lifecycle_state: "extracted".into(),
        archived: false,
        superseded_by: memory.superseded_by,
        parent_id: memory.parent_id,
        source_node_id: memory.source_node_id.clone(),
        content_source_conversation_id: memory.content_source_conversation_id.clone(),
        content_source_node_id: memory.content_source_node_id.clone(),
        grounding_source_conversation_id: memory.grounding_source_conversation_id.clone(),
        grounding_source_node_id: memory.grounding_source_node_id.clone(),
        source_episode_id: memory.source_episode_id,
        source_time_ns: memory.source_time_ns,
        mutation_id: "dream-cooldown-extracted".into(),
        created_at_ns: memory.created_at_ns,
        updated_at_ns: memory.updated_at_ns.saturating_add(1),
    };
    cva.publish_memory(Some(id), memory.revision, draft)
        .unwrap();
}
