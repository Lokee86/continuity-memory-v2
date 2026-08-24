use serde_json::{Map, Value, json};

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

pub const SYNTHESIS_PROMPT: &str = r#"You are the wording pass for Continuity Insomnia.
You receive an authoritative episode plus deterministic synthesis groups built from a validated authority/disposition ledger. Every group already has final semantic ownership: source, authority kind, category, type, lifecycle, assistant-authority provenance, grounding provenance, and retained propositions.

Your ONLY job is to write one concise durable Memory title and content body for every supplied group.
Rules:
1. Return exactly one wording result for every required group key. Do not add, drop, merge, split, rename, or reorder groups.
2. Preserve every proposition in the group. You may consolidate wording, but may not omit a retained proposition or add a proposition not present in the group.
3. Preserve modality exactly: current, future, historical, uncertainty, and correction semantics must not drift.
4. Strip conversational/checkpoint phrasing that is not part of the retained propositions. Do not introduce prompt/phase/step numbers, receipts, implementation chatter, rationale, architecture, paths, consequences, or contextual facts absent from the group.
5. Grounding and authority metadata are already final and are not fields you can edit.
6. Keep content compact and durable. The title should identify the remembered state; the body should state the grouped propositions naturally without editorial commentary.

The episode is supplied only to help preserve referent wording when necessary. It is not permission to reinterpret the groups."#;

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

pub fn synthesis_schema(groups: &Value) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();
    for group in groups.as_array().into_iter().flatten() {
        let Some(group_id) = group["group_id"].as_str() else {
            continue;
        };
        required.push(Value::String(group_id.to_owned()));
        properties.insert(
            group_id.to_owned(),
            json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "title": {"type": "string"},
                    "content": {"type": "string"}
                },
                "required": ["title", "content"]
            }),
        );
    }
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "groups": {
                "type": "object",
                "additionalProperties": false,
                "properties": properties,
                "required": required
            }
        },
        "required": ["groups"]
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
