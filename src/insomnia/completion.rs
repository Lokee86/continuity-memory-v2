use crate::EpisodeId;
use crate::memory_codec::{decode_record, encode_record};
use crate::memory_model::MemoryRecord;

const COMPLETION_MAGIC: [u8; 8] = *b"CVAINSC1";

#[derive(Clone, Debug)]
pub(crate) struct InsomniaCompletion {
    pub(crate) episode_id: EpisodeId,
    pub(crate) attempt: u32,
    pub(crate) started_at_ns: i64,
    pub(crate) completed_at_ns: i64,
    pub(crate) extractor_model: String,
    pub(crate) extractor_version: String,
    pub(crate) rejected_count: u32,
    pub(crate) memory_ids: Vec<crate::MemoryId>,
    pub(crate) records: Vec<MemoryRecord>,
}

pub(crate) fn encode_completion(value: &InsomniaCompletion) -> Result<Vec<u8>, &'static str> {
    let mut out = Vec::with_capacity(160 + value.records.len() * 256);
    out.extend_from_slice(&COMPLETION_MAGIC);
    out.extend_from_slice(&value.episode_id.0);
    out.extend_from_slice(&value.attempt.to_le_bytes());
    out.extend_from_slice(&value.started_at_ns.to_le_bytes());
    out.extend_from_slice(&value.completed_at_ns.to_le_bytes());
    out.extend_from_slice(&value.rejected_count.to_le_bytes());
    write_string(&mut out, &value.extractor_model)?;
    write_string(&mut out, &value.extractor_version)?;
    let memory_count =
        u32::try_from(value.memory_ids.len()).map_err(|_| "too many completion memory ids")?;
    out.extend_from_slice(&memory_count.to_le_bytes());
    for id in &value.memory_ids {
        out.extend_from_slice(&id.0);
    }
    let count = u32::try_from(value.records.len()).map_err(|_| "too many completion records")?;
    out.extend_from_slice(&count.to_le_bytes());
    for record in &value.records {
        out.extend_from_slice(&record.global_version.to_le_bytes());
        out.extend_from_slice(&record.memory_version.to_le_bytes());
        let encoded = encode_record(record).map_err(|_| "invalid completion memory record")?;
        let len = u32::try_from(encoded.len()).map_err(|_| "completion record too large")?;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&encoded);
    }
    Ok(out)
}

pub(crate) fn decode_completion(bytes: &[u8]) -> Result<Option<InsomniaCompletion>, &'static str> {
    if bytes.len() < 8 || bytes[..8] != COMPLETION_MAGIC {
        return Ok(None);
    }
    if bytes.len() < 72 {
        return Err("short Insomnia completion");
    }
    let episode_id = EpisodeId(bytes[8..40].try_into().unwrap());
    let attempt = u32::from_le_bytes(bytes[40..44].try_into().unwrap());
    let started_at_ns = i64::from_le_bytes(bytes[44..52].try_into().unwrap());
    let completed_at_ns = i64::from_le_bytes(bytes[52..60].try_into().unwrap());
    let rejected_count = u32::from_le_bytes(bytes[60..64].try_into().unwrap());
    let mut cursor = 64;
    let extractor_model = read_string(bytes, &mut cursor)?;
    let extractor_version = read_string(bytes, &mut cursor)?;
    let memory_count = read_u32(bytes, &mut cursor)? as usize;
    let mut memory_ids = Vec::with_capacity(memory_count);
    for _ in 0..memory_count {
        let end = cursor
            .checked_add(32)
            .ok_or("completion memory id overflow")?;
        let raw = bytes
            .get(cursor..end)
            .ok_or("truncated completion memory id")?;
        cursor = end;
        memory_ids.push(crate::MemoryId(raw.try_into().unwrap()));
    }
    let count = read_u32(bytes, &mut cursor)? as usize;
    let mut records = Vec::with_capacity(count);
    for _ in 0..count {
        let global_version = read_u64(bytes, &mut cursor)?;
        let memory_version = read_u64(bytes, &mut cursor)?;
        let len = read_u32(bytes, &mut cursor)? as usize;
        let end = cursor
            .checked_add(len)
            .ok_or("completion record overflow")?;
        let encoded = bytes
            .get(cursor..end)
            .ok_or("truncated completion record")?;
        cursor = end;
        let mut record = decode_record(encoded)
            .map_err(|_| "invalid completion memory record")?
            .ok_or("missing completion memory record")?;
        record.global_version = global_version;
        record.memory_version = memory_version;
        records.push(record);
    }
    if cursor != bytes.len() {
        return Err("Insomnia completion trailing bytes");
    }
    Ok(Some(InsomniaCompletion {
        episode_id,
        attempt,
        started_at_ns,
        completed_at_ns,
        extractor_model,
        extractor_version,
        rejected_count,
        memory_ids,
        records,
    }))
}

fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), &'static str> {
    let len = u32::try_from(value.len()).map_err(|_| "completion string too large")?;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn read_string(bytes: &[u8], cursor: &mut usize) -> Result<String, &'static str> {
    let len = read_u32(bytes, cursor)? as usize;
    let end = cursor
        .checked_add(len)
        .ok_or("completion string overflow")?;
    let raw = bytes
        .get(*cursor..end)
        .ok_or("truncated completion string")?;
    *cursor = end;
    String::from_utf8(raw.to_vec()).map_err(|_| "invalid completion utf8")
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, &'static str> {
    let end = cursor.checked_add(4).ok_or("completion integer overflow")?;
    let raw = bytes.get(*cursor..end).ok_or("truncated completion u32")?;
    *cursor = end;
    Ok(u32::from_le_bytes(raw.try_into().unwrap()))
}

fn read_u64(bytes: &[u8], cursor: &mut usize) -> Result<u64, &'static str> {
    let end = cursor.checked_add(8).ok_or("completion integer overflow")?;
    let raw = bytes.get(*cursor..end).ok_or("truncated completion u64")?;
    *cursor = end;
    Ok(u64::from_le_bytes(raw.try_into().unwrap()))
}
