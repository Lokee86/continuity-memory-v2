use crate::{
    EpisodeId, InsomniaAttempt, InsomniaError, InsomniaLeaseToken, InsomniaPriority, InsomniaWork,
    InsomniaWorkState, MemoryId,
};

const FORMAT_MAGIC: [u8; 8] = *b"CVAINSF1";
const WORK_MAGIC: [u8; 8] = *b"CVAINSW1";
const ATTEMPT_MAGIC: [u8; 8] = *b"CVAINSA1";

pub(crate) enum InsomniaRecord {
    Work(InsomniaWork),
    Attempt(InsomniaAttempt),
}

pub(crate) fn encode_format() -> Vec<u8> {
    FORMAT_MAGIC.to_vec()
}

pub(crate) fn decode_format(bytes: &[u8]) -> Result<bool, InsomniaError> {
    if bytes.len() >= 8 && bytes[..8] == FORMAT_MAGIC {
        if bytes.len() != 8 {
            return Err(InsomniaError::CorruptRecord(
                "insomnia format trailing bytes",
            ));
        }
        return Ok(true);
    }
    Ok(false)
}

pub(crate) fn encode_work(work: &InsomniaWork) -> Result<Vec<u8>, InsomniaError> {
    let mut out = Vec::with_capacity(160);
    out.extend_from_slice(&WORK_MAGIC);
    out.extend_from_slice(&work.episode_id.0);
    out.push(work.priority.tag());
    out.push(work.state.tag());
    out.extend_from_slice(&work.attempt_count.to_le_bytes());
    out.extend_from_slice(&work.updated_at_ns.to_le_bytes());
    write_optional_string(&mut out, work.lease_owner.as_deref())?;
    write_optional_token(&mut out, work.lease_token);
    write_optional_i64(&mut out, work.lease_expires_ns);
    write_optional_i64(&mut out, work.retry_after_ns);
    write_optional_string(&mut out, work.last_error.as_deref())?;
    Ok(out)
}

pub(crate) fn encode_attempt(attempt: &InsomniaAttempt) -> Result<Vec<u8>, InsomniaError> {
    let mut out = Vec::with_capacity(192 + attempt.memory_ids.len() * 32);
    out.extend_from_slice(&ATTEMPT_MAGIC);
    out.extend_from_slice(&attempt.episode_id.0);
    out.extend_from_slice(&attempt.attempt.to_le_bytes());
    out.push(attempt.state.tag());
    out.extend_from_slice(&attempt.started_at_ns.to_le_bytes());
    out.extend_from_slice(&attempt.completed_at_ns.to_le_bytes());
    out.extend_from_slice(&attempt.rejected_count.to_le_bytes());
    write_string(&mut out, &attempt.extractor_model)?;
    write_string(&mut out, &attempt.extractor_version)?;
    let count = u32::try_from(attempt.memory_ids.len())
        .map_err(|_| InsomniaError::InvalidField("memory ids"))?;
    out.extend_from_slice(&count.to_le_bytes());
    for id in &attempt.memory_ids {
        out.extend_from_slice(&id.0);
    }
    write_optional_string(&mut out, attempt.error.as_deref())?;
    Ok(out)
}

pub(crate) fn decode_record(bytes: &[u8]) -> Result<Option<InsomniaRecord>, InsomniaError> {
    if bytes.len() < 8 {
        return Ok(None);
    }
    if bytes[..8] == WORK_MAGIC {
        return decode_work(bytes).map(|work| Some(InsomniaRecord::Work(work)));
    }
    if bytes[..8] == ATTEMPT_MAGIC {
        return decode_attempt(bytes).map(|attempt| Some(InsomniaRecord::Attempt(attempt)));
    }
    Ok(None)
}

fn decode_work(bytes: &[u8]) -> Result<InsomniaWork, InsomniaError> {
    if bytes.len() < 54 {
        return Err(InsomniaError::CorruptRecord("short insomnia work"));
    }
    let episode_id = EpisodeId(bytes[8..40].try_into().unwrap());
    let priority = InsomniaPriority::from_tag(bytes[40])
        .ok_or(InsomniaError::CorruptRecord("invalid insomnia priority"))?;
    let state = InsomniaWorkState::from_tag(bytes[41])
        .ok_or(InsomniaError::CorruptRecord("invalid insomnia state"))?;
    let attempt_count = u32::from_le_bytes(bytes[42..46].try_into().unwrap());
    let updated_at_ns = i64::from_le_bytes(bytes[46..54].try_into().unwrap());
    let mut cursor = 54;
    let lease_owner = read_optional_string(bytes, &mut cursor)?;
    let lease_token = read_optional_token(bytes, &mut cursor)?;
    let lease_expires_ns = read_optional_i64(bytes, &mut cursor)?;
    let retry_after_ns = read_optional_i64(bytes, &mut cursor)?;
    let last_error = read_optional_string(bytes, &mut cursor)?;
    if cursor != bytes.len() {
        return Err(InsomniaError::CorruptRecord("insomnia work trailing bytes"));
    }
    Ok(InsomniaWork {
        episode_id,
        priority,
        state,
        attempt_count,
        lease_owner,
        lease_token,
        lease_expires_ns,
        retry_after_ns,
        last_error,
        updated_at_ns,
    })
}

