use crate::{Cva, GraphRelationKind, MemoryDraft, MemoryId};
use std::fs;
use std::path::{Path, PathBuf};
pub(crate) fn test_dir() -> PathBuf {
    let unique = uuid::Uuid::new_v4();
    let dir = std::env::temp_dir().join(format!("reliquary-graph-reconcile-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub(crate) fn create_workspace(path: &Path, memories: &[&str]) -> Vec<MemoryId> {
    let mut cva = Cva::create_project(path).unwrap();
    let ids = memories
        .iter()
        .map(|name| publish_memory(&mut cva, name))
        .collect();
    cva.sync().unwrap();
    ids
}

pub(crate) fn publish_memory(cva: &mut Cva, name: &str) -> MemoryId {
    cva.publish_memory(
        None,
        0,
        MemoryDraft {
            category: "fact".into(),
            memory_type: "project".into(),
            authority_kind: "unknown".into(),
            title: name.into(),
            content: format!("memory {name}"),
            scope: "project".into(),
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
            mutation_id: name.into(),
            created_at_ns: 1,
            updated_at_ns: 1,
        },
    )
    .unwrap()
    .0
    .id
}

pub(crate) fn diverge_archive(cva: &mut Cva, side: &str) {
    cva.append_node(
        format!("{side}-node"),
        format!("{side}-conversation"),
        None,
        "user".into(),
        2,
        side,
    )
    .unwrap();
}

pub(crate) fn set(
    cva: &mut Cva,
    source: MemoryId,
    target: MemoryId,
    kind: GraphRelationKind,
    active: bool,
) {
    cva.set_memory_relation(source, target, kind, active, cva.graph_version())
        .unwrap();
}

pub(crate) fn has_relation(
    cva: &Cva,
    source: MemoryId,
    target: MemoryId,
    kind: GraphRelationKind,
) -> bool {
    cva.graph_relations().iter().any(|relation| {
        relation.source == source && relation.target == target && relation.kind == kind
    })
}
