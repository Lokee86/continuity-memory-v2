use crate::memory_codec_scalar::{
    read_i64, read_optional_episode, read_optional_i64, write_optional_episode, write_optional_i64,
};
use crate::memory_model::MemoryRecord;
use crate::{MemoryBodyId, MemoryError, MemoryId, ObjectRef};

const FORMAT_MAGIC: [u8; 8] = *b"CVAMEMF2";
const BODY_MAGIC: [u8; 8] = *b"CVAMBDY1";
const RECORD_MAGIC_V2: [u8; 8] = *b"CVAMEMR2";
const RECORD_MAGIC_V3: [u8; 8] = *b"CVAMEMR3";
const RECORD_MAGIC_V4: [u8; 8] = *b"CVAMEMR4";
const VERSION_MAGIC: [u8; 8] = *b"CVAMEMV1";

pub(crate) struct MemoryVersion {
    pub global_version: u64,
    pub memory_version: u64,
    pub record: ObjectRef,
}

pub(crate) fn encode_format() -> Vec<u8> {
    FORMAT_MAGIC.to_vec()
}

pub(crate) fn decode_format(bytes: &[u8]) -> Result<bool, MemoryError> {
    if bytes.len() >= 8 && bytes[..8] == FORMAT_MAGIC {
        if bytes.len() != 8 {
            return Err(MemoryError::CorruptRecord("memory format trailing bytes"));
        }
        return Ok(true);
    }
    Ok(false)
}

pub(crate) fn encode_body(id: MemoryBodyId, body: &[u8]) -> Result<Vec<u8>, MemoryError> {
    let len = u32::try_from(body.len()).map_err(|_| MemoryError::FieldTooLarge)?;
    let mut out = Vec::with_capacity(44 + body.len());
    out.extend_from_slice(&BODY_MAGIC);
    out.extend_from_slice(&id.0);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(body);
    Ok(out)
}

pub(crate) fn decode_body(bytes: &[u8]) -> Result<Option<(MemoryBodyId, Vec<u8>)>, MemoryError> {
    if bytes.len() < 8 || bytes[..8] != BODY_MAGIC {
        return Ok(None);
    }
    if bytes.len() < 44 {
        return Err(MemoryError::CorruptRecord("short memory body"));
    }
    let id = MemoryBodyId(bytes[8..40].try_into().unwrap());
    let len = u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize;
    let body = bytes
        .get(44..44 + len)
        .ok_or(MemoryError::CorruptRecord("truncated memory body"))?;
    if 44 + len != bytes.len() {
        return Err(MemoryError::CorruptRecord("memory body trailing bytes"));
    }
    Ok(Some((id, body.to_vec())))
}

pub(crate) fn encode_record(record: &MemoryRecord) -> Result<Vec<u8>, MemoryError> {
    let mut out = Vec::with_capacity(256);
    out.extend_from_slice(&RECORD_MAGIC_V4);
    out.extend_from_slice(&record.id.0);
    out.extend_from_slice(&record.revision.to_le_bytes());
    out.extend_from_slice(&record.body_id.0);
    out.push(u8::from(record.archived));
    write_optional_id(&mut out, record.superseded_by);
    write_optional_id(&mut out, record.parent_id);
    write_optional_episode(&mut out, record.source_episode_id);
    write_optional_i64(&mut out, record.source_time_ns);
    out.extend_from_slice(&record.created_at_ns.to_le_bytes());
    out.extend_from_slice(&record.updated_at_ns.to_le_bytes());
    write_string(&mut out, &record.category)?;
    write_string(&mut out, &record.memory_type)?;
    write_string(&mut out, &record.authority_kind)?;
    write_string(&mut out, &record.scope)?;
    write_string(&mut out, &record.lifecycle_state)?;
    write_optional_string(&mut out, record.source_node_id.as_deref())?;
    write_optional_string(&mut out, record.content_source_conversation_id.as_deref())?;
    write_optional_string(&mut out, record.content_source_node_id.as_deref())?;
    write_optional_string(&mut out, record.grounding_source_conversation_id.as_deref())?;
    write_optional_string(&mut out, record.grounding_source_node_id.as_deref())?;
    write_string(&mut out, &record.mutation_id)?;
    Ok(out)
}

