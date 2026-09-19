use crate::{ArchiveError, Episode, EpisodeBoundary, EpisodeId, EpisodeOrigin};

pub const EPISODE_MAGIC: [u8; 8] = *b"CVAEPIS1";

pub fn encode_episode(episode: &Episode) -> Result<Vec<u8>, ArchiveError> {
    let mut out = Vec::with_capacity(128);
    out.extend_from_slice(&EPISODE_MAGIC);
    out.extend_from_slice(&episode.id.0);
    out.push(episode.origin.tag());
    out.push(episode.boundary.tag());
    out.extend_from_slice(&episode.source_through_ns.to_le_bytes());
    out.extend_from_slice(&episode.finalized_at_ns.to_le_bytes());
    write_string(&mut out, &episode.conversation_id)?;
    write_string(&mut out, &episode.start_node_id)?;
    write_string(&mut out, &episode.end_node_id)?;
    Ok(out)
}

pub fn decode_episode(bytes: &[u8]) -> Result<Option<Episode>, ArchiveError> {
    if bytes.len() < 8 || bytes[..8] != EPISODE_MAGIC {
        return Ok(None);
    }
    if bytes.len() < 58 {
        return Err(ArchiveError::CorruptRecord("short episode record"));
    }
    let id = EpisodeId(bytes[8..40].try_into().unwrap());
    let origin = EpisodeOrigin::from_tag(bytes[40])
        .ok_or(ArchiveError::CorruptRecord("invalid episode origin"))?;
    let boundary = EpisodeBoundary::from_tag(bytes[41])
        .ok_or(ArchiveError::CorruptRecord("invalid episode boundary"))?;
    let source_through_ns = i64::from_le_bytes(bytes[42..50].try_into().unwrap());
    let finalized_at_ns = i64::from_le_bytes(bytes[50..58].try_into().unwrap());
    let mut cursor = 58;
    let conversation_id = read_string(bytes, &mut cursor)?;
    let start_node_id = read_string(bytes, &mut cursor)?;
    let end_node_id = read_string(bytes, &mut cursor)?;
    if cursor != bytes.len() {
        return Err(ArchiveError::CorruptRecord("episode trailing bytes"));
    }
    Ok(Some(Episode {
        id,
        conversation_id,
        start_node_id,
        end_node_id,
        origin,
        boundary,
        source_through_ns,
        finalized_at_ns,
    }))
}

fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), ArchiveError> {
    let len = u32::try_from(value.len()).map_err(|_| ArchiveError::FieldTooLarge)?;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn read_string(bytes: &[u8], cursor: &mut usize) -> Result<String, ArchiveError> {
    let length_end = cursor.checked_add(4).ok_or(ArchiveError::FieldTooLarge)?;
    let raw = bytes
        .get(*cursor..length_end)
        .ok_or(ArchiveError::CorruptRecord("missing episode string length"))?;
    let len = u32::from_le_bytes(raw.try_into().unwrap()) as usize;
    *cursor = length_end;
    let end = cursor.checked_add(len).ok_or(ArchiveError::FieldTooLarge)?;
    let value = bytes
        .get(*cursor..end)
        .ok_or(ArchiveError::CorruptRecord("truncated episode string"))?;
    *cursor = end;
    String::from_utf8(value.to_vec()).map_err(|_| ArchiveError::InvalidUtf8)
}
