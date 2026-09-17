use super::extraction::{InsomniaCandidate, InsomniaExtractionError, InsomniaRoutingMetadata};
use crate::{
    GeneralEndpoint, MAX_MEMORY_ENTITY_MENTIONS, MAX_MEMORY_LEXICAL_TERMS,
    MAX_MEMORY_ROUTING_TEXT_BYTES, MemoryEntityMention, MemoryTextField,
};
use serde_json::{Map, Value, json};

pub(super) const ENRICHMENT_SYSTEM_PROMPT: &str = r#"You are the routing-metadata enrichment pass for Reliquary Insomnia. You receive final durable Memory title/content text. The Memory proposition, wording, ownership, provenance, and lifecycle are already authoritative and MUST NOT be changed.
For each supplied Memory, extract only bounded routing metadata. entity_mentions are exact source-language spans that name potentially durable referents: people, organizations, places, projects, products, repositories, teams, sites, files, software systems, named protocols/standards, code/API artifacts, and stable descriptive referents such as "the Vancouver office", "my brother", or "the user" when it clearly denotes the current owner. Do NOT emit every salient noun. Use the smallest surface phrase that still distinguishes the intended referent. Prefer "Windows Git" over overlapping "Windows" + "Git" at that occurrence; emit a separate standalone "Git" elsewhere. For Markdown inline code such as `game.gd`, return "game.gd" without delimiters. Exclude pronouns, transient runtime roles, generic classes, abstract concepts, properties/states, and task/process labels such as "Prompt 94", "Phase 11A", "player state", or "hitboxes". Never canonicalize, paraphrase, change case, pluralize/singularize, or invent text. Do not resolve identity, assign types/IDs, or merge aliases.
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
        retain_most_specific_mentions(&mut entity_mentions);
        if entity_mentions.len() > MAX_MEMORY_ENTITY_MENTIONS {
            return Err(invalid(
                "expanded Entity mentions exceed bounded item limit",
            ));
        }
        let mut lexical_terms = Vec::new();
        for term in terms {
            if let Some(term) = validate_term(candidate, term)? {
                lexical_terms.push(term);
            }
        }
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
        return Ok(());
    }
    let source = match field {
        MemoryTextField::Title => candidate.title.as_str(),
        MemoryTextField::Content => candidate.content.as_str(),
    };
    let text = normalize_inline_code_mention(source, text);
    let starts = occurrence_starts(source, text);
    if starts.is_empty() {
        return Ok(());
    }
    for start in starts {
        let end = start + text.len();
        let surface = &source[start..end];
        if !is_semantic_match_boundary(source, start, end, surface) {
            continue;
        }
        output.push(MemoryEntityMention {
            field,
            start_byte: u32::try_from(start)
                .map_err(|_| invalid("entity mention span overflow"))?,
            end_byte: u32::try_from(end).map_err(|_| invalid("entity mention span overflow"))?,
            text: surface.to_owned(),
        });
    }
    Ok(())
}

fn occurrence_starts(source: &str, text: &str) -> Vec<usize> {
    let exact = source
        .match_indices(text)
        .map(|(start, _)| start)
        .collect::<Vec<_>>();
    if !exact.is_empty() || !text.is_ascii() {
        return exact;
    }
    source
        .as_bytes()
        .windows(text.len())
        .enumerate()
        .filter_map(|(start, window)| {
            let end = start + text.len();
            (source.is_char_boundary(start)
                && source.is_char_boundary(end)
                && window.eq_ignore_ascii_case(text.as_bytes()))
            .then_some(start)
        })
        .collect()
}

fn retain_most_specific_mentions(mentions: &mut Vec<MemoryEntityMention>) {
    let keep = mentions
        .iter()
        .enumerate()
        .map(|(index, mention)| {
            !mentions.iter().enumerate().any(|(other_index, other)| {
                index != other_index
                    && mention.field == other.field
                    && other.start_byte <= mention.start_byte
                    && other.end_byte >= mention.end_byte
                    && (other.start_byte < mention.start_byte || other.end_byte > mention.end_byte)
            })
        })
        .collect::<Vec<_>>();
    let mut index = 0usize;
    mentions.retain(|_| {
        let retain = keep[index];
        index += 1;
        retain
    });
}

fn normalize_inline_code_mention<'a>(source: &str, text: &'a str) -> &'a str {
    if text.len() >= 3 && text.starts_with('`') && text.ends_with('`') {
        let inner = &text[1..text.len() - 1];
        if !inner.is_empty()
            && !inner.starts_with('`')
            && !inner.ends_with('`')
            && !occurrence_starts(source, inner).is_empty()
        {
            return inner;
        }
    }
    text
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
) -> Result<Option<String>, InsomniaExtractionError> {
    let term = value
        .as_str()
        .ok_or_else(|| invalid("lexical term must be a string"))?;
    if term.is_empty() || term != term.trim() || term.len() > MAX_MEMORY_ROUTING_TEXT_BYTES {
        return Ok(None);
    }
    if candidate.title.contains(term) || candidate.content.contains(term) {
        return Ok(Some(term.to_owned()));
    }
    for source in [&candidate.title, &candidate.content] {
        if let Some(start) = occurrence_starts(source, term).into_iter().next() {
            return Ok(Some(source[start..start + term.len()].to_owned()));
        }
    }
    Ok(None)
}

fn invalid(message: impl Into<String>) -> InsomniaExtractionError {
    InsomniaExtractionError::InvalidOutput(message.into())
}
