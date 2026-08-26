use reliquary_memory::{
    Cva, DreamCandidateSet, DreamPairClassification, DreamProcessResult, DreamPublicationOutcome,
    GraphRelation, Memory, MemoryId,
};
use serde_json::{Value, json};
use std::collections::HashMap;

pub fn id_hex(id: MemoryId) -> String {
    id.0.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn memory_json(memory: &Memory) -> Value {
    json!({
        "id": id_hex(memory.id),
        "revision": memory.revision,
        "category": memory.category,
        "type": memory.memory_type,
        "authority_kind": memory.authority_kind,
        "title": memory.title,
        "content": memory.content,
        "scope": memory.scope,
        "lifecycle": memory.lifecycle_state,
        "archived": memory.archived,
        "superseded_by": memory.superseded_by.map(id_hex),
        "parent_id": memory.parent_id.map(id_hex),
        "source_node_id": memory.source_node_id,
        "created_at_ns": memory.created_at_ns,
        "updated_at_ns": memory.updated_at_ns,
        "global_version": memory.global_version,
        "memory_version": memory.memory_version,
    })
}

pub fn candidates_json(set: &DreamCandidateSet) -> Value {
    Value::Array(
        set.candidates
            .iter()
            .map(|candidate| {
                json!({
                    "id": id_hex(candidate.context.memory.id),
                    "title": candidate.context.memory.title,
                    "semantic_score": candidate.semantic_score,
                    "semantic_rank": candidate.semantic_rank,
                    "prior_semantic_rank": candidate.prior_semantic_rank,
                    "lexical_score": candidate.lexical_score,
                    "lexical_rank": candidate.lexical_rank,
                    "temporal_score": candidate.temporal_score,
                    "temporal_rank": candidate.temporal_rank,
                    "temporal_matches": candidate.temporal_matches.iter()
                        .map(|value| format!("{value:?}"))
                        .collect::<Vec<_>>(),
                })
            })
            .collect(),
    )
}

pub fn result_json(result: &DreamProcessResult) -> Value {
    let pairs = result
        .pairs
        .iter()
        .map(|pair| {
            let classification = &pair.classification;
            let evidence = classification
                .evidence
                .iter()
                .map(|item| {
                    json!({
                        "side": format!("{:?}", item.side),
                        "quote": item.quote,
                    })
                })
                .collect::<Vec<_>>();
            let verification = pair.verification.as_ref().map(|value| {
                json!({
                    "model": value.verifier_model,
                    "relation_supported": format!("{:?}", value.relation_supported),
                    "direction_supported": format!("{:?}", value.direction_supported),
                    "evidence_supported": format!("{:?}", value.evidence_supported),
                    "verdict": format!("{:?}", value.verdict),
                })
            });
            let candidate_id = pair_candidate_id(result.source.id, classification);
            json!({
                "candidate_id": id_hex(candidate_id),
                "classification": {
                    "model": classification.model,
                    "relation": format!("{:?}", classification.relation),
                    "direction": format!("{:?}", classification.direction),
                    "evidence": evidence,
                },
                "verification": verification,
                "publication": publication_json(&pair.publication),
            })
        })
        .collect::<Vec<_>>();

    json!({
        "source_after": memory_json(&result.source),
        "candidate_count": result.candidate_count,
        "pairs": pairs,
        "lifecycle": {
            "revised": result.lifecycle.revised.iter().map(memory_json).collect::<Vec<_>>(),
            "archived": result.lifecycle.archived.iter().copied().map(id_hex).collect::<Vec<_>>(),
            "canonicalized": result.lifecycle.canonicalized.iter().copied().map(id_hex).collect::<Vec<_>>(),
            "promoted_to_knowledge": result.lifecycle.promoted_to_knowledge,
        }
    })
}

pub(crate) fn pair_candidate_id(
    source_id: MemoryId,
    classification: &DreamPairClassification,
) -> MemoryId {
    if classification.a == source_id {
        classification.b
    } else {
        debug_assert_eq!(classification.b, source_id);
        classification.a
    }
}

fn publication_json(outcome: &DreamPublicationOutcome) -> Value {
    match outcome {
        DreamPublicationOutcome::NoChange => json!({"outcome": "NoChange"}),
        DreamPublicationOutcome::Withheld(verdict) => json!({
            "outcome": "Withheld",
            "verdict": format!("{verdict:?}"),
        }),
        DreamPublicationOutcome::Published(relations) => json!({
            "outcome": "Published",
            "relations": relations.iter().map(relation_json).collect::<Vec<_>>(),
        }),
    }
}

pub fn durable_snapshot(cva: &mut Cva) -> Result<Value, Box<dyn std::error::Error>> {
    let mut memories = Vec::new();
    let mut titles = HashMap::new();
    for id in cva.memory_ids() {
        let memory = cva.memory(id)?;
        titles.insert(id, memory.title.clone());
        memories.push(memory_json(&memory));
    }
    let relations = cva
        .graph_relations()
        .iter()
        .map(|relation| {
            let mut value = relation_json(relation);
            if let Some(object) = value.as_object_mut() {
                object.insert(
                    "source_title".into(),
                    titles
                        .get(&relation.source)
                        .cloned()
                        .map(Value::String)
                        .unwrap_or(Value::Null),
                );
                object.insert(
                    "target_title".into(),
                    titles
                        .get(&relation.target)
                        .cloned()
                        .map(Value::String)
                        .unwrap_or(Value::Null),
                );
            }
            value
        })
        .collect::<Vec<_>>();
    let remaining_extracted = memories
        .iter()
        .filter(|memory| memory["archived"] == false && memory["lifecycle"] == "extracted")
        .count();
    let graph = cva.graph_stats();
    let memory = cva.memory_stats();
    let vectors = cva.memory_vector_stats();
    Ok(json!({
        "memory_stats": {
            "memories": memory.memories,
            "revisions": memory.revisions,
            "bodies": memory.bodies,
            "memory_version": memory.memory_version,
        },
        "memory_vectors": {
            "objects": vectors.objects,
            "rows": vectors.rows,
            "bindings": vectors.bindings,
        },
        "graph_stats": {
            "nodes": graph.nodes,
            "active_relations": graph.active_relations,
            "relation_mutations": graph.relation_mutations,
            "graph_version": graph.graph_version,
        },
        "remaining_extracted": remaining_extracted,
        "memories": memories,
        "relations": relations,
    }))
}

fn relation_json(relation: &GraphRelation) -> Value {
    json!({
        "source": id_hex(relation.source),
        "target": id_hex(relation.target),
        "kind": format!("{:?}", relation.kind),
        "active": relation.active,
        "global_version": relation.global_version,
        "graph_version": relation.graph_version,
    })
}
