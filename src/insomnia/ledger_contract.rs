use super::contract::MAX_INSOMNIA_CANDIDATES;
use serde_json::{Value, json};

pub const INSOMNIA_LEDGER_CONTRACT_VERSION: &str = "v2-9-ledger";

pub const INSOMNIA_DISPOSITION_SYSTEM_PROMPT: &str = r#"Build a complete authority/disposition ledger for one authoritative conversation episode.
Do not write polished Memories. First decide what every user turn actually authorizes.
Return every user turn exactly once, in source order, even when it contains nothing durable. Every returned user turn must contain at least one clause/unit.
Split a user turn only when its clauses genuinely require different dispositions, authority sources, lifecycle/modality, categories, or types. Do NOT atomize one coherent retained design/decision into a separate unit for every implementation detail. Clauses that belong to the same durable proposition should remain one retained unit. Each source_quote must be an exact contiguous quote from that user turn.
Each unit must be retain or omit. Retain only durable user-authoritative facts, preferences, decisions, instructions, relationships, constraints, corrections, commitments, current state, plans, unresolved state, or historically useful state. Omit acknowledgments, chatter, questions that do not assert a proposition, transient execution requests, execution receipts, temporary phase/prompt numbering, candidate/example values, and implementation scaffolding that is only useful to execute the current task rather than to preserve future state.
For retain units, proposition must be a concise normalized statement containing the complete durable meaning of only that unit. It must not import claims from neighboring turns except through an explicitly permitted assistant authority source or referent-only grounding source.
For omit units, proposition must be empty and authority_kind/category/type must be none.
authority_kind must be direct, correction, adoption, retention, or none. Use adoption only when the user explicitly accepts/selects/directs action on assistant-authored content. Use retention when the user explicitly asks to retain a proposition. Use direct/correction when the user's own words state or correct the proposition.
authority_source_* is only for earlier assistant-authored propositions explicitly adopted or retained by the user. grounding_source_* is only for resolving a referent/identity in the user's own proposition. Grounding may never add semantic detail. Do not attach grounding merely because neighboring assistant text explains, confirms, or elaborates a self-contained user assertion; if the user proposition is understandable on its own, grounding must be empty.
lifecycle must be standing, current, planned, unresolved, historical, execution_only, transient, or non_authoritative. Retained units may use only standing/current/planned/unresolved/historical.
A confirmation tag does not automatically make a clause non-authoritative. For example, "collision has to stay server-authoritative, right?" contains an asserted constraint and should retain that assertion. Conversely, "Should collision be server-authoritative?" is only a question and should be omitted.
Execution receipts never authorize the contents of completed work. "I've completed through 41" is execution_only and must not become a Memory about what Prompt 41 contained.
Mixed receipt/status turns must be split. In "Up to prompt 72 completed and everything seems to be working OK so far. Some visual bugs remain", omit the completion clause but retain the independently asserted current state.
Short deictic adoption is meaningful. If the assistant proposes a shared C/C++ adapter and the user says "Fuckit, add that", retain an adoption and cite the exact earlier assistant authority source. Likewise, short confirmations such as "alright, that works", "good rule", "do that then", and "get it done" are adoption when they clearly accept the immediately preceding concrete assistant proposition; do not discard them merely for being short.
A turn such as "custom room codes aren't working right now, that's not major is it?" contains a retainable current-state assertion and a separate non-authoritative priority question; do not infer priority.
A user recalling or questioning an earlier plan does not re-authorize that plan. For example, "I thought we were gonna do this in Godot first... now you're saying server first?" is a question/plan-conflict report and must be omitted unless the same user turn separately commits to a direction. A bare renumbering such as "phase 4, which is now phase 3" is transient metadata and must be omitted.
Pasted or quoted agent/assistant material inside a user message is context, not automatically one user-authoritative proposition per quoted bullet. If the user asks only to review/revise/fold in that material, retain the durable design or constraints the user actually endorses, summarized coherently; do not promote every example value, method name, prompt number, or temporary implementation step into independent durable state. However, explicit first-person framing such as "we need", "the plan is", "need to preserve", or a direct repetition/endorsement of the plan is user authority for the governing requirements. In a long turn containing an authoritative architecture/requirements block plus a trailing implementation preference, retain the governing architecture as well as any independently durable trailing preference; do not keep only the last sentence.
Prefer final authority within the episode. When a later user-authoritative turn replaces an earlier implementation/design, omit the superseded predecessor rather than preserving overlapping current-state Memories from both. Preserve an older statement as historical only when the historical fact itself is independently useful and clearly scoped as historical.
Evidence requests are read-only and may be used only for an explicit callback/adoption/retention or genuinely unresolved referent after exhausting the episode and nearby ancestry. Use at most four narrow requests. Archive evidence never supplies user authority.
Always return both turns and evidence_requests. Return an empty evidence_requests array when none is needed."#;

