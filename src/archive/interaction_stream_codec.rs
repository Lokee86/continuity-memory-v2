use crate::{CvaError, InteractionRole, InteractionStreamRecord, InteractionStreamStatus};

const RECORD_MAGIC: [u8; 8] = *b"CVAISTR1";

pub(crate) fn encode(record: &InteractionStreamRecord) -> Result<Vec<u8>, CvaError> {
    let mut out = Vec::with_capacity(48 + record.content.len());
    out.extend_from_slice(&RECORD_MAGIC);
    write_string(&mut out, &record.message_id)?;
    write_string(&mut out, &record.session_id)?;
    match &record.parent_message_id {
        Some(parent) => {
            out.push(1);
            write_string(&mut out, parent)?;
        }
        None => out.push(0),
    }
    out.push(match record.role {
        InteractionRole::User => 0,
        InteractionRole::Agent => 1,
    });
    out.extend_from_slice(&record.timestamp_ns.to_le_bytes());
    out.push(match record.status {
        InteractionStreamStatus::Streaming => 0,
        InteractionStreamStatus::Interrupted => 1,
    });
    write_string(&mut out, &record.content)?;
    Ok(out)
}

pub(crate) fn decode(bytes: &[u8]) -> Result<Option<InteractionStreamRecord>, CvaError> {
    if bytes.len() < 8 || bytes[..8] != RECORD_MAGIC {
        return Ok(None);
    }
    let mut cursor = 8;
    let message_id = read_string(bytes, &mut cursor)?;
    let session_id = read_string(bytes, &mut cursor)?;
    let parent_message_id = match read_byte(bytes, &mut cursor)? {
        0 => None,
        1 => Some(read_string(bytes, &mut cursor)?),
        _ => return Err(corrupt("invalid interaction stream parent flag")),
    };
    let role = match read_byte(bytes, &mut cursor)? {
        0 => InteractionRole::User,
        1 => InteractionRole::Agent,
        _ => return Err(corrupt("invalid interaction stream role")),
    };
    let timestamp_ns = read_i64(bytes, &mut cursor)?;
    let status = match read_byte(bytes, &mut cursor)? {
        0 => InteractionStreamStatus::Streaming,
        1 => InteractionStreamStatus::Interrupted,
        _ => return Err(corrupt("invalid interaction stream status")),
    };
    let content = read_string(bytes, &mut cursor)?;
    if cursor != bytes.len() {
        return Err(corrupt("interaction stream trailing bytes"));
    }
    Ok(Some(InteractionStreamRecord {
        message_id,
        session_id,
        parent_message_id,
        role,
        timestamp_ns,
        content,
        status,
    }))
}

fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), CvaError> {
    let len = u32::try_from(value.len())
        .map_err(|_| corrupt("interaction stream field exceeds u32 length"))?;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn read_string(bytes: &[u8], cursor: &mut usize) -> Result<String, CvaError> {
    let length_end = cursor
        .checked_add(4)
        .ok_or_else(|| corrupt("interaction stream length overflow"))?;
    let raw = bytes
        .get(*cursor..length_end)
        .ok_or_else(|| corrupt("interaction stream missing string length"))?;
    let len = u32::from_le_bytes(raw.try_into().unwrap()) as usize;
    *cursor = length_end;
    let end = cursor
        .checked_add(len)
        .ok_or_else(|| corrupt("interaction stream string overflow"))?;
    let raw = bytes
        .get(*cursor..end)
        .ok_or_else(|| corrupt("interaction stream truncated string"))?;
    *cursor = end;
    String::from_utf8(raw.to_vec())
        .map_err(|_| corrupt("interaction stream contains invalid UTF-8"))
}

fn read_byte(bytes: &[u8], cursor: &mut usize) -> Result<u8, CvaError> {
    let value = *bytes
        .get(*cursor)
        .ok_or_else(|| corrupt("interaction stream truncated byte"))?;
    *cursor += 1;
    Ok(value)
}

fn read_i64(bytes: &[u8], cursor: &mut usize) -> Result<i64, CvaError> {
    let end = cursor
        .checked_add(8)
        .ok_or_else(|| corrupt("interaction stream timestamp overflow"))?;
    let raw = bytes
        .get(*cursor..end)
        .ok_or_else(|| corrupt("interaction stream truncated timestamp"))?;
    *cursor = end;
    Ok(i64::from_le_bytes(raw.try_into().unwrap()))
}

fn corrupt(message: impl Into<String>) -> CvaError {
    CvaError::InteractionStream(message.into())
}
