use serde_json::{Value, json};

pub const LEDGER_PROMPT: &str = r#"You are the authority-selection pass for Continuity Insomnia.
Given one complete authoritative conversation episode, produce a clause-level ledger. This pass decides WHAT durable user-authoritative state exists. It does not write Memories.
Every user turn MUST be accounted for by at least one entry, in source order. Assistant turns never receive ledger entries. A mixed user turn may have multiple entries when one clause is durable and another is only a question, receipt, or transient metadata.

Apply a strict durable-state test before retaining anything: after the immediate exchange/task is over, would this proposition still be useful as user/project state? Do not retain conversational scaffolding merely because it is factually true.

Authority rules:
1. direct: the user turn itself asserts durable state. direct MUST have empty authority_source_node_id.
2. correction: the user explicitly corrects/replaces earlier state. correction MUST have empty authority_source_node_id; earlier context may be grounding only when needed to identify what is being corrected.
3. adoption: the user explicitly accepts/selects/confirms a specific assistant-authored proposition. Terse assent such as "that works", "sounds good", "do it", "go with that", or equivalent CAN be durable adoption when it directly answers a single clear preceding assistant proposal; the user does not need to restate the adopted proposition. authority_source_node_id is REQUIRED and must identify the earlier assistant turn containing that proposition.
4. retention: the user explicitly asks to remember/retain a proposition. If the proposition is assistant-authored, authority_source_node_id is required; otherwise it is empty.
5. Grounding identifies an otherwise unclear referent only. It never authorizes details absent from the user turn. Do NOT use grounding merely because context exists. Do NOT ground a turn to itself. If the user proposition is understandable without another turn, grounding_source_node_id MUST be empty. Do not duplicate an adoption authority source as grounding unless a separate unresolved referent genuinely requires it.
6. Omitted entries MUST have authority_source_node_id and grounding_source_node_id empty.

Disposition rules:
7. Questions and requests are non-authoritative by default. Do not turn a proposed choice, recalled prior plan, challenge, uncertainty, or open/alternative question into a decision/correction merely because a proposition can be inferred from it. In particular, "I thought we were going to X; now you're saying Y?" reports uncertainty about prior/current plans; it does NOT establish X. However, a confirmation-seeking tag appended to an independently asserted proposition can still carry authority: clauses such as "X has to/must/needs to be Y, right?" may retain the asserted requirement/constraint while omitting only the request for confirmation. The test is whether the clause grammatically states the user's own present fact/requirement/preference/constraint before the tag, rather than merely asking which option is true. A declarative current-state premise may likewise be retained while its accompanying question is omitted.
8. Pure execution/checkpoint receipts are omitted even when they accurately report progress: prompt/phase/step numbers, "completed through N", "done with prompt N", test/commit/push completion, transient renumbering, and similar workflow position are not durable state by themselves. Do not reinterpret checkpoint position as a durable project fact. If the same turn separately states resulting project state, SPLIT it and retain that state without checkpoint framing. Resulting state includes not only concrete feature/file/bug changes but also an independent current functional assessment such as "everything seems to be working OK so far". A checkpoint prefix does not contaminate a separately asserted project-status clause.
9. Transient troubleshooting artifacts, pasted logs, temporary command output, immediate execution requests, and ephemeral debugging observations are omitted unless the user independently states a durable conclusion/state derived from them.
10. Preserve modality exactly. Future plans/possibilities remain future commitments; uncertainty remains uncertainty; historical events do not become current state.
11. Prefer the FINAL user-authoritative source for overlapping semantic state. When a later authoritative turn restates, refines, or replaces an earlier proposition, mark the earlier overlapping clause superseded rather than retaining a duplicate/high-level invariant from the older source. Keep older state only when it is independently durable and not represented by the later authoritative state.
12. Advice, explanations, examples, and assistant proposals are not user state unless explicitly adopted/retained. Do not treat assistant confirmation after a user assertion as authority for that assertion.
13. Keep propositions narrow. Do not infer importance, rationale, implementation detail, architecture, path, causal explanation, or broader state from nearby context unless the user actually authorizes it. Grounding may resolve "it/that/they/the project" but may not add facts.

For retain/superseded entries, source_quote must be an exact contiguous quote from the source user turn. For a wholly omitted turn, source_quote may be empty. For omitted entries use authority_kind/category/type/lifecycle = none and proposition = empty. Completeness is more important than brevity in this pass; retention count is not. When uncertain whether a clause is durable authority or conversational scaffolding, omit it unless the user clearly states durable state."#;

