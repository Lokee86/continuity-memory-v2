use crate::{ArchiveError, ConversationMetadata};

pub(crate) const CONVERSATION_METADATA_MAGIC: [u8; 8] = *b"CVACONV1";

pub(crate) fn encode_conversation_metadata(
    metadata: &ConversationMetadata,
) -> Result<Vec<u8>, ArchiveError> {
    let mut out = Vec::with_capacity(64 + metadata.conversation_id.len());
    out.extend_from_slice(&CONVERSATION_METADATA_MAGIC);
    write_string(&mut out, &metadata.conversation_id)?;
    write_string(&mut out, metadata.title.as_deref().unwrap_or(""))?;
    Ok(out)
}

pub(crate) fn decode_conversation_metadata(
    bytes: &[u8],
) -> Result<ConversationMetadata, ArchiveError> {
    if bytes.len() < 8 || bytes[..8] != CONVERSATION_METADATA_MAGIC {
        return Err(ArchiveError::CorruptRecord(
            "invalid conversation metadata record",
        ));
    }
    let mut cursor = 8;
    let conversation_id = read_string(bytes, &mut cursor)?;
    let title = read_string(bytes, &mut cursor)?;
    if cursor != bytes.len() {
        return Err(ArchiveError::CorruptRecord(
            "conversation metadata trailing bytes",
        ));
    }
    Ok(ConversationMetadata {
        conversation_id,
        title: (!title.is_empty()).then_some(title),
    })
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
        .ok_or(ArchiveError::CorruptRecord(
            "missing conversation metadata string length",
        ))?;
    let len = u32::from_le_bytes(raw.try_into().unwrap()) as usize;
    *cursor = length_end;
    let end = cursor.checked_add(len).ok_or(ArchiveError::FieldTooLarge)?;
    let value = bytes.get(*cursor..end).ok_or(ArchiveError::CorruptRecord(
        "truncated conversation metadata string",
    ))?;
    *cursor = end;
    String::from_utf8(value.to_vec()).map_err(|_| ArchiveError::InvalidUtf8)
}
