use crate::MemoryError;

pub(crate) fn read_i64(bytes: &[u8], cursor: &mut usize) -> Result<i64, MemoryError> {
    let end = cursor.checked_add(8).ok_or(MemoryError::FieldTooLarge)?;
    let raw = bytes
        .get(*cursor..end)
        .ok_or(MemoryError::CorruptRecord("truncated i64"))?;
    *cursor = end;
    Ok(i64::from_le_bytes(raw.try_into().unwrap()))
}
