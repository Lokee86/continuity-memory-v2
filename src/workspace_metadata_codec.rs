use crate::{WorkspaceMetadata, WorkspaceMetadataError};

const FORMAT_MAGIC: [u8; 8] = *b"CVAWKFM1";
const RECORD_MAGIC: [u8; 8] = *b"CVAWKSP1";

pub(crate) fn encode_format() -> Vec<u8> {
    FORMAT_MAGIC.to_vec()
}

pub(crate) fn decode_format(bytes: &[u8]) -> Result<bool, WorkspaceMetadataError> {
    if bytes.len() < 8 || bytes[..8] != FORMAT_MAGIC {
        return Ok(false);
    }
    if bytes.len() != 8 {
        return Err(WorkspaceMetadataError::CorruptRecord(
            "invalid workspace format marker",
        ));
    }
    Ok(true)
}

pub(crate) fn encode_metadata(
    metadata: &WorkspaceMetadata,
) -> Result<Vec<u8>, WorkspaceMetadataError> {
    metadata.validate()?;
    let mut out = Vec::with_capacity(
        20 + metadata.id.len() + metadata.name.len() + metadata.workspace_type.len(),
    );
    out.extend_from_slice(&RECORD_MAGIC);
    write_string(&mut out, "id", &metadata.id)?;
    write_string(&mut out, "name", &metadata.name)?;
    write_string(&mut out, "workspace_type", &metadata.workspace_type)?;
    Ok(out)
}

pub(crate) fn decode_metadata(
    bytes: &[u8],
) -> Result<Option<WorkspaceMetadata>, WorkspaceMetadataError> {
    if bytes.len() < 8 || bytes[..8] != RECORD_MAGIC {
        return Ok(None);
    }
    let mut cursor = 8;
    let metadata = WorkspaceMetadata {
        id: read_string(bytes, &mut cursor)?,
        name: read_string(bytes, &mut cursor)?,
        workspace_type: read_string(bytes, &mut cursor)?,
    };
    if cursor != bytes.len() {
        return Err(WorkspaceMetadataError::CorruptRecord(
            "workspace metadata trailing bytes",
        ));
    }
    metadata.validate()?;
    Ok(Some(metadata))
}

fn write_string(
    out: &mut Vec<u8>,
    field: &'static str,
    value: &str,
) -> Result<(), WorkspaceMetadataError> {
    let len =
        u32::try_from(value.len()).map_err(|_| WorkspaceMetadataError::FieldTooLarge(field))?;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn read_string(bytes: &[u8], cursor: &mut usize) -> Result<String, WorkspaceMetadataError> {
    let length_end = cursor
        .checked_add(4)
        .ok_or(WorkspaceMetadataError::CorruptRecord(
            "string length overflow",
        ))?;
    let raw = bytes
        .get(*cursor..length_end)
        .ok_or(WorkspaceMetadataError::CorruptRecord(
            "missing string length",
        ))?;
    let len = u32::from_le_bytes(raw.try_into().unwrap()) as usize;
    *cursor = length_end;
    let end = cursor
        .checked_add(len)
        .ok_or(WorkspaceMetadataError::CorruptRecord(
            "string length overflow",
        ))?;
    let value = bytes
        .get(*cursor..end)
        .ok_or(WorkspaceMetadataError::CorruptRecord("truncated string"))?;
    *cursor = end;
    String::from_utf8(value.to_vec()).map_err(|_| WorkspaceMetadataError::InvalidUtf8)
}
