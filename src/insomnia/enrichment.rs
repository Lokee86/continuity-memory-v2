use super::extraction::{InsomniaCandidate, InsomniaExtractionError, InsomniaRoutingMetadata};
use crate::{
    GeneralEndpoint, MAX_MEMORY_ENTITY_MENTIONS, MAX_MEMORY_LEXICAL_TERMS,
    MAX_MEMORY_ROUTING_TEXT_BYTES, MemoryEntityMention, MemoryTextField,
};
use serde_json::{Map, Value, json};

pub(super) const ENRICHMENT_SYSTEM_PROMPT: &str = r#"You are the routing-metadata enrichment pass for Reliquary Insomnia. You receive final durable Memory title/content text. The Memory proposition, wording, ownership, provenance, and lifecycle are already authoritative and MUST NOT be changed.
For each supplied Memory, extract only bounded routing metadata. entity_mentions: exact text spans that denote potentially durable concrete referents such as people, organizations, places, projects, products, repositories, teams, sites, files, software systems, or stable descriptive referents such as "the Vancouver office" or "my brother" when they identify a particular thing. Do not resolve identity, invent canonical names, assign types/IDs, merge aliases, or replace source wording. Exclude pronouns, generic classes, abstract concepts, and incidental adjectives.
lexical_terms: salient exact words or short phrases copied from the final title/content that would help deterministically retrieve related Memories. Prefer distinctive domain/project/entity terms. Exclude stopwords, generic conversational language, paraphrases, and synonyms not literally present.
Every emitted entity mention MUST specify whether its exact text came from title or content. Every emitted string MUST be copied verbatim from that field. Return exactly one result for every supplied Memory key, including empty arrays when nothing qualifies."#;
pub(super) fn enrich(
    endpoint: &dyn GeneralEndpoint,
    candidates: &mut [InsomniaCandidate],
) -> Result<(), InsomniaExtractionError> {
    if candidates.is_empty() {
        return Ok(());
    }
    let payload = json!({
        "memories": candidates.iter().map(|candidate| json!({
            "memory_key": candidate.key,
            "title": candidate.title,
            "content": candidate.content,
        })).collect::<Vec<_>>()
    });
    let payload = serde_json::to_string(&payload)
        .map_err(|error| InsomniaExtractionError::InvalidOutput(error.to_string()))?;
    let result = endpoint.complete_json(
        ENRICHMENT_SYSTEM_PROMPT,
        &payload,
        "insomnia_memory_routing_metadata",
        &schema(candidates),
    )?;
    apply(candidates, &result)
}

fn schema(candidates: &[InsomniaCandidate]) -> Value {
    let mut properties = Map::new();
    let required = candidates
        .iter()
        .map(|candidate| Value::String(candidate.key.clone()))
        .collect::<Vec<_>>();
    for candidate in candidates {
        properties.insert(
            candidate.key.clone(),
            json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "entity_mentions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "additionalProperties": false,
                            "properties": {
                                "field": {"type": "string", "enum": ["title", "content"]},
                                "text": {"type": "string"}
                            },
                            "required": ["field", "text"]
                        }
                    },
                    "lexical_terms": {"type": "array", "items": {"type": "string"}}
                },
                "required": ["entity_mentions", "lexical_terms"]
            }),
        );
    }
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "memories": {
                "type": "object",
                "additionalProperties": false,
                "properties": properties,
                "required": required
            }
        },
        "required": ["memories"]
    })
}

