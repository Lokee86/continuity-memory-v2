use crate::{
    DreamMemoryContext, DreamTemporalFrequency, DreamTemporalGranularity, DreamTemporalOrigin,
    DreamTemporalWeekday, GraphRelationKind,
};
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
        "temporal": temporal_json(context),
        "graph_relations": relations,
    })
}

fn temporal_json(context: &DreamMemoryContext) -> Value {
    let anchors: Vec<_> = context
        .temporal
        .anchors
        .iter()
        .map(|anchor| {
            json!({
                "start_ns": anchor.start_ns,
                "end_ns": anchor.end_ns,
                "granularity": granularity_name(anchor.granularity),
                "origin": origin_name(anchor.origin),
                "evidence": anchor.evidence,
            })
        })
        .collect();
    let patterns: Vec<_> = context
        .temporal
        .patterns
        .iter()
        .map(|pattern| {
            json!({
                "frequency": frequency_name(pattern.frequency),
                "interval": pattern.interval,
                "weekday": pattern.weekday.map(weekday_name),
                "month_day": pattern.month_day,
                "month": pattern.month,
                "evidence": pattern.evidence,
            })
        })
        .collect();
    json!({"anchors": anchors, "patterns": patterns})
}

pub(crate) fn memory_text(context: &DreamMemoryContext) -> String {
    format!("{}\n{}", context.memory.title, context.memory.content)
}

pub(crate) fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn granularity_name(value: DreamTemporalGranularity) -> &'static str {
    match value {
        DreamTemporalGranularity::Instant => "instant",
        DreamTemporalGranularity::Day => "day",
        DreamTemporalGranularity::Week => "week",
        DreamTemporalGranularity::Month => "month",
        DreamTemporalGranularity::Quarter => "quarter",
        DreamTemporalGranularity::Year => "year",
        DreamTemporalGranularity::Range => "range",
    }
}

fn origin_name(value: DreamTemporalOrigin) -> &'static str {
    match value {
        DreamTemporalOrigin::Explicit => "explicit",
        DreamTemporalOrigin::Relative => "relative",
    }
}

fn frequency_name(value: DreamTemporalFrequency) -> &'static str {
    match value {
        DreamTemporalFrequency::Daily => "daily",
        DreamTemporalFrequency::Weekly => "weekly",
        DreamTemporalFrequency::Monthly => "monthly",
        DreamTemporalFrequency::Quarterly => "quarterly",
        DreamTemporalFrequency::Yearly => "yearly",
    }
}

fn weekday_name(value: DreamTemporalWeekday) -> &'static str {
    match value {
        DreamTemporalWeekday::Monday => "monday",
        DreamTemporalWeekday::Tuesday => "tuesday",
        DreamTemporalWeekday::Wednesday => "wednesday",
        DreamTemporalWeekday::Thursday => "thursday",
        DreamTemporalWeekday::Friday => "friday",
        DreamTemporalWeekday::Saturday => "saturday",
        DreamTemporalWeekday::Sunday => "sunday",
    }
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