fn decode_attempt(bytes: &[u8]) -> Result<InsomniaAttempt, InsomniaError> {
    if bytes.len() < 65 {
        return Err(InsomniaError::CorruptRecord("short insomnia attempt"));
    }
    let episode_id = EpisodeId(bytes[8..40].try_into().unwrap());
    let attempt = u32::from_le_bytes(bytes[40..44].try_into().unwrap());
    let state = InsomniaWorkState::from_tag(bytes[44])
        .ok_or(InsomniaError::CorruptRecord("invalid attempt state"))?;
    let started_at_ns = i64::from_le_bytes(bytes[45..53].try_into().unwrap());
    let completed_at_ns = i64::from_le_bytes(bytes[53..61].try_into().unwrap());
    let rejected_count = u32::from_le_bytes(bytes[61..65].try_into().unwrap());
    let mut cursor = 65;
    let extractor_model = read_string(bytes, &mut cursor)?;
    let extractor_version = read_string(bytes, &mut cursor)?;
    let count_end = cursor
        .checked_add(4)
        .ok_or(InsomniaError::InvalidField("memory ids"))?;
    let count_raw = bytes
        .get(cursor..count_end)
        .ok_or(InsomniaError::CorruptRecord("missing memory id count"))?;
    let count = u32::from_le_bytes(count_raw.try_into().unwrap()) as usize;
    cursor = count_end;
    let mut memory_ids = Vec::with_capacity(count);
    for _ in 0..count {
        let end = cursor
            .checked_add(32)
            .ok_or(InsomniaError::InvalidField("memory ids"))?;
        let raw = bytes
            .get(cursor..end)
            .ok_or(InsomniaError::CorruptRecord("truncated memory id"))?;
        memory_ids.push(MemoryId(raw.try_into().unwrap()));
        cursor = end;
    }
    let error = read_optional_string(bytes, &mut cursor)?;
    if cursor != bytes.len() {
        return Err(InsomniaError::CorruptRecord(
            "insomnia attempt trailing bytes",
        ));
    }
    Ok(InsomniaAttempt {
        episode_id,
        attempt,
        state,
        started_at_ns,
        completed_at_ns,
        extractor_model,
        extractor_version,
        memory_ids,
        rejected_count,
        error,
    })
}

fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), InsomniaError> {
    let len = u32::try_from(value.len()).map_err(|_| InsomniaError::InvalidField("string"))?;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn read_string(bytes: &[u8], cursor: &mut usize) -> Result<String, InsomniaError> {
    let end = cursor
        .checked_add(4)
        .ok_or(InsomniaError::InvalidField("string"))?;
    let raw = bytes
        .get(*cursor..end)
        .ok_or(InsomniaError::CorruptRecord("missing string length"))?;
    let len = u32::from_le_bytes(raw.try_into().unwrap()) as usize;
    *cursor = end;
    let end = cursor
        .checked_add(len)
        .ok_or(InsomniaError::InvalidField("string"))?;
    let raw = bytes
        .get(*cursor..end)
        .ok_or(InsomniaError::CorruptRecord("truncated string"))?;
    *cursor = end;
    String::from_utf8(raw.to_vec()).map_err(|_| InsomniaError::CorruptRecord("invalid utf8"))
}

fn write_optional_string(out: &mut Vec<u8>, value: Option<&str>) -> Result<(), InsomniaError> {
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

fn read_optional_string(bytes: &[u8], cursor: &mut usize) -> Result<Option<String>, InsomniaError> {
    let flag = *bytes
        .get(*cursor)
        .ok_or(InsomniaError::CorruptRecord("missing optional string flag"))?;
    *cursor += 1;
    match flag {
        0 => Ok(None),
        1 => read_string(bytes, cursor).map(Some),
        _ => Err(InsomniaError::CorruptRecord("invalid optional string flag")),
    }
}

fn write_optional_token(out: &mut Vec<u8>, value: Option<InsomniaLeaseToken>) {
    match value {
        Some(token) => {
            out.push(1);
            out.extend_from_slice(&token.0);
        }
        None => out.push(0),
    }
}

fn read_optional_token(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<Option<InsomniaLeaseToken>, InsomniaError> {
    let flag = *bytes
        .get(*cursor)
        .ok_or(InsomniaError::CorruptRecord("missing lease token flag"))?;
    *cursor += 1;
    match flag {
        0 => Ok(None),
        1 => {
            let end = cursor
                .checked_add(32)
                .ok_or(InsomniaError::InvalidField("lease token"))?;
            let raw = bytes
                .get(*cursor..end)
                .ok_or(InsomniaError::CorruptRecord("truncated lease token"))?;
            *cursor = end;
            Ok(Some(InsomniaLeaseToken(raw.try_into().unwrap())))
        }
        _ => Err(InsomniaError::CorruptRecord("invalid lease token flag")),
    }
}

fn write_optional_i64(out: &mut Vec<u8>, value: Option<i64>) {
    match value {
        Some(value) => {
            out.push(1);
            out.extend_from_slice(&value.to_le_bytes());
        }
        None => out.push(0),
    }
}

fn read_optional_i64(bytes: &[u8], cursor: &mut usize) -> Result<Option<i64>, InsomniaError> {
    let flag = *bytes
        .get(*cursor)
        .ok_or(InsomniaError::CorruptRecord("missing optional i64 flag"))?;
    *cursor += 1;
    match flag {
        0 => Ok(None),
        1 => {
            let end = cursor
                .checked_add(8)
                .ok_or(InsomniaError::InvalidField("i64"))?;
            let raw = bytes
                .get(*cursor..end)
                .ok_or(InsomniaError::CorruptRecord("truncated optional i64"))?;
            *cursor = end;
            Ok(Some(i64::from_le_bytes(raw.try_into().unwrap())))
        }
        _ => Err(InsomniaError::CorruptRecord("invalid optional i64 flag")),
    }
}
