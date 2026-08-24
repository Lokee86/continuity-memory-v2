use super::extraction::{InsomniaEvidenceTurn, InsomniaExtractionError};
use crate::ResolvedTurn;
use serde_json::{Map, Value, json};
use std::collections::HashSet;

pub(super) const LEDGER_SYSTEM_PROMPT: &str = r#"You are the authority-selection pass for Continuity Insomnia.
Given one complete authoritative conversation episode, produce a clause-level ledger. This pass decides WHAT durable user-authoritative state exists. It does not write Memories.
Every user turn MUST be accounted for by at least one clause under its required turn ID. Assistant turns never receive ledger clauses. A mixed user turn may have multiple clauses when one clause is durable and another is only a question, receipt, or transient metadata.

Apply a strict durable-state test before retaining anything: after the immediate exchange/task is over, would this proposition still be useful as user/project state? Do not retain conversational scaffolding merely because it is factually true.

Authority rules:
1. direct: the user turn itself asserts durable state. direct MUST have empty authority_source_node_id.
2. correction: the user explicitly corrects/replaces earlier state. correction MUST have empty authority_source_node_id; earlier context may be grounding only when needed to identify what is being corrected.
3. adoption: the user explicitly accepts/selects/confirms a specific assistant-authored proposition. Terse assent such as "that works", "sounds good", "do it", "go with that", or equivalent CAN be durable adoption when it directly answers a single clear preceding assistant proposal. authority_source_node_id is REQUIRED and identifies that assistant turn.
4. retention: the user explicitly asks to remember/retain a proposition. If the proposition is assistant-authored, authority_source_node_id is required; otherwise it is empty.
5. grounding_source_node_id identifies an otherwise unclear referent only. It never authorizes details absent from the user turn. Do not ground merely because context exists. Do not self-ground. If the proposition is understandable independently, grounding_source_node_id MUST be empty.
6. Omitted clauses MUST use authority_kind/category/type/lifecycle = none, an empty proposition, and empty authority/grounding source IDs.

Disposition rules:
7. Questions and requests are non-authoritative by default. Do not turn a proposed choice, recalled prior plan, challenge, uncertainty, or open/alternative question into a decision/correction merely because a proposition can be inferred from it. "I thought we were going to X; now you're saying Y?" does NOT establish X. A confirmation tag on an independently asserted proposition can retain the assertion: "X has to/must/needs to be Y, right?" may retain the requirement while omitting the request for confirmation.
8. Pure execution/checkpoint receipts are omitted: prompt/phase/step numbers, "completed through N", test/commit/push completion, transient renumbering, and similar workflow position are not durable state by themselves. If the same turn separately states resulting project state, SPLIT it and retain that state without checkpoint framing. A current functional assessment such as "everything seems to be working OK so far" counts as resulting state.
9. Transient troubleshooting artifacts, pasted logs, temporary command output, immediate execution requests, and ephemeral debugging observations are omitted unless the user independently states a durable conclusion/state derived from them.
10. Preserve modality exactly. Future plans/possibilities remain future; uncertainty remains uncertainty; historical events do not become current state.
11. Prefer the FINAL user-authoritative source for overlapping semantic state. When a later authoritative turn restates, refines, or replaces an earlier proposition, mark the earlier overlapping clause superseded. Keep older state only when independently durable and not represented later.
12. Advice, explanations, examples, and assistant proposals are not user state unless explicitly adopted/retained. Do not treat assistant confirmation after a user assertion as authority for that assertion.
13. Keep propositions narrow. Do not infer importance, rationale, implementation detail, architecture, paths, causal explanation, or broader state from nearby context unless the user actually authorizes it.

Evidence rules:
14. Request read-only archive evidence only when necessary to resolve an explicit callback, adopted/retained assistant content, or a genuinely unresolved referent that is not available in the authoritative episode. Exhaust the episode first. Use at most four narrow requests.
15. Archive evidence is supporting context, never independent user authority. source authority MUST remain a user turn in the authoritative episode. Earlier assistant evidence may supply authority_source_node_id only when a current user turn explicitly adopts/retains that assistant proposition. Earlier user or assistant evidence may supply grounding_source_node_id only to resolve a referent.
16. When read_only_archive_evidence is supplied, return the FINAL ledger and an empty evidence_requests array. No second evidence round is allowed.

For retained/superseded clauses, proposition states only the authorized durable state. Continuity owns byte-exact provenance; do not reproduce source quotes. When uncertain whether a clause is durable authority or conversational scaffolding, omit it."#;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct LedgerEntry {
    pub source_node_id: String,
    pub disposition: String,
    pub authority_kind: String,
    pub category: String,
    pub memory_type: String,
    pub lifecycle: String,
    pub proposition: String,
    pub authority_source_node_id: String,
    pub grounding_source_node_id: String,
}

