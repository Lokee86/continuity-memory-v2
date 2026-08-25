use crate::{DreamMemoryContext, GraphRelationKind};
use serde_json::{Value, json};

pub(crate) fn canonical_pair<'a>(
    left: &'a DreamMemoryContext,
    right: &'a DreamMemoryContext,
) -> Option<(&'a DreamMemoryContext, &'a DreamMemoryContext)> {
    if left.memory.id == right.memory.id {
        return None;
    }
    Some(if left.memory.id.0 < right.memory.id.0 {
        (left, right)
    } else {
        (right, left)
    })
}

pub(crate) fn context_json(context: &DreamMemoryContext) -> Value {
    let relations: Vec<_> = context
        .graph_relations
        .iter()
        .map(|relation| {
            json!({
                "source": hex(&relation.source.0),
                "target": hex(&relation.target.0),
                "kind": graph_relation_name(relation.kind),
            })
        })
        .collect();
    json!({
        "memory_id": hex(&context.memory.id.0),
        "body_id": hex(&context.body_id.0),
        "category": context.memory.category,
        "memory_type": context.memory.memory_type,
        "lifecycle_state": context.memory.lifecycle_state,
        "title": context.memory.title,
        "content": context.memory.content,
        "source_timestamp_ns": context.source_timestamp_ns,
        "graph_relations": relations,
    })
}

pub(crate) fn memory_text(context: &DreamMemoryContext) -> String {
    format!("{}\n{}", context.memory.title, context.memory.content)
}

pub(crate) fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn graph_relation_name(kind: GraphRelationKind) -> &'static str {
    match kind {
        GraphRelationKind::Topical => "topical",
        GraphRelationKind::Factual => "factual",
        GraphRelationKind::Causal => "causal",
        GraphRelationKind::Recurrent => "recurrent",
        GraphRelationKind::References => "references",
        GraphRelationKind::DuplicateOf => "duplicate_of",
        GraphRelationKind::Supersedes => "supersedes",
        GraphRelationKind::StructuralParent => "structural_parent",
    }
}
