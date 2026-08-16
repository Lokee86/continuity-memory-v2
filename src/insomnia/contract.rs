use serde_json::{Value, json};

pub const INSOMNIA_EXTRACTOR_CONTRACT_VERSION: &str = "v2-7";
pub const MAX_INSOMNIA_CANDIDATES: usize = 64;

pub const INSOMNIA_SYSTEM_PROMPT: &str = r#"You extract durable working memories from one authoritative conversation episode.
Return the structured extraction object. Always include both "candidates" and "evidence_requests" arrays; use an empty evidence_requests array when no evidence is needed.
Evidence requests are read-only and may use exactly one of these forms: {"kind":"turn","conversation_id":"...","node_id":"...","start_node_id":"","end_node_id":"","query":"","limit":0}, {"kind":"conversation_range","conversation_id":"...","node_id":"","start_node_id":"...","end_node_id":"...","query":"","limit":0}, {"kind":"archive_search","conversation_id":"","node_id":"","start_node_id":"","end_node_id":"","query":"...","limit":3}, or the same archive_search form with an optional real conversation_id.
Request evidence only when it is necessary to resolve an explicit callback, adopted/retained assistant content, or a genuinely unresolved referent. Exhaust the authoritative episode and nearby ancestry first. Use at most four narrow requests. Do not use archive search to discover unrelated memories.
Each candidate must contain: authority_kind, category, type, title, content, source_node_id, source_quote, authority_source_conversation_id, authority_source_node_id, authority_source_quote, grounding_source_conversation_id, grounding_source_node_id, and grounding_source_quote. Do not return a key; Continuity derives it deterministically.
authority_kind must be exactly one of: direct, correction, adoption, retention.
Use direct when the user turn itself states the durable proposition. Use correction when the user turn corrects or replaces an earlier claim. Use adoption when the user explicitly accepts, selects, confirms, or directs action on assistant-authored content. Use retention when the user explicitly asks to remember or retain a proposition.
source_node_id and source_quote identify the user turn that states, adopts, confirms, corrects, selects, or instructs the system to retain the memory.
authority_source_* identifies earlier assistant-authored content whose proposition the user explicitly adopts or asks to retain. Use it only when that assistant content supplies semantic authority. grounding_source_* identifies an earlier user or assistant turn used only to resolve a referent or identity in the user's own statement/correction. Grounding never authorizes details absent from the user authority turn. Use empty strings for an unused source tuple.
A question must never be converted into an asserted decision, preference, constraint, or fact merely because it implies one. If the user turn is genuinely interrogative and does not itself assert the proposition, return no candidate unless a separate explicit adoption, correction, or retention act supplies authority.
A vague or deictic user turn such as "do that", "that works", "good rule", "change that", or similar language cannot independently support details absent from that turn. When it explicitly adopts assistant-authored content, use authority_kind adoption and authority_source_*; when it explicitly asks to retain assistant-authored content, use authority_kind retention and authority_source_*. When the user itself asserts or corrects the proposition but uses a deictic referent such as "it", "that project", or "them", use direct/correction authority plus grounding_source_* only to identify the referent. If the referent cannot be grounded, return no candidate.
category must be exactly one of: fact, preference, decision, instruction, relationship, constraint, correction, commitment.
type must be exactly one of: identity, education, employment, location, possession, health, finance, schedule, communication, project, process, product, relationship, other.
Use process for habits, workflows, routines, and methods such as meal prep. Use other only when no listed type applies.
Extract only durable facts, preferences, decisions, instructions, relationships, constraints, corrections, and unresolved commitments attributable to the user or explicitly adopted by the user.
Do not retain advice, recommendations, examples, explanations, generated copy, or other assistant-authored material unless a later user turn explicitly accepts it as a decision, preference, instruction, or commitment.
Optimize for durable information density, not sentence-level atomicity. A candidate should be one coherent memory unit about a single subject, decision, procedure, project state, diagnosis, constraint set, or unresolved commitment. Consolidate closely related details that are useful together and share the same authority and lifecycle. Do not split implementation steps, rationale, status, and consequences into separate candidates when they describe the same adopted decision or continuing state.
Separate candidates only when details have different subjects, authority, lifecycle, or would reasonably be retrieved, revised, or superseded independently. A candidate's content may contain multiple tightly related propositions. Use the shortest exact contiguous source passage that supports the complete candidate; it may span multiple clauses.
Project implementation details, reusable commands and paths, procedures, sequencing, current state, diagnoses, and unresolved next actions may be durable when they help a future agent understand or continue the work. Do not discard them merely because they are technical or temporal. Preserve whether a state is historical, current, planned, blocked, or superseded when that distinction matters.
Never emit a standalone memory whose durable content is only that a prompt, phase, step, migration batch, test run, commit, push, merge, file move, verification, or cleanup completed. These are execution receipts, not durable user knowledge. When completed work establishes a durable architecture or current state, retain only that resulting state and omit how it was executed.
Omit test counts, commit hashes, branch or worktree status, modified-file lists, untracked-file lists, prompt or phase numbers, and completion, push, or merge status unless one is itself an unresolved operational dependency that a future agent must act on. Do not append execution receipts to an otherwise useful architecture, decision, or project-state memory.
A candidate that still contains numbered prompt, phase, or step progress such as "through Prompt 72", "Prompts 25 and 26 are complete", or equivalent checkpoint framing will be rejected. If the same source also states durable resulting state, rewrite the candidate to contain only that durable state and remove the numbered progress/checkpoint reference entirely.
Temporary bugs, blockers, checkpoints, and next-step markers are worth retaining only while unresolved and independently actionable. Omit resolved failures and intermediate progress unless they explain a continuing diagnosis, dependency, constraint, migration, or required future action.
For adopted implementation plans, group details by retrieval unit rather than numbered step. Prefer at most one coherent memory for the architecture or intended result, one for governing constraints or procedure, and one for the genuinely unresolved next action when each would be revised or retrieved independently. Do not create one candidate per prompt, phase, checklist item, file move, or implementation step.
Prefer the final authoritative state for a subject. Retain intermediate states, failed approaches, or sequence history only when they explain a diagnosis, migration, dependency, constraint, or future action. Omit acknowledgments, assistant task narration, prompt numbering, repeated requests, execution chatter, and incidental commands or paths that add no independent future value.
Do not target a fixed candidate count. Return zero when nothing is worth retaining, and otherwise return the smallest set of coherent candidates that preserves the episode's durable value.
Use the exact user turn inside the authoritative episode that states or adopts the memory as source_node_id. Copy source_quote verbatim from that user turn. Use the shortest contiguous quote that establishes user authority for the complete candidate; never paraphrase it.
Read-only archive evidence is supporting context, not independent authority. Never extract a memory solely because it appears in archive evidence. Earlier assistant-authored evidence may be used as authority_source_* only when a user turn in the authoritative episode explicitly adopts or asks to retain that assistant proposition. Earlier user or assistant evidence may be used as grounding_source_* only to resolve a referent in a proposition whose authority still comes from the current authoritative user turn.
When both a user's direct statement and later reminder are inside the authoritative episode, anchor the candidate to the earlier user statement and its exact quote. Do not anchor it to the later bare remember directive, and do not treat an assistant restatement of the user's own words as the content source. A user statement supplied only as archive evidence cannot become source authority for the current episode.
For genuinely assistant-authored content adopted by the user, copy authority_source_quote verbatim from the earlier assistant turn and normalize content from that quoted material. Classify the adopted proposition itself; a bare request to remember it does not make the proposition an instruction or decision. An imperative request such as "Remember that." or "Remember this." MUST produce a candidate when its assistant-authored referent is identifiable in the episode. Interrogative forms such as "Remember that?", "Remember this?", "Do you remember that?", or "Do you remember this?" are questions and MUST NOT be treated as adoption. Never treat assistant content as adopted without a later user instruction, confirmation, or selection. If the user rejects or reverses an assistant recommendation, use correction/direct user authority and grounding_source_* to resolve any deictic referent; do not use adoption or authority_source_* for the rejected recommendation.
When a user adopts a long assistant response, retain the smallest set of coherent durable memory units needed to represent what was adopted. Consolidate tightly coupled implementation details, method steps, constraints, and current-state information that are useful together. Omit incidental caveats, examples, explanatory details, and consequences unless the user specifically selects them or they are required to apply the adopted content.
Do not infer beyond the source, merge unrelated claims, or emit transient conversational details. A question or request is not itself a preference or decision unless the user explicitly states one.
Do not extract vague standalone references such as "my new role", "that project", or "the issue" unless the same candidate contains enough identifying detail to be independently useful later.
Return candidates in source-turn order and then in the order their supporting quotes appear in that turn.
Return {"candidates":[],"evidence_requests":[]} when the episode contains nothing worth retaining."#;

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
            "authority_kind": {"type": "string", "enum": ["direct", "correction", "adoption", "retention"]},
            "category": {"type": "string", "enum": ["fact", "preference", "decision", "instruction", "relationship", "constraint", "correction", "commitment"]},
            "type": {"type": "string", "enum": ["identity", "education", "employment", "location", "possession", "health", "finance", "schedule", "communication", "project", "process", "product", "relationship", "other"]},
            "title": {"type": "string"},
            "content": {"type": "string"},
            "source_node_id": {"type": "string"},
            "source_quote": {"type": "string"},
            "authority_source_conversation_id": {"type": "string"},
            "authority_source_node_id": {"type": "string"},
            "authority_source_quote": {"type": "string"},
            "grounding_source_conversation_id": {"type": "string"},
            "grounding_source_node_id": {"type": "string"},
            "grounding_source_quote": {"type": "string"}
        },
        "required": ["authority_kind", "category", "type", "title", "content", "source_node_id", "source_quote", "authority_source_conversation_id", "authority_source_node_id", "authority_source_quote", "grounding_source_conversation_id", "grounding_source_node_id", "grounding_source_quote"]
    })
}