pub(super) fn schema(
    turns: &[ResolvedTurn],
    evidence_turns: &[InsomniaEvidenceTurn],
    allow_evidence_requests: bool,
) -> Value {
    let user_ids: Vec<String> = turns
        .iter()
        .filter(|turn| turn.role == "user")
        .map(|turn| turn.node_id.clone())
        .collect();
    let mut assistant_ids: Vec<String> = turns
        .iter()
        .filter(|turn| turn.role == "assistant")
        .map(|turn| turn.node_id.clone())
        .chain(
            evidence_turns
                .iter()
                .filter(|turn| turn.role == "assistant")
                .map(|turn| turn.node_id.clone()),
        )
        .collect();
    assistant_ids.push(String::new());
    let mut support_ids: Vec<String> = turns
        .iter()
        .map(|turn| turn.node_id.clone())
        .chain(evidence_turns.iter().map(|turn| turn.node_id.clone()))
        .collect();
    support_ids.push(String::new());

    let clause = clause_schema(assistant_ids, support_ids);
    let mut turn_properties = Map::new();
    let mut required_turns = Vec::new();
    for id in user_ids {
        required_turns.push(Value::String(id.clone()));
        turn_properties.insert(
            id,
            json!({"type": "array", "minItems": 1, "maxItems": 16, "items": clause}),
        );
    }
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "turns": {
                "type": "object",
                "additionalProperties": false,
                "properties": turn_properties,
                "required": required_turns
            },
            "evidence_requests": evidence_request_schema(allow_evidence_requests)
        },
        "required": ["turns", "evidence_requests"]
    })
}

pub(super) fn generic_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "turns": {"type": "object"},
            "evidence_requests": evidence_request_schema(true)
        },
        "required": ["turns", "evidence_requests"]
    })
}

pub(super) fn parse(
    value: &Value,
    turns: &[ResolvedTurn],
    evidence_turns: &[InsomniaEvidenceTurn],
) -> Result<Vec<LedgerEntry>, InsomniaExtractionError> {
    let object = value
        .get("turns")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("missing ledger turns object"))?;
    let user_ids: Vec<&str> = turns
        .iter()
        .filter(|turn| turn.role == "user")
        .map(|turn| turn.node_id.as_str())
        .collect();
    let allowed_users: HashSet<&str> = user_ids.iter().copied().collect();
    if object.len() != allowed_users.len()
        || object
            .keys()
            .any(|key| !allowed_users.contains(key.as_str()))
    {
        return Err(invalid(
            "ledger turns object contains a missing or non-user source key",
        ));
    }
    let allowed_assistants: HashSet<&str> = turns
        .iter()
        .filter(|turn| turn.role == "assistant")
        .map(|turn| turn.node_id.as_str())
        .chain(
            evidence_turns
                .iter()
                .filter(|turn| turn.role == "assistant")
                .map(|turn| turn.node_id.as_str()),
        )
        .collect();
    let allowed_support: HashSet<&str> = turns
        .iter()
        .map(|turn| turn.node_id.as_str())
        .chain(evidence_turns.iter().map(|turn| turn.node_id.as_str()))
        .collect();
    let mut entries = Vec::new();
    for source_node_id in user_ids {
        let clauses = object
            .get(source_node_id)
            .and_then(Value::as_array)
            .ok_or_else(|| invalid(format!("ledger omitted user turn {source_node_id}")))?;
        if clauses.is_empty() {
            return Err(invalid(format!(
                "ledger emitted no clauses for {source_node_id}"
            )));
        }
        for clause in clauses {
            entries.push(parse_clause(
                source_node_id,
                clause,
                &allowed_assistants,
                &allowed_support,
            )?);
        }
    }
    Ok(entries)
}