pub const INSOMNIA_SYNTHESIS_SYSTEM_PROMPT: &str = r#"Synthesize standalone Memories from an already-authorized retained-unit ledger.
You are not deciding what to retain. Every supplied ledger_id must be covered exactly once across the returned Memories: no drops, duplicates, or invented ids.
Consolidate aggressively when multiple retained units share the same merge_group and form one coherent durable Memory. Never combine units from different merge_group values. Return the smallest coherent set that preserves every authorized proposition without turning temporary implementation mechanics into separate Memories when they can be represented as one governing design.
For each Memory return covered_ledger_ids, category, title, and content. covered_ledger_ids must contain one or more exact supplied ids from one merge_group. category must be one of the categories already present among the covered units; choose the category that best describes the consolidated Memory.
The content must preserve the complete meaning and lifecycle/modality of all covered propositions and add no new claim, rationale, implementation detail, consequence, or scope.
The propositions are authoritative. source quotes are provided only to preserve wording/modality. Do not infer from anything that is not present in the covered retained units.
Keep each Memory concise, standalone, and useful to a future agent. Do not mention ledger ids, prompts, extraction machinery, or provenance metadata in the Memory text."#;

pub fn insomnia_ledger_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "turns": {
                "type": "array",
                "maxItems": 256,
                "items": ledger_turn_schema()
            },
            "evidence_requests": evidence_requests_schema()
        },
        "required": ["turns", "evidence_requests"]
    })
}

pub fn insomnia_synthesis_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "memories": {
                "type": "array",
                "maxItems": MAX_INSOMNIA_CANDIDATES,
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "covered_ledger_ids": {
                            "type": "array",
                            "minItems": 1,
                            "maxItems": MAX_INSOMNIA_CANDIDATES,
                            "items": {"type": "string"}
                        },
                        "category": {"type": "string", "enum": ["fact", "preference", "decision", "instruction", "relationship", "constraint", "correction", "commitment"]},
                        "title": {"type": "string"},
                        "content": {"type": "string"}
                    },
                    "required": ["covered_ledger_ids", "category", "title", "content"]
                }
            }
        },
        "required": ["memories"]
    })
}

fn ledger_turn_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "source_node_id": {"type": "string"},
            "units": {
                "type": "array",
                "minItems": 1,
                "maxItems": 32,
                "items": ledger_unit_schema()
            }
        },
        "required": ["source_node_id", "units"]
    })
}

fn ledger_unit_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "source_quote": {"type": "string"},
            "disposition": {"type": "string", "enum": ["retain", "omit"]},
            "authority_kind": {"type": "string", "enum": ["direct", "correction", "adoption", "retention", "none"]},
            "category": {"type": "string", "enum": ["fact", "preference", "decision", "instruction", "relationship", "constraint", "correction", "commitment", "none"]},
            "type": {"type": "string", "enum": ["identity", "education", "employment", "location", "possession", "health", "finance", "schedule", "communication", "project", "process", "product", "relationship", "other", "none"]},
            "lifecycle": {"type": "string", "enum": ["standing", "current", "planned", "unresolved", "historical", "execution_only", "transient", "non_authoritative"]},
            "proposition": {"type": "string"},
            "reason": {"type": "string"},
            "authority_source_conversation_id": {"type": "string"},
            "authority_source_node_id": {"type": "string"},
            "authority_source_quote": {"type": "string"},
            "grounding_source_conversation_id": {"type": "string"},
            "grounding_source_node_id": {"type": "string"},
            "grounding_source_quote": {"type": "string"}
        },
        "required": ["source_quote", "disposition", "authority_kind", "category", "type", "lifecycle", "proposition", "reason", "authority_source_conversation_id", "authority_source_node_id", "authority_source_quote", "grounding_source_conversation_id", "grounding_source_node_id", "grounding_source_quote"]
    })
}

fn evidence_requests_schema() -> Value {
    json!({
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
    })
}