pub(crate) fn decode_record(bytes: &[u8]) -> Result<Option<MemoryRecord>, MemoryError> {
    if bytes.len() < 8 {
        return Ok(None);
    }
    let version = if bytes[..8] == RECORD_MAGIC_V4 {
        4
    } else if bytes[..8] == RECORD_MAGIC_V3 {
        3
    } else if bytes[..8] == RECORD_MAGIC_V2 {
        2
    } else {
        return Ok(None);
    };
    if bytes.len() < 123 {
        return Err(MemoryError::CorruptRecord("short memory record"));
    }
    let id = MemoryId(bytes[8..40].try_into().unwrap());
    let revision = u64::from_le_bytes(bytes[40..48].try_into().unwrap());
    let body_id = MemoryBodyId(bytes[48..80].try_into().unwrap());
    let archived = match bytes[80] {
        0 => false,
        1 => true,
        _ => return Err(MemoryError::CorruptRecord("invalid memory archived flag")),
    };
    let mut cursor = 81;
    let superseded_by = read_optional_id(bytes, &mut cursor)?;
    let parent_id = read_optional_id(bytes, &mut cursor)?;
    let source_episode_id = read_optional_episode(bytes, &mut cursor)?;
    let source_time_ns = if version >= 4 {
        read_optional_i64(bytes, &mut cursor)?
    } else {
        None
    };
    let created_at_ns = read_i64(bytes, &mut cursor)?;
    let updated_at_ns = read_i64(bytes, &mut cursor)?;
    let category = read_string(bytes, &mut cursor)?;
    let memory_type = read_string(bytes, &mut cursor)?;
    let authority_kind = if version >= 3 {
        read_string(bytes, &mut cursor)?
    } else {
        "unknown".into()
    };
    let scope = read_string(bytes, &mut cursor)?;
    let lifecycle_state = read_string(bytes, &mut cursor)?;
    let source_node_id = read_optional_string(bytes, &mut cursor)?;
    let content_source_conversation_id = read_optional_string(bytes, &mut cursor)?;
    let content_source_node_id = read_optional_string(bytes, &mut cursor)?;
    let grounding_source_conversation_id = read_optional_string(bytes, &mut cursor)?;
    let grounding_source_node_id = read_optional_string(bytes, &mut cursor)?;
    let mutation_id = read_string(bytes, &mut cursor)?;
    if cursor != bytes.len() {
        return Err(MemoryError::CorruptRecord("memory record trailing bytes"));
    }
    Ok(Some(MemoryRecord {
        id,
        revision,
        body_id,
        category,
        memory_type,
        authority_kind,
        scope,
        lifecycle_state,
        archived,
        superseded_by,
        parent_id,
        source_node_id,
        content_source_conversation_id,
        content_source_node_id,
        grounding_source_conversation_id,
        grounding_source_node_id,
        source_episode_id,
        source_time_ns,
        mutation_id,
        created_at_ns,
        updated_at_ns,
        global_version: 0,
        memory_version: 0,
    }))
}

pub(crate) fn encode_version(version: MemoryVersion) -> Vec<u8> {
    let mut out = Vec::with_capacity(40);
    out.extend_from_slice(&VERSION_MAGIC);
    out.extend_from_slice(&version.global_version.to_le_bytes());
    out.extend_from_slice(&version.memory_version.to_le_bytes());
    out.extend_from_slice(&version.record.legacy_bytes());
    out
}

pub(crate) fn decode_version(bytes: &[u8]) -> Result<Option<MemoryVersion>, MemoryError> {
    if bytes.len() < 8 || bytes[..8] != VERSION_MAGIC {
        return Ok(None);
    }
    if bytes.len() != 40 {
        return Err(MemoryError::CorruptRecord("invalid memory version record"));
    }
    Ok(Some(MemoryVersion {
        global_version: u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
        memory_version: u64::from_le_bytes(bytes[16..24].try_into().unwrap()),
        record: ObjectRef::from_legacy_bytes(bytes[24..40].try_into().unwrap()),
    }))
}

fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), MemoryError> {
    let len = u32::try_from(value.len()).map_err(|_| MemoryError::FieldTooLarge)?;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn read_string(bytes: &[u8], cursor: &mut usize) -> Result<String, MemoryError> {
    let end = cursor.checked_add(4).ok_or(MemoryError::FieldTooLarge)?;
    let raw = bytes
        .get(*cursor..end)
        .ok_or(MemoryError::CorruptRecord("missing memory string length"))?;
    let len = u32::from_le_bytes(raw.try_into().unwrap()) as usize;
    *cursor = end;
    let end = cursor.checked_add(len).ok_or(MemoryError::FieldTooLarge)?;
    let value = bytes
        .get(*cursor..end)
        .ok_or(MemoryError::CorruptRecord("truncated memory string"))?;
    *cursor = end;
    String::from_utf8(value.to_vec()).map_err(|_| MemoryError::InvalidUtf8)
}

fn write_optional_string(out: &mut Vec<u8>, value: Option<&str>) -> Result<(), MemoryError> {
    match value {
        Some(value) => {
            out.push(1);
            write_string(out, value)
        }
        None => {
            out.push(0);
            Ok(())
        }
    }
}

fn read_optional_string(bytes: &[u8], cursor: &mut usize) -> Result<Option<String>, MemoryError> {
    let flag = *bytes
        .get(*cursor)
        .ok_or(MemoryError::CorruptRecord("missing optional string flag"))?;
    *cursor += 1;
    match flag {
        0 => Ok(None),
        1 => read_string(bytes, cursor).map(Some),
        _ => Err(MemoryError::CorruptRecord("invalid optional string flag")),
    }
}

fn write_optional_id(out: &mut Vec<u8>, value: Option<MemoryId>) {
    match value {
        Some(id) => {
            out.push(1);
            out.extend_from_slice(&id.0);
        }
        None => out.push(0),
    }
}

fn read_optional_id(bytes: &[u8], cursor: &mut usize) -> Result<Option<MemoryId>, MemoryError> {
    let flag = *bytes.get(*cursor).ok_or(MemoryError::CorruptRecord(
        "missing optional memory id flag",
    ))?;
    *cursor += 1;
    match flag {
        0 => Ok(None),
        1 => {
            let end = cursor.checked_add(32).ok_or(MemoryError::FieldTooLarge)?;
            let raw = bytes
                .get(*cursor..end)
                .ok_or(MemoryError::CorruptRecord("truncated optional memory id"))?;
            *cursor = end;
            Ok(Some(MemoryId(raw.try_into().unwrap())))
        }
        _ => Err(MemoryError::CorruptRecord(
            "invalid optional memory id flag",
        )),
    }
}
