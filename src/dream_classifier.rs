use crate::dream_classifier_schema::{DREAM_CLASSIFIER_SYSTEM_PROMPT, dream_classifier_schema};
use crate::{
    DreamCandidateSet, DreamClassificationError, DreamEvidenceSide, DreamMemoryContext,
    DreamPairClassification, DreamPairEvidence, DreamRelationDirection, DreamRelationKind,
    GeneralEndpoint, GraphRelationKind,
};
use serde_json::{Value, json};

pub struct DreamClassifier<E> {
    endpoint: E,
}

impl<E: GeneralEndpoint> DreamClassifier<E> {
    pub fn new(endpoint: E) -> Self {
        Self { endpoint }
    }

    pub fn model(&self) -> &str {
        self.endpoint.model()
    }

    pub fn classify_candidates(
        &self,
        set: &DreamCandidateSet,
    ) -> Result<Vec<DreamPairClassification>, DreamClassificationError> {
        set.candidates
            .iter()
            .map(|candidate| self.classify_pair(&set.source, &candidate.context))
            .collect()
    }

    pub fn classify_pair(
        &self,
        left: &DreamMemoryContext,
        right: &DreamMemoryContext,
    ) -> Result<DreamPairClassification, DreamClassificationError> {
        let (a, b) = canonical_pair(left, right)?;
        let payload = serde_json::to_string(&json!({
            "contract_version": crate::DREAM_CLASSIFIER_CONTRACT_VERSION,
            "a": context_json(a),
            "b": context_json(b),
        }))
        .map_err(|error| DreamClassificationError::InvalidOutput(error.to_string()))?;
        let output = self.endpoint.complete_json(
            DREAM_CLASSIFIER_SYSTEM_PROMPT,
            &payload,
            "dream_pair_classification",
            &dream_classifier_schema(),
        )?;
        parse_output(self.endpoint.model(), a, b, &output)
    }
}

fn canonical_pair<'a>(
    left: &'a DreamMemoryContext,
    right: &'a DreamMemoryContext,
) -> Result<(&'a DreamMemoryContext, &'a DreamMemoryContext), DreamClassificationError> {
    if left.memory.id == right.memory.id {
        return Err(DreamClassificationError::InvalidPair);
    }
    Ok(if left.memory.id.0 < right.memory.id.0 {
        (left, right)
    } else {
        (right, left)
    })
}

fn context_json(context: &DreamMemoryContext) -> Value {
    let relations: Vec<_> = context
        .graph_relations
        .iter()
        .map(|relation| {
            json!({
                "source": hex(&relation.source.0),
                "target": hex(&relation.target.0),
                "kind": relation_name(relation.kind),
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

fn parse_output(
    model: &str,
    a: &DreamMemoryContext,
    b: &DreamMemoryContext,
    output: &Value,
) -> Result<DreamPairClassification, DreamClassificationError> {
    let relation = parse_relation(required_string(output, "relation")?)?;
    let direction = parse_direction(required_string(output, "direction")?)?;
    validate_relation_direction(relation, direction)?;
    let evidence = parse_evidence(output, relation, a, b)?;
    Ok(DreamPairClassification {
        model: model.to_owned(),
        a: a.memory.id,
        b: b.memory.id,
        relation,
        direction,
        evidence,
    })
}

fn parse_evidence(
    output: &Value,
    relation: DreamRelationKind,
    a: &DreamMemoryContext,
    b: &DreamMemoryContext,
) -> Result<Vec<DreamPairEvidence>, DreamClassificationError> {
    let values = output
        .get("evidence")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("evidence must be an array"))?;
    if relation == DreamRelationKind::None {
        return if values.is_empty() {
            Ok(Vec::new())
        } else {
            Err(invalid("none must not include evidence"))
        };
    }
    if values.len() != 2 {
        return Err(invalid(
            "non-none relation requires exactly two evidence quotes",
        ));
    }
    let mut parsed = Vec::with_capacity(2);
    for value in values {
        let side = match required_string(value, "side")? {
            "a" => DreamEvidenceSide::A,
            "b" => DreamEvidenceSide::B,
            _ => return Err(invalid("invalid evidence side")),
        };
        if parsed
            .iter()
            .any(|item: &DreamPairEvidence| item.side == side)
        {
            return Err(invalid("evidence must contain one quote per side"));
        }
        let quote = required_string(value, "quote")?.trim().to_owned();
        let context = if side == DreamEvidenceSide::A { a } else { b };
        if quote.is_empty() || !memory_text(context).contains(&quote) {
            return Err(invalid("evidence quote is not verbatim Memory text"));
        }
        parsed.push(DreamPairEvidence { side, quote });
    }
    if parsed.len() != 2 {
        return Err(invalid("evidence must contain A and B"));
    }
    Ok(parsed)
}

fn validate_relation_direction(
    relation: DreamRelationKind,
    direction: DreamRelationDirection,
) -> Result<(), DreamClassificationError> {
    let valid = match relation {
        DreamRelationKind::None => direction == DreamRelationDirection::None,
        DreamRelationKind::Topical
        | DreamRelationKind::Recurrent
        | DreamRelationKind::DuplicateOf => direction == DreamRelationDirection::Undirected,
        DreamRelationKind::Factual | DreamRelationKind::Causal | DreamRelationKind::Supersedes => {
            matches!(
                direction,
                DreamRelationDirection::AToB | DreamRelationDirection::BToA
            )
        }
    };
    valid
        .then_some(())
        .ok_or_else(|| invalid("relation/direction mismatch"))
}

fn parse_relation(value: &str) -> Result<DreamRelationKind, DreamClassificationError> {
    Ok(match value {
        "none" => DreamRelationKind::None,
        "topical" => DreamRelationKind::Topical,
        "factual" => DreamRelationKind::Factual,
        "causal" => DreamRelationKind::Causal,
        "recurrent" => DreamRelationKind::Recurrent,
        "duplicate_of" => DreamRelationKind::DuplicateOf,
        "supersedes" => DreamRelationKind::Supersedes,
        _ => return Err(invalid("unknown relation")),
    })
}

fn parse_direction(value: &str) -> Result<DreamRelationDirection, DreamClassificationError> {
    Ok(match value {
        "none" => DreamRelationDirection::None,
        "undirected" => DreamRelationDirection::Undirected,
        "a_to_b" => DreamRelationDirection::AToB,
        "b_to_a" => DreamRelationDirection::BToA,
        _ => return Err(invalid("unknown direction")),
    })
}

fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, DreamClassificationError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(&format!("missing string field {key}")))
}

fn memory_text(context: &DreamMemoryContext) -> String {
    format!("{}\n{}", context.memory.title, context.memory.content)
}

fn relation_name(kind: GraphRelationKind) -> &'static str {
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

fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn invalid(message: &str) -> DreamClassificationError {
    DreamClassificationError::InvalidOutput(message.into())
}
