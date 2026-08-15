use serde_json::{Value, json};

pub const INSOMNIA_EXTRACTOR_CONTRACT_VERSION: &str = "v2-2";
pub const MAX_INSOMNIA_CANDIDATES: usize = 64;

pub const INSOMNIA_SYSTEM_PROMPT: &str = r#"You extract durable working memories from one authoritative Continuity episode.
Return only the structured extraction object. Always include both candidates and evidence_requests arrays; use an empty evidence_requests array when no archive evidence is needed.

Evidence requests are read-only and exist only to resolve explicit callbacks, adopted prior assistant content, or narrow historical context needed to interpret the authoritative episode. Use at most four requests and at most one evidence round. Never use archive search to discover unrelated memories.
Evidence request kinds are:
- turn: set conversation_id and node_id; leave start_node_id, end_node_id, and query empty and limit 0.
- conversation_range: set conversation_id, start_node_id, and end_node_id for one ancestry path; leave node_id and query empty and limit 0.
- archive_search: set query and limit (1-5); conversation_id may optionally narrow the search; leave node_id, start_node_id, and end_node_id empty.
After read-only archive evidence is supplied, return final candidates and an empty evidence_requests array. A second evidence round is forbidden.

Each candidate must contain category, type, title, content, source_node_id, source_quote, content_source_conversation_id, content_source_node_id, and content_source_quote.
category must be exactly one of: fact, preference, decision, instruction, relationship, constraint, correction, commitment.
type must be exactly one of: identity, education, employment, location, possession, health, finance, schedule, communication, project, process, product, relationship, other.

Extract only durable facts, preferences, decisions, instructions, relationships, constraints, corrections, and unresolved commitments attributable to the user or explicitly adopted by the user. Optimize for durable information density rather than sentence-level atomicity. Consolidate tightly related details that share authority and lifecycle. Omit acknowledgments, execution chatter, test/commit receipts, prompt numbering, and transient resolved state unless needed to explain a continuing constraint, diagnosis, dependency, or future action.

source_node_id MUST identify a user turn inside the authoritative episode that states, adopts, confirms, corrects, selects, or explicitly asks to retain the candidate. source_quote MUST be a verbatim contiguous substring of that user turn. Read-only archive evidence can never supply source_node_id or independent user authority.

When a user turn explicitly adopts assistant-authored content, content_source_conversation_id and content_source_node_id identify that earlier assistant turn and content_source_quote is a verbatim contiguous substring of it. The assistant content may come from the authoritative episode or from archive evidence supplied during the one evidence round. Otherwise all three content-source fields MUST be empty strings. Never treat assistant content as user-authorized merely because it appears in archive evidence.

Explicit imperative retention is a strong requirement. An identifiable imperative such as "remember this", "remember that", "keep this in mind", or an equivalent instruction MUST produce durable retention of its referent even when that referent would otherwise appear transient. When the user directly stated the remembered content earlier in the authoritative episode, anchor source_node_id to that direct user statement when possible rather than the later bare reminder. When the referent is genuinely assistant-authored, the retention instruction is the user authority and the assistant turn is the content source. Interrogative forms such as "Do you remember that?", "Do you remember this?", "Remember that?", or "Remember this?" are questions and MUST NOT be treated as retention instructions.

Do not infer beyond the source. Return zero candidates when nothing durable is justified. Do not manufacture a candidate merely because the episode was scheduled by create_memory; create_memory changes processing urgency, not source truth."#;

pub fn insomnia_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "candidates": {
                "type": "array",
                "maxItems": MAX_INSOMNIA_CANDIDATES,
                "items": candidate_schema()
            },
            "evidence_requests": {
                "type": "array",
                "maxItems": 4,
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
            }
        },
        "required": ["candidates", "evidence_requests"]
    })
}

fn candidate_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "category": {"type": "string", "enum": ["fact", "preference", "decision", "instruction", "relationship", "constraint", "correction", "commitment"]},
            "type": {"type": "string", "enum": ["identity", "education", "employment", "location", "possession", "health", "finance", "schedule", "communication", "project", "process", "product", "relationship", "other"]},
            "title": {"type": "string"},
            "content": {"type": "string"},
            "source_node_id": {"type": "string"},
            "source_quote": {"type": "string"},
            "content_source_conversation_id": {"type": "string"},
            "content_source_node_id": {"type": "string"},
            "content_source_quote": {"type": "string"}
        },
        "required": ["category", "type", "title", "content", "source_node_id", "source_quote", "content_source_conversation_id", "content_source_node_id", "content_source_quote"]
    })
}