fn apply(
    candidates: &mut [InsomniaCandidate],
    result: &Value,
) -> Result<(), InsomniaExtractionError> {
    let values = result
        .get("memories")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("routing metadata output is missing memories object"))?;
    if values.len() != candidates.len() {
        return Err(invalid("routing metadata Memory count mismatch"));
    }
    for candidate in candidates {
        let value = values
            .get(&candidate.key)
            .ok_or_else(|| invalid(format!("routing metadata omitted {}", candidate.key)))?;
        let mentions = value
            .get("entity_mentions")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("entity_mentions must be an array"))?;
        let terms = value
            .get("lexical_terms")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("lexical_terms must be an array"))?;
        if mentions.len() > MAX_MEMORY_ENTITY_MENTIONS || terms.len() > MAX_MEMORY_LEXICAL_TERMS {
            return Err(invalid("routing metadata exceeds bounded item limits"));
        }
        let mut entity_mentions = Vec::new();
        for mention in mentions {
            append_mentions(candidate, mention, &mut entity_mentions)?;
        }
        entity_mentions.sort_by(|left, right| {
            (left.field, left.start_byte, left.end_byte, &left.text).cmp(&(
                right.field,
                right.start_byte,
                right.end_byte,
                &right.text,
            ))
        });
        entity_mentions.dedup();
        if entity_mentions.len() > MAX_MEMORY_ENTITY_MENTIONS {
            return Err(invalid(
                "expanded Entity mentions exceed bounded item limit",
            ));
        }
        let mut lexical_terms = terms
            .iter()
            .map(|term| validate_term(candidate, term))
            .collect::<Result<Vec<_>, _>>()?;
        lexical_terms.sort();
        lexical_terms.dedup();
        candidate.routing_metadata = Some(InsomniaRoutingMetadata {
            entity_mentions,
            lexical_terms,
        });
    }
    Ok(())
}

fn append_mentions(
    candidate: &InsomniaCandidate,
    value: &Value,
    output: &mut Vec<MemoryEntityMention>,
) -> Result<(), InsomniaExtractionError> {
    let field = match value.get("field").and_then(Value::as_str) {
        Some("title") => MemoryTextField::Title,
        Some("content") => MemoryTextField::Content,
        _ => return Err(invalid("entity mention field is invalid")),
    };
    let text = value
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("entity mention text is missing"))?;
    if text.is_empty() || text != text.trim() || text.len() > MAX_MEMORY_ROUTING_TEXT_BYTES {
        return Err(invalid("entity mention text is invalid"));
    }
    let source = match field {
        MemoryTextField::Title => candidate.title.as_str(),
        MemoryTextField::Content => candidate.content.as_str(),
    };
    let mut saw_occurrence = false;
    let mut found = false;
    for (start, _) in source.match_indices(text) {
        saw_occurrence = true;
        let end = start + text.len();
        if !is_semantic_match_boundary(source, start, end, text) {
            continue;
        }
        found = true;
        output.push(MemoryEntityMention {
            field,
            start_byte: u32::try_from(start)
                .map_err(|_| invalid("entity mention span overflow"))?,
            end_byte: u32::try_from(end).map_err(|_| invalid("entity mention span overflow"))?,
            text: text.to_owned(),
        });
    }
    if !saw_occurrence {
        return Err(invalid(
            "entity mention is not verbatim durable Memory text",
        ));
    }
    if !found {
        return Err(invalid(
            "entity mention has no boundary-compatible durable Memory occurrence",
        ));
    }
    Ok(())
}

fn is_semantic_match_boundary(source: &str, start: usize, end: usize, text: &str) -> bool {
    let starts_word = text.chars().next().is_some_and(is_word_char);
    let ends_word = text.chars().next_back().is_some_and(is_word_char);
    let left_ok = !starts_word
        || source[..start]
            .chars()
            .next_back()
            .is_none_or(|ch| !is_word_char(ch));
    let right_ok = !ends_word
        || source[end..]
            .chars()
            .next()
            .is_none_or(|ch| !is_word_char(ch));
    left_ok && right_ok
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn validate_term(
    candidate: &InsomniaCandidate,
    value: &Value,
) -> Result<String, InsomniaExtractionError> {
    let term = value
        .as_str()
        .ok_or_else(|| invalid("lexical term must be a string"))?;
    if term.is_empty() || term != term.trim() || term.len() > MAX_MEMORY_ROUTING_TEXT_BYTES {
        return Err(invalid("lexical term is invalid"));
    }
    if !candidate.title.contains(term) && !candidate.content.contains(term) {
        return Err(invalid("lexical term is not verbatim durable Memory text"));
    }
    Ok(term.to_owned())
}

fn invalid(message: impl Into<String>) -> InsomniaExtractionError {
    InsomniaExtractionError::InvalidOutput(message.into())
}
