use crate::memory_routing_model::{
    MAX_MEMORY_ENTITY_MENTIONS, MAX_MEMORY_LEXICAL_TERMS, MemoryEntityMention,
    MemoryRoutingMetadata, MemoryTextField,
};
use crate::{MemoryBodyId, MemoryError, MemoryId};

const ROUTING_MAGIC: [u8; 8] = *b"CVAMRTE1";

pub(crate) fn encode(value: &MemoryRoutingMetadata) -> Result<Vec<u8>, MemoryError> {
    let mention_count =
        u32::try_from(value.entity_mentions.len()).map_err(|_| MemoryError::FieldTooLarge)?;
    let term_count =
        u32::try_from(value.lexical_terms.len()).map_err(|_| MemoryError::FieldTooLarge)?;
    let mut out = Vec::with_capacity(96);
    out.extend_from_slice(&ROUTING_MAGIC);
    out.extend_from_slice(&value.memory_id.0);
    out.extend_from_slice(&value.body_id.0);
    out.extend_from_slice(&mention_count.to_le_bytes());
    for mention in &value.entity_mentions {
        out.push(mention.field.tag());
        out.extend_from_slice(&mention.start_byte.to_le_bytes());
        out.extend_from_slice(&mention.end_byte.to_le_bytes());
        write_string(&mut out, &mention.text)?;
    }
    out.extend_from_slice(&term_count.to_le_bytes());
    for term in &value.lexical_terms {
        write_string(&mut out, term)?;
    }
    Ok(out)
}

pub(crate) fn decode(bytes: &[u8]) -> Result<Option<MemoryRoutingMetadata>, MemoryError> {
    if bytes.len() < 8 || bytes[..8] != ROUTING_MAGIC {
        return Ok(None);
    }
    if bytes.len() < 80 {
        return Err(MemoryError::CorruptRecord("short Memory routing metadata"));
    }
    let memory_id = MemoryId(bytes[8..40].try_into().unwrap());
    let body_id = MemoryBodyId(bytes[40..72].try_into().unwrap());
    let mut cursor = 72;
    let mention_count = read_u32(bytes, &mut cursor)? as usize;
    if mention_count > MAX_MEMORY_ENTITY_MENTIONS {
        return Err(MemoryError::CorruptRecord(
            "too many Memory entity mentions",
        ));
    }
    let mut entity_mentions = Vec::with_capacity(mention_count);
    for _ in 0..mention_count {
        let field = MemoryTextField::from_tag(read_u8(bytes, &mut cursor)?)
            .ok_or(MemoryError::CorruptRecord("invalid Memory mention field"))?;
        let start_byte = read_u32(bytes, &mut cursor)?;
        let end_byte = read_u32(bytes, &mut cursor)?;
        let text = read_string(bytes, &mut cursor)?;
        entity_mentions.push(MemoryEntityMention {
            field,
            start_byte,
            end_byte,
            text,
        });
    }
    let term_count = read_u32(bytes, &mut cursor)? as usize;
    if term_count > MAX_MEMORY_LEXICAL_TERMS {
        return Err(MemoryError::CorruptRecord("too many Memory lexical terms"));
    }
    let mut lexical_terms = Vec::with_capacity(term_count);
    for _ in 0..term_count {
        lexical_terms.push(read_string(bytes, &mut cursor)?);
    }
    if cursor != bytes.len() {
        return Err(MemoryError::CorruptRecord(
            "Memory routing metadata trailing bytes",
        ));
    }
    Ok(Some(MemoryRoutingMetadata {
        memory_id,
        body_id,
        entity_mentions,
        lexical_terms,
    }))
}

fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), MemoryError> {
    let len = u32::try_from(value.len()).map_err(|_| MemoryError::FieldTooLarge)?;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn read_string(bytes: &[u8], cursor: &mut usize) -> Result<String, MemoryError> {
    let len = read_u32(bytes, cursor)? as usize;
    let end = cursor.checked_add(len).ok_or(MemoryError::FieldTooLarge)?;
    let raw = bytes.get(*cursor..end).ok_or(MemoryError::CorruptRecord(
        "truncated Memory routing string",
    ))?;
    *cursor = end;
    String::from_utf8(raw.to_vec()).map_err(|_| MemoryError::InvalidUtf8)
}

fn read_u8(bytes: &[u8], cursor: &mut usize) -> Result<u8, MemoryError> {
    let value = *bytes
        .get(*cursor)
        .ok_or(MemoryError::CorruptRecord("truncated Memory routing byte"))?;
    *cursor += 1;
    Ok(value)
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, MemoryError> {
    let end = cursor.checked_add(4).ok_or(MemoryError::FieldTooLarge)?;
    let raw = bytes.get(*cursor..end).ok_or(MemoryError::CorruptRecord(
        "truncated Memory routing integer",
    ))?;
    *cursor = end;
    Ok(u32::from_le_bytes(raw.try_into().unwrap()))
}