pub const SYNTHESIS_PROMPT: &str = r#"You are the synthesis pass for Continuity Insomnia.
You receive an authoritative episode plus a validated authority/disposition ledger. The ledger is the semantic gate. Write durable Memories ONLY from ledger entries whose disposition is retain. Never resurrect omit or superseded entries, and never add a proposition that the ledger did not authorize.
Rules:
1. source_node_id is the user authority turn from a retained ledger entry. source_quote is an exact contiguous quote from that turn.
2. direct/correction candidates must not use assistant authority provenance. adoption requires the earlier assistant authority source. retention uses assistant authority provenance only when the retained proposition came from assistant-authored content.
3. Grounding may resolve a referent only and must not expand semantic content.
4. Optimize for durable information density. Consolidate tightly related retained clauses from the SAME user authority turn when they share authority/lifecycle and belong together. Do not merge unrelated state.
5. Preserve current/future/historical modality and corrections. Do not include superseded state as current.
6. Strip execution receipts, numbered prompt/phase/step progress, transient implementation chatter, and unsupported contextual detail.
7. Keep narrow propositions narrow. Do not infer importance, rationale, architecture, paths, or consequences that the retained ledger entry did not authorize.
8. Produce zero candidates when the ledger contains no retained entries.
9. Use empty strings for unused provenance fields.
10. Copy authority/grounding source quotes exactly from the referenced episode turn, and use the episode conversation_id for matching source conversation fields.
The episode is supplied so you can copy exact quotes and supporting source text. It is not permission to override the ledger."#;

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

pub fn ledger_schema(episode: &Value) -> Value {
    let user_ids = ids_for_role(episode, "user");
    let assistant_ids = with_empty(ids_for_role(episode, "assistant"));
    let all_ids = with_empty(all_ids(episode));
    json!({
        "type": "object", "additionalProperties": false,
        "properties": {"entries": {
            "type": "array", "maxItems": 128,
            "items": {
                "type": "object", "additionalProperties": false,
                "properties": {
                    "source_node_id": {"type": "string", "enum": user_ids},
                    "source_quote": {"type": "string"},
                    "disposition": {"type": "string", "enum": ["retain", "omit", "superseded"]},
                    "authority_kind": {"type": "string", "enum": ["direct", "correction", "adoption", "retention", "none"]},
                    "category": {"type": "string", "enum": enum_with_none(CATEGORIES)},
                    "type": {"type": "string", "enum": enum_with_none(TYPES)},
                    "lifecycle": {"type": "string", "enum": ["current", "future", "historical", "superseded", "none"]},
                    "proposition": {"type": "string"},
                    "authority_source_node_id": {"type": "string", "enum": assistant_ids},
                    "grounding_source_node_id": {"type": "string", "enum": all_ids},
                    "reason": {"type": "string"}
                },
                "required": ["source_node_id", "source_quote", "disposition", "authority_kind", "category", "type", "lifecycle", "proposition", "authority_source_node_id", "grounding_source_node_id", "reason"]
            }
        }},
        "required": ["entries"]
    })
}

pub fn synthesis_schema(episode: &Value, ledger: &Value) -> Value {
    let retained: Vec<String> = ledger["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|entry| entry["disposition"] == "retain")
        .filter_map(|entry| entry["source_node_id"].as_str().map(str::to_owned))
        .collect();
    let source_ids = if retained.is_empty() {
        vec![String::new()]
    } else {
        retained
    };
    let max_items = if source_ids == [String::new()] { 0 } else { 64 };
    let assistant_ids = with_empty(ids_for_role(episode, "assistant"));
    let all_ids = with_empty(all_ids(episode));
    json!({
        "type": "object", "additionalProperties": false,
        "properties": {"candidates": {
            "type": "array", "maxItems": max_items,
            "items": {
                "type": "object", "additionalProperties": false,
                "properties": {
                    "authority_kind": {"type": "string", "enum": ["direct", "correction", "adoption", "retention"]},
                    "category": {"type": "string", "enum": CATEGORIES},
                    "type": {"type": "string", "enum": TYPES},
                    "title": {"type": "string"}, "content": {"type": "string"},
                    "source_node_id": {"type": "string", "enum": source_ids},
                    "source_quote": {"type": "string"},
                    "authority_source_conversation_id": {"type": "string"},
                    "authority_source_node_id": {"type": "string", "enum": assistant_ids},
                    "authority_source_quote": {"type": "string"},
                    "grounding_source_conversation_id": {"type": "string"},
                    "grounding_source_node_id": {"type": "string", "enum": all_ids},
                    "grounding_source_quote": {"type": "string"}
                },
                "required": ["authority_kind", "category", "type", "title", "content", "source_node_id", "source_quote", "authority_source_conversation_id", "authority_source_node_id", "authority_source_quote", "grounding_source_conversation_id", "grounding_source_node_id", "grounding_source_quote"]
            }
        }},
        "required": ["candidates"]
    })
}

fn ids_for_role(episode: &Value, role: &str) -> Vec<String> {
    episode["turns"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|turn| turn["role"] == role)
        .filter_map(|turn| turn["id"].as_str().map(str::to_owned))
        .collect()
}
fn all_ids(episode: &Value) -> Vec<String> {
    episode["turns"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|turn| turn["id"].as_str().map(str::to_owned))
        .collect()
}
fn with_empty(mut values: Vec<String>) -> Vec<String> {
    values.push(String::new());
    values
}
fn enum_with_none(values: &[&str]) -> Vec<String> {
    values
        .iter()
        .copied()
        .chain(["none"])
        .map(str::to_owned)
        .collect()
}
