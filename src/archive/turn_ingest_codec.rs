use crate::{ArchiveError, ContentId, FileId, IngestedTurn, Node, StoredFile};

pub(crate) const LEGACY_INGESTED_TURN_MAGIC: [u8; 8] = *b"CVATURN1";
pub(crate) const INGESTED_TURN_MAGIC: [u8; 8] = *b"CVATURN2";

pub(crate) fn encode_ingested_turn(turn: &IngestedTurn) -> Result<Vec<u8>, ArchiveError> {
    let count = u32::try_from(turn.attachments.len()).map_err(|_| ArchiveError::FieldTooLarge)?;
    let mut out = Vec::with_capacity(128 + turn.attachments.len() * 96);
    out.extend_from_slice(&INGESTED_TURN_MAGIC);
    out.extend_from_slice(&turn.node.timestamp_ns.to_le_bytes());
    out.extend_from_slice(&turn.node.content_id.0);
    write_string(&mut out, &turn.node.id)?;
    write_string(&mut out, &turn.node.conversation_id)?;
    write_string(&mut out, turn.node.parent_id.as_deref().unwrap_or(""))?;
    write_string(&mut out, &turn.node.role)?;
    write_string(&mut out, turn.node.principal_id.as_deref().unwrap_or(""))?;
    out.extend_from_slice(&count.to_le_bytes());
    for file in &turn.attachments {
        out.extend_from_slice(&file.id.0);
        out.extend_from_slice(&file.content_id.0);
        out.extend_from_slice(&file.byte_length.to_le_bytes());
        write_string(&mut out, &file.filename)?;
        write_string(&mut out, file.mime_type.as_deref().unwrap_or(""))?;
    }
    Ok(out)
}

pub(crate) fn decode_ingested_turn(bytes: &[u8]) -> Result<IngestedTurn, ArchiveError> {
    if bytes.len() < 52 {
        return Err(ArchiveError::CorruptRecord("short ingested turn"));
    }
    let timestamp_ns = i64::from_le_bytes(bytes[8..16].try_into().unwrap());
    let content_id = ContentId(bytes[16..48].try_into().unwrap());
    let mut cursor = 48;
    let id = read_string(bytes, &mut cursor)?;
    let conversation_id = read_string(bytes, &mut cursor)?;
    let parent = read_string(bytes, &mut cursor)?;
    let role = read_string(bytes, &mut cursor)?;
    let principal_id = if bytes[..8] == INGESTED_TURN_MAGIC {
        let principal = read_string(bytes, &mut cursor)?;
        (!principal.is_empty()).then_some(principal)
    } else {
        None
    };
    let count = read_u32(bytes, &mut cursor)? as usize;
    let mut attachments = Vec::with_capacity(count);
    for _ in 0..count {
        let fixed_end = cursor.checked_add(72).ok_or(ArchiveError::FieldTooLarge)?;
        let fixed = bytes
            .get(cursor..fixed_end)
            .ok_or(ArchiveError::CorruptRecord("truncated ingested attachment"))?;
        let file_id = FileId(fixed[0..32].try_into().unwrap());
        let file_content_id = ContentId(fixed[32..64].try_into().unwrap());
        let byte_length = u64::from_le_bytes(fixed[64..72].try_into().unwrap());
        cursor = fixed_end;
        let filename = read_string(bytes, &mut cursor)?;
        let mime_type = read_string(bytes, &mut cursor)?;
        attachments.push(StoredFile {
            id: file_id,
            content_id: file_content_id,
            filename,
            mime_type: (!mime_type.is_empty()).then_some(mime_type),
            byte_length,
        });
    }
    if cursor != bytes.len() {
        return Err(ArchiveError::CorruptRecord("ingested turn trailing bytes"));
    }
    Ok(IngestedTurn {
        node: Node {
            id,
            conversation_id,
            parent_id: (!parent.is_empty()).then_some(parent),
            role,
            principal_id,
            timestamp_ns,
            content_id,
        },
        attachments,
    })
}

fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), ArchiveError> {
    let len = u32::try_from(value.len()).map_err(|_| ArchiveError::FieldTooLarge)?;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, ArchiveError> {
    let end = cursor.checked_add(4).ok_or(ArchiveError::FieldTooLarge)?;
    let raw = bytes
        .get(*cursor..end)
        .ok_or(ArchiveError::CorruptRecord("missing u32"))?;
    *cursor = end;
    Ok(u32::from_le_bytes(raw.try_into().unwrap()))
}

fn read_string(bytes: &[u8], cursor: &mut usize) -> Result<String, ArchiveError> {
    let len = read_u32(bytes, cursor)? as usize;
    let end = cursor.checked_add(len).ok_or(ArchiveError::FieldTooLarge)?;
    let value = bytes
        .get(*cursor..end)
        .ok_or(ArchiveError::CorruptRecord("truncated string"))?;
    *cursor = end;
    String::from_utf8(value.to_vec()).map_err(|_| ArchiveError::InvalidUtf8)
}
