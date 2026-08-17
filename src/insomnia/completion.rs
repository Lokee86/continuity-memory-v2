use crate::EpisodeId;
use crate::MemoryBodyId;
use crate::memory_codec::{decode_record, encode_record};
use crate::memory_model::MemoryRecord;

const COMPLETION_MAGIC_V1: [u8; 8] = *b"CVAINSC1";
const COMPLETION_MAGIC_V2: [u8; 8] = *b"CVAINSC2";

#[derive(Clone, Debug)]
pub(crate) struct InsomniaCompletionBody {
    pub(crate) id: MemoryBodyId,
    pub(crate) bytes: Vec<u8>,
}

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
    pub(crate) global_version_start: u64,
    pub(crate) bodies: Vec<InsomniaCompletionBody>,
    pub(crate) records: Vec<MemoryRecord>,
}

pub(crate) fn encode_completion(value: &InsomniaCompletion) -> Result<Vec<u8>, &'static str> {
    let global_version_count =
        u32::try_from(value.records.len()).map_err(|_| "too many completion versions")?;
    if global_version_count > 0 && value.global_version_start == 0 {
        return Err("invalid completion global version start");
    }
    let mut out = Vec::with_capacity(
        192 + value.records.len() * 256
            + value
                .bodies
                .iter()
                .map(|body| body.bytes.len())
                .sum::<usize>(),
    );
    out.extend_from_slice(&COMPLETION_MAGIC_V2);
    out.extend_from_slice(&value.episode_id.0);
    out.extend_from_slice(&value.attempt.to_le_bytes());
    out.extend_from_slice(&value.started_at_ns.to_le_bytes());
    out.extend_from_slice(&value.completed_at_ns.to_le_bytes());
    out.extend_from_slice(&value.rejected_count.to_le_bytes());
    out.extend_from_slice(&value.global_version_start.to_le_bytes());
    out.extend_from_slice(&global_version_count.to_le_bytes());
    write_string(&mut out, &value.extractor_model)?;
    write_string(&mut out, &value.extractor_version)?;
    let memory_count =
        u32::try_from(value.memory_ids.len()).map_err(|_| "too many completion memory ids")?;
    out.extend_from_slice(&memory_count.to_le_bytes());
    for id in &value.memory_ids {
        out.extend_from_slice(&id.0);
    }
    let body_count = u32::try_from(value.bodies.len()).map_err(|_| "too many completion bodies")?;
    out.extend_from_slice(&body_count.to_le_bytes());
    for body in &value.bodies {
        out.extend_from_slice(&body.id.0);
        let len = u32::try_from(body.bytes.len()).map_err(|_| "completion body too large")?;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&body.bytes);
    }
    let count = u32::try_from(value.records.len()).map_err(|_| "too many completion records")?;
    out.extend_from_slice(&count.to_le_bytes());
    for (index, record) in value.records.iter().enumerate() {
        let expected_global = value
            .global_version_start
            .checked_add(u64::try_from(index).map_err(|_| "completion version overflow")?)
            .ok_or("completion version overflow")?;
        if record.global_version != expected_global {
            return Err("non-contiguous completion global versions");
        }
        out.extend_from_slice(&record.global_version.to_le_bytes());
        out.extend_from_slice(&record.memory_version.to_le_bytes());
        let encoded = encode_record(record).map_err(|_| "invalid completion memory record")?;
        let len = u32::try_from(encoded.len()).map_err(|_| "completion record too large")?;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&encoded);
    }
    Ok(out)
}

pub(crate) fn decode_embedded_version_range(
    bytes: &[u8],
) -> Result<Option<(u64, u32)>, &'static str> {
    if bytes.len() < 8 || bytes[..8] != COMPLETION_MAGIC_V2 {
        return Ok(None);
    }
    if bytes.len() < 76 {
        return Err("short Insomnia v2 completion");
    }
    let start = u64::from_le_bytes(bytes[64..72].try_into().unwrap());
    let count = u32::from_le_bytes(bytes[72..76].try_into().unwrap());
    if count > 0 && start == 0 {
        return Err("invalid completion global version range");
    }
    Ok(Some((start, count)))
}

pub(crate) fn decode_completion(bytes: &[u8]) -> Result<Option<InsomniaCompletion>, &'static str> {
    if bytes.len() < 8 {
        return Ok(None);
    }
    if bytes[..8] == COMPLETION_MAGIC_V2 {
        return decode_completion_v2(bytes).map(Some);
    }
    if bytes[..8] == COMPLETION_MAGIC_V1 {
        return decode_completion_v1(bytes).map(Some);
    }
    Ok(None)
}