fn parse_clause(
    source_node_id: &str,
    value: &Value,
    allowed_assistants: &HashSet<&str>,
    allowed_support: &HashSet<&str>,
) -> Result<LedgerEntry, InsomniaExtractionError> {
    let disposition = string(value, "disposition")?;
    if !["retain", "omit", "superseded"].contains(&disposition.as_str()) {
        return Err(invalid("ledger disposition is not recognized"));
    }
    if disposition == "omit" {
        return Ok(LedgerEntry {
            source_node_id: source_node_id.to_owned(),
            disposition,
            authority_kind: "none".into(),
            category: "none".into(),
            memory_type: "none".into(),
            lifecycle: "none".into(),
            proposition: String::new(),
            authority_source_node_id: String::new(),
            grounding_source_node_id: String::new(),
        });
    }
    let authority_kind = string(value, "authority_kind")?;
    let category = string(value, "category")?;
    let memory_type = string(value, "type")?;
    let lifecycle = string(value, "lifecycle")?;
    let proposition = string(value, "proposition")?;
    let mut authority_source_node_id = string(value, "authority_source_node_id")?;
    let mut grounding_source_node_id = string(value, "grounding_source_node_id")?;
    if !["direct", "correction", "adoption", "retention"].contains(&authority_kind.as_str()) {
        return Err(invalid("ledger authority kind is not recognized"));
    }
    if !CATEGORIES.contains(&category.as_str()) || !TYPES.contains(&memory_type.as_str()) {
        return Err(invalid("ledger classification is not recognized"));
    }
    if !["current", "future", "historical", "superseded"].contains(&lifecycle.as_str()) {
        return Err(invalid("ledger lifecycle is not recognized"));
    }
    if proposition.trim().is_empty() {
        return Err(invalid("retained ledger proposition is empty"));
    }
    if matches!(authority_kind.as_str(), "direct" | "correction") {
        authority_source_node_id.clear();
    }
    if authority_kind == "adoption" && authority_source_node_id.is_empty() {
        return Err(invalid("ledger adoption lacks assistant authority source"));
    }
    if !authority_source_node_id.is_empty()
        && !allowed_assistants.contains(authority_source_node_id.as_str())
    {
        return Err(invalid(
            "ledger authority source is not an available assistant turn",
        ));
    }
    if grounding_source_node_id == source_node_id {
        grounding_source_node_id.clear();
    }
    if !grounding_source_node_id.is_empty()
        && !allowed_support.contains(grounding_source_node_id.as_str())
    {
        return Err(invalid("ledger grounding source is not available evidence"));
    }
    Ok(LedgerEntry {
        source_node_id: source_node_id.to_owned(),
        disposition,
        authority_kind,
        category,
        memory_type,
        lifecycle,
        proposition: proposition.trim().to_owned(),
        authority_source_node_id,
        grounding_source_node_id,
    })
}

fn clause_schema(assistant_ids: Vec<String>, support_ids: Vec<String>) -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "disposition": {"type": "string", "enum": ["retain", "omit", "superseded"]},
            "authority_kind": {"type": "string", "enum": ["direct", "correction", "adoption", "retention", "none"]},
            "category": {"type": "string", "enum": ["fact", "preference", "decision", "instruction", "relationship", "constraint", "correction", "commitment", "none"]},
            "type": {"type": "string", "enum": ["identity", "education", "employment", "location", "possession", "health", "finance", "schedule", "communication", "project", "process", "product", "relationship", "other", "none"]},
            "lifecycle": {"type": "string", "enum": ["current", "future", "historical", "superseded", "none"]},
            "proposition": {"type": "string"},
            "authority_source_node_id": {"type": "string", "enum": assistant_ids},
            "grounding_source_node_id": {"type": "string", "enum": support_ids},
            "reason": {"type": "string"}
        },
        "required": ["disposition", "authority_kind", "category", "type", "lifecycle", "proposition", "authority_source_node_id", "grounding_source_node_id", "reason"]
    })
}

fn evidence_request_schema(allow: bool) -> Value {
    json!({
        "type": "array",
        "maxItems": if allow { 4 } else { 0 },
        "items": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "kind": {"type": "string", "enum": ["turn", "conversation_range", "archive_search"]},
                "conversation_id": {"type": "string"},
                "node_id": {"type": "string"},
                "start_node_id": {"type": "string"},
                "end_node_id": {"type": "string"},
                "query": {"type": "string"},
                "limit": {"type": "integer", "minimum": 0, "maximum": 5}
            },
            "required": ["kind", "conversation_id", "node_id", "start_node_id", "end_node_id", "query", "limit"]
        }
    })
}

fn string(value: &Value, key: &str) -> Result<String, InsomniaExtractionError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(|value| value.trim().to_owned())
        .ok_or_else(|| invalid(format!("ledger field {key} is missing")))
}

fn invalid(message: impl Into<String>) -> InsomniaExtractionError {
    InsomniaExtractionError::InvalidOutput(message.into())
}

const CATEGORIES: &[&str] = &[
    "fact",
    "preference",
    "decision",
    "instruction",
    "relationship",
    "constraint",
    "correction",
    "commitment",
];
const TYPES: &[&str] = &[
    "identity",
    "education",
    "employment",
    "location",
    "possession",
    "health",
    "finance",
    "schedule",
    "communication",
    "project",
    "process",
    "product",
    "relationship",
    "other",
];
