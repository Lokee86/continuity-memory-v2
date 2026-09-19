use crate::entity_resolver::{
    EntityResolver, invalid, mention_field, parse_reason, required_string,
};
use crate::entity_resolver_schema::{
    ENTITY_ADMISSION_SYSTEM_PROMPT, ENTITY_MATERIALIZATION_SYSTEM_PROMPT, entity_admission_schema,
    entity_materialization_schema,
};
use crate::{
    EntityAdmissionDecision, EntityAdmissionOutput, EntityCandidateSet, EntityMaterialization,
    EntityResolutionReason, EntityResolverError, GeneralEndpoint, Memory,
};
use serde_json::{Value, json};

pub const MAX_ENTITY_ADMISSION_CONTEXT_MEMORIES: usize = 8;
const MAX_ENTITY_ADMISSION_CONTEXT_TEXT_BYTES: usize = 2048;

impl<E: GeneralEndpoint> EntityResolver<E> {
    pub(crate) fn admit(
        &self,
        memory: &Memory,
        set: &EntityCandidateSet,
        context: &[Memory],
    ) -> Result<EntityAdmissionOutput, EntityResolverError> {
        if !set.candidates.is_empty() {
            return Err(invalid("Entity admission requires an empty candidate set"));
        }
        if is_bare_generic_surface(&set.mention.text) {
            return Ok(EntityAdmissionOutput {
                decision: EntityAdmissionDecision::Reject,
                reason: EntityResolutionReason::GenericRole,
                materialization: None,
            });
        }
        let payload = json!({
            "title": memory.title,
            "content": memory.content,
            "mention": {
                "field": mention_field(set.mention.field),
                "start_byte": set.mention.start_byte,
                "end_byte": set.mention.end_byte,
                "text": set.mention.text,
            },
            "context_evidence": context.iter()
                .take(MAX_ENTITY_ADMISSION_CONTEXT_MEMORIES)
                .map(|value| json!({
                    "memory_id": hex(&value.id.0),
                    "title": truncate_utf8(
                        &value.title,
                        MAX_ENTITY_ADMISSION_CONTEXT_TEXT_BYTES,
                    ),
                    "content": truncate_utf8(
                        &value.content,
                        MAX_ENTITY_ADMISSION_CONTEXT_TEXT_BYTES,
                    ),
                }))
                .collect::<Vec<_>>(),
        });
        let output = self.complete_json(
            ENTITY_ADMISSION_SYSTEM_PROMPT,
            &payload.to_string(),
            "entity_admission_v1",
            &entity_admission_schema(),
        )?;
        parse_admission_output(&output)
    }

    pub(crate) fn materialize(
        &self,
        memory: &Memory,
        set: &EntityCandidateSet,
    ) -> Result<EntityMaterialization, EntityResolverError> {
        let payload = json!({
            "title": memory.title,
            "content": memory.content,
            "mention": {
                "field": mention_field(set.mention.field),
                "start_byte": set.mention.start_byte,
                "end_byte": set.mention.end_byte,
                "text": set.mention.text,
            },
        });
        let output = self.complete_json(
            ENTITY_MATERIALIZATION_SYSTEM_PROMPT,
            &payload.to_string(),
            "entity_materialization_v1",
            &entity_materialization_schema(),
        )?;
        parse_materialization(&output)
    }
}

fn parse_admission_output(output: &Value) -> Result<EntityAdmissionOutput, EntityResolverError> {
    let decision = match required_string(output, "decision")? {
        "create_new" => EntityAdmissionDecision::CreateNew,
        "unresolved" => EntityAdmissionDecision::Unresolved,
        "reject" => EntityAdmissionDecision::Reject,
        _ => return Err(invalid("unknown admission decision")),
    };
    let reason = parse_reason(required_string(output, "reason")?)?;
    let kind = required_string(output, "entity_kind")?;
    let summary = required_string(output, "entity_summary")?;
    let materialization = match decision {
        EntityAdmissionDecision::CreateNew => {
            if kind == "unknown" || summary.trim().is_empty() {
                return Err(invalid("create_new admission requires Entity metadata"));
            }
            Some(validate_materialization(kind, summary)?)
        }
        EntityAdmissionDecision::Unresolved | EntityAdmissionDecision::Reject => {
            if kind != "unknown" || !summary.is_empty() {
                return Err(invalid(
                    "non-create admission must not materialize Entity metadata",
                ));
            }
            None
        }
    };
    Ok(EntityAdmissionOutput {
        decision,
        reason,
        materialization,
    })
}

fn parse_materialization(output: &Value) -> Result<EntityMaterialization, EntityResolverError> {
    validate_materialization(
        required_string(output, "entity_kind")?,
        required_string(output, "entity_summary")?,
    )
}

fn validate_materialization(
    kind: &str,
    summary: &str,
) -> Result<EntityMaterialization, EntityResolverError> {
    let summary = summary.trim();
    if kind.is_empty() || kind == "unknown" || kind.len() > crate::MAX_ENTITY_KIND_BYTES {
        return Err(invalid("invalid Entity kind"));
    }
    if summary.is_empty() || summary.len() > crate::MAX_ENTITY_SUMMARY_BYTES {
        return Err(invalid("invalid Entity summary"));
    }
    Ok(EntityMaterialization {
        kind: kind.to_owned(),
        summary: summary.to_owned(),
    })
}

fn is_bare_generic_surface(value: &str) -> bool {
    const GENERIC: &[&str] = &[
        "agent",
        "application",
        "assistant",
        "client",
        "code",
        "data",
        "deployment",
        "devtool",
        "documentation",
        "entry",
        "file",
        "flow",
        "implementation",
        "lane",
        "pipeline",
        "project",
        "repository",
        "request",
        "response",
        "script",
        "scripts",
        "server",
        "session",
        "shader",
        "state",
        "status",
        "system",
        "tool",
        "tooling",
    ];
    let normalized = value.trim().to_ascii_lowercase();
    let bare = normalized.strip_prefix("the ").unwrap_or(&normalized);
    GENERIC.contains(&bare)
}

fn truncate_utf8(value: &str, max_bytes: usize) -> &str {
    if value.len() <= max_bytes {
        return value;
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

fn hex(bytes: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}
