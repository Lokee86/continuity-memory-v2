use serde_json::{Value, json};

pub const ENTITY_RESOLVER_CONTRACT_VERSION: &str = "v4";
pub const ENTITY_ADMISSION_CONTRACT_VERSION: &str = "v1";
pub const ENTITY_MATERIALIZATION_CONTRACT_VERSION: &str = "v1";

pub const ENTITY_RESOLVER_SYSTEM_PROMPT: &str = "Resolve only the supplied Entity mention. Lexical or alias equality generates candidates; it NEVER proves identity. Compare the current Memory against each existing durable Entity's kind, summary, and evidence. A false merge is worse than leaving the mention unresolved: require positive contextual evidence before resolve_existing. Entity-kind or role conflicts are strong negative evidence: if the current context identifies a project, person, place, file, service, device, database, or other concrete kind, do not resolve it to a candidate whose kind/summary describes a different kind merely because the surface text matches. Verify that both identity category and surrounding facts are compatible. If context explicitly establishes a distinct durable identity not represented by the candidates, choose create_new. If evidence is insufficient, choose unresolved. Reject only when the mention is not a durable Entity candidate. Do not add, rewrite, merge, or alter the mention span.";

pub const ENTITY_ADMISSION_SYSTEM_PROMPT: &str = r#"Validate first admission of one exact Entity mention when no existing Entity candidate was retrieved. Upstream extraction proposed the span, but it may still be generic, incomplete, over-broad, or over-split. Decide create_new only when the EXACT supplied mention, interpreted in the current Memory, denotes one durable reusable referent. Durable referents include named people/organizations/projects/products/repositories/tools/services/subsystems, files/directories/paths, protocols/formats, durable code types or symbols, stable input actions, enum/state values, and stable owner-local software components. A durable software identity may be lowercase, hyphenated, underscored, or descriptively named. A generic-looking multiword span may still be a specific persistent component when the Memory treats that exact phrase as one referent; an explicit second/new instance may establish a distinct durable referent. The Memory may explain what the exact span denotes, but NEVER borrow words outside the mention to repair or expand it. If the durable identity requires an omitted modifier outside the supplied span, reject the supplied span rather than reconstructing it. Reject bare generic category/activity/role nouns, abstract processes, transient values, sentence-local descriptors, and incomplete fragments. Exact compound/symbol surfaces may identify durable referents when the Memory treats them as persistent things. Context_evidence may contain bounded owner-local Memories using the same surface. It is supporting evidence only: Admission decides whether THIS occurrence in the current Memory identifies one durable referent. The surface does not need to be globally unique, and other context using the same words for a second distinct referent does not by itself make the current occurrence ambiguous; later identity resolution can split that later occurrence. Choose unresolved only when the current Memory itself does not identify which durable referent is meant. In particular, a bare filename/basename such as main.go without a concrete namespace/path or other distinguishing identity is unresolved when the current Memory does not establish which file it denotes. By contrast, an explicitly existing/planned owner-local component may be admitted even if another same-surface component exists elsewhere. Do not resolve to an existing Entity because none was supplied. If and only if decision=create_new, also choose the narrowest supported entity_kind and write one compact source-grounded entity_summary that identifies this specific referent. Use the current Memory as the identity anchor. You MAY use context_evidence to infer stable kind or identity semantics only when that evidence is clearly compatible with the SAME referent. Surface equality alone does not make context compatible. Exclude context that could describe a second/new/other/different instance or otherwise conflicts with the current referent; never merge distinct same-surface identities into the metadata. The summary must describe the stable identity of the referent—what it is—not the transient task, change, operation, or event currently involving it. Omit operation-specific detail unless that detail is necessary to distinguish this identity from another durable referent. For unresolved or reject, return entity_kind=unknown and an empty entity_summary."#;

pub const ENTITY_MATERIALIZATION_SYSTEM_PROMPT: &str = "Materialize metadata for one newly accepted durable Entity. The identity decision is already made; do not reject, merge, rename, add aliases, or change the mention. Choose the narrowest supported entity_kind and write one compact source-grounded entity_summary identifying this specific referent using only the supplied Memory. Describe the stable identity of the referent—what it is—not the transient task, change, operation, or event currently involving it. Omit operation-specific detail unless it is necessary to distinguish this identity from another durable referent. Do not invent facts not present in the Memory.";

pub fn entity_resolver_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["decision", "reason", "target_candidate_index"],
        "properties": {
            "decision": {
                "type": "string",
                "enum": ["resolve_existing", "create_new", "unresolved", "reject"]
            },
            "reason": {"type": "string", "enum": resolution_reasons()},
            "target_candidate_index": {"type": "integer"}
        }
    })
}

pub fn entity_admission_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["decision", "reason", "entity_kind", "entity_summary"],
        "properties": {
            "decision": {
                "type": "string",
                "enum": ["create_new", "unresolved", "reject"]
            },
            "reason": {"type": "string", "enum": resolution_reasons()},
            "entity_kind": {"type": "string", "enum": entity_kinds_with_unknown()},
            "entity_summary": {"type": "string", "maxLength": 1024}
        }
    })
}

pub fn entity_materialization_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["entity_kind", "entity_summary"],
        "properties": {
            "entity_kind": {"type": "string", "enum": entity_kinds()},
            "entity_summary": {"type": "string", "minLength": 1, "maxLength": 1024}
        }
    })
}

fn resolution_reasons() -> Vec<&'static str> {
    vec![
        "context_match",
        "context_conflict_new_identity",
        "named_referent",
        "persistent_artifact",
        "descriptive_identity",
        "first_seen_identity",
        "insufficient_evidence",
        "generic_role",
        "abstract_process",
        "transient_value",
        "sentence_local",
        "wrapper_category",
        "ambiguous",
    ]
}

fn entity_kinds() -> Vec<&'static str> {
    vec![
        "application",
        "code_component",
        "code_symbol",
        "code_type",
        "data_format",
        "directory",
        "domain_entity",
        "enum_value",
        "file",
        "input_action",
        "programming_language",
        "repository",
        "runtime_entity",
        "service",
        "subsystem",
        "tool",
        "tool_service",
        "ui_component",
        "other",
    ]
}

fn entity_kinds_with_unknown() -> Vec<&'static str> {
    let mut values = entity_kinds();
    values.push("unknown");
    values
}
