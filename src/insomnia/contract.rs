use serde_json::{Value, json};

pub const INSOMNIA_EXTRACTOR_CONTRACT_VERSION: &str = "v2-1";
pub const MAX_INSOMNIA_CANDIDATES: usize = 64;

pub const INSOMNIA_SYSTEM_PROMPT: &str = r#"You extract durable working memories from one authoritative Continuity episode.
Return only the structured extraction object.

Each candidate must contain category, type, title, content, source_node_id, source_quote, content_source_conversation_id, content_source_node_id, and content_source_quote.
category must be exactly one of: fact, preference, decision, instruction, relationship, constraint, correction, commitment.
type must be exactly one of: identity, education, employment, location, possession, health, finance, schedule, communication, project, process, product, relationship, other.

Extract only durable facts, preferences, decisions, instructions, relationships, constraints, corrections, and unresolved commitments attributable to the user or explicitly adopted by the user. Optimize for durable information density rather than sentence-level atomicity. Consolidate tightly related details that share authority and lifecycle. Omit acknowledgments, execution chatter, test/commit receipts, prompt numbering, and transient resolved state unless needed to explain a continuing constraint, diagnosis, dependency, or future action.

source_node_id MUST identify a user turn inside the authoritative episode that states, adopts, confirms, corrects, selects, or explicitly asks to retain the candidate. source_quote MUST be a verbatim contiguous substring of that user turn.

When a user turn explicitly adopts assistant-authored content, content_source_conversation_id and content_source_node_id identify that earlier assistant turn and content_source_quote is a verbatim contiguous substring of it. Otherwise all three content-source fields MUST be empty strings. Never treat assistant content as user-authorized merely because it is present in the episode.

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
                "items": {
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
                }
            }
        },
        "required": ["candidates"]
    })
}