fn decode_completion_v2(bytes: &[u8]) -> Result<InsomniaCompletion, &'static str> {
    if bytes.len() < 84 {
        return Err("short Insomnia v2 completion");
    }
    let episode_id = EpisodeId(bytes[8..40].try_into().unwrap());
    let attempt = u32::from_le_bytes(bytes[40..44].try_into().unwrap());
    let started_at_ns = i64::from_le_bytes(bytes[44..52].try_into().unwrap());
    let completed_at_ns = i64::from_le_bytes(bytes[52..60].try_into().unwrap());
    let rejected_count = u32::from_le_bytes(bytes[60..64].try_into().unwrap());
    let global_version_start = u64::from_le_bytes(bytes[64..72].try_into().unwrap());
    let global_version_count = u32::from_le_bytes(bytes[72..76].try_into().unwrap());
    if global_version_count > 0 && global_version_start == 0 {
        return Err("invalid completion global version range");
    }
    let mut cursor = 76;
    let extractor_model = read_string(bytes, &mut cursor)?;
    let extractor_version = read_string(bytes, &mut cursor)?;
    let memory_ids = read_memory_ids(bytes, &mut cursor)?;
    let body_count = read_u32(bytes, &mut cursor)? as usize;
    let mut bodies = Vec::with_capacity(body_count);
    for _ in 0..body_count {
        let id_end = cursor
            .checked_add(32)
            .ok_or("completion body id overflow")?;
        let id_raw = bytes
            .get(cursor..id_end)
            .ok_or("truncated completion body id")?;
        cursor = id_end;
        let len = read_u32(bytes, &mut cursor)? as usize;
        let end = cursor.checked_add(len).ok_or("completion body overflow")?;
        let body = bytes
            .get(cursor..end)
            .ok_or("truncated completion body")?
            .to_vec();
        cursor = end;
        bodies.push(InsomniaCompletionBody {
            id: MemoryBodyId(id_raw.try_into().unwrap()),
            bytes: body,
        });
    }
    let records = read_records(bytes, &mut cursor)?;
    if global_version_count as usize != records.len() {
        return Err("completion global version count mismatch");
    }
    for (index, record) in records.iter().enumerate() {
        let expected = global_version_start
            .checked_add(u64::try_from(index).map_err(|_| "completion version overflow")?)
            .ok_or("completion version overflow")?;
        if record.global_version != expected {
            return Err("non-contiguous completion global versions");
        }
    }
    if cursor != bytes.len() {
        return Err("Insomnia completion trailing bytes");
    }
    Ok(InsomniaCompletion {
        episode_id,
        attempt,
        started_at_ns,
        completed_at_ns,
        extractor_model,
        extractor_version,
        rejected_count,
        memory_ids,
        global_version_start,
        bodies,
        records,
    })
}

fn decode_completion_v1(bytes: &[u8]) -> Result<InsomniaCompletion, &'static str> {
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
    let memory_ids = read_memory_ids(bytes, &mut cursor)?;
    let records = read_records(bytes, &mut cursor)?;
    if cursor != bytes.len() {
        return Err("Insomnia completion trailing bytes");
    }
    let global_version_start = records
        .first()
        .map(|record| record.global_version)
        .unwrap_or(0);
    Ok(InsomniaCompletion {
        episode_id,
        attempt,
        started_at_ns,
        completed_at_ns,
        extractor_model,
        extractor_version,
        rejected_count,
        memory_ids,
        global_version_start,
        bodies: Vec::new(),
        records,
    })
}

fn read_memory_ids(bytes: &[u8], cursor: &mut usize) -> Result<Vec<crate::MemoryId>, &'static str> {
    let memory_count = read_u32(bytes, cursor)? as usize;
    let mut memory_ids = Vec::with_capacity(memory_count);
    for _ in 0..memory_count {
        let end = cursor
            .checked_add(32)
            .ok_or("completion memory id overflow")?;
        let raw = bytes
            .get(*cursor..end)
            .ok_or("truncated completion memory id")?;
        *cursor = end;
        memory_ids.push(crate::MemoryId(raw.try_into().unwrap()));
    }
    Ok(memory_ids)
}

fn read_records(bytes: &[u8], cursor: &mut usize) -> Result<Vec<MemoryRecord>, &'static str> {
    let count = read_u32(bytes, cursor)? as usize;
    let mut records = Vec::with_capacity(count);
    for _ in 0..count {
        let global_version = read_u64(bytes, cursor)?;
        let memory_version = read_u64(bytes, cursor)?;
        let len = read_u32(bytes, cursor)? as usize;
        let end = cursor
            .checked_add(len)
            .ok_or("completion record overflow")?;
        let encoded = bytes
            .get(*cursor..end)
            .ok_or("truncated completion record")?;
        *cursor = end;
        let mut record = decode_record(encoded)
            .map_err(|_| "invalid completion memory record")?
            .ok_or("missing completion memory record")?;
        record.global_version = global_version;
        record.memory_version = memory_version;
        records.push(record);
    }
    Ok(records)
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
