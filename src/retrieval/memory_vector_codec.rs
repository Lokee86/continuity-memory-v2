use crate::{
    CompatibilityProfileId, MemoryBodyId, MemoryVectorError, MemoryVectorId, PackedVectorId,
};

const FORMAT_MAGIC: &[u8; 8] = b"CVAMVFM1";
const FORMAT_SCHEMA: u32 = 1;
const OBJECT_MAGIC: &[u8; 8] = b"CVAMVEC1";
const OBJECT_HEADER_LEN: usize = 112;
const ID_LEN: usize = 32;

pub(crate) struct DecodedMemoryVectors {
    pub id: MemoryVectorId,
    pub compatibility_profile_id: CompatibilityProfileId,
    pub packed_vector_id: PackedVectorId,
    pub memory_body_ids: Vec<MemoryBodyId>,
}

pub(crate) fn encode_format() -> Vec<u8> {
    let mut bytes = Vec::with_capacity(12);
    bytes.extend_from_slice(FORMAT_MAGIC);
    bytes.extend_from_slice(&FORMAT_SCHEMA.to_le_bytes());
    bytes
}

pub(crate) fn decode_format(bytes: &[u8]) -> Result<bool, MemoryVectorError> {
    if !bytes.starts_with(FORMAT_MAGIC) {
        return Ok(false);
    }
    if bytes.len() != 12 || read_u32(bytes, 8)? != FORMAT_SCHEMA {
        return Err(MemoryVectorError::CorruptRecord(
            "Memory-Vector format marker",
        ));
    }
    Ok(true)
}

pub(crate) fn encode_object(
    id: MemoryVectorId,
    compatibility_profile_id: CompatibilityProfileId,
    packed_vector_id: PackedVectorId,
    memory_body_ids: &[MemoryBodyId],
) -> Result<Vec<u8>, MemoryVectorError> {
    let rows = u64::try_from(memory_body_ids.len()).map_err(|_| MemoryVectorError::SizeOverflow)?;
    let mapping_bytes = memory_body_ids
        .len()
        .checked_mul(ID_LEN)
        .ok_or(MemoryVectorError::SizeOverflow)?;
    let mut bytes = Vec::with_capacity(
        OBJECT_HEADER_LEN
            .checked_add(mapping_bytes)
            .ok_or(MemoryVectorError::SizeOverflow)?,
    );
    bytes.extend_from_slice(OBJECT_MAGIC);
    bytes.extend_from_slice(&id.0);
    bytes.extend_from_slice(&compatibility_profile_id.0);
    bytes.extend_from_slice(&packed_vector_id.0);
    bytes.extend_from_slice(&rows.to_le_bytes());
    for body_id in memory_body_ids {
        bytes.extend_from_slice(&body_id.0);
    }
    Ok(bytes)
}

pub(crate) fn decode_object(
    bytes: &[u8],
) -> Result<Option<DecodedMemoryVectors>, MemoryVectorError> {
    if !bytes.starts_with(OBJECT_MAGIC) {
        return Ok(None);
    }
    if bytes.len() < OBJECT_HEADER_LEN {
        return Err(MemoryVectorError::CorruptRecord(
            "Memory-Vector object header",
        ));
    }
    let id = MemoryVectorId(bytes[8..40].try_into().expect("fixed Memory-Vector id"));
    let compatibility_profile_id = CompatibilityProfileId(
        bytes[40..72]
            .try_into()
            .expect("fixed compatibility-profile id"),
    );
    let packed_vector_id =
        PackedVectorId(bytes[72..104].try_into().expect("fixed packed-vector id"));
    let rows =
        usize::try_from(read_u64(bytes, 104)?).map_err(|_| MemoryVectorError::SizeOverflow)?;
    let mapping_bytes = rows
        .checked_mul(ID_LEN)
        .ok_or(MemoryVectorError::SizeOverflow)?;
    let expected_len = OBJECT_HEADER_LEN
        .checked_add(mapping_bytes)
        .ok_or(MemoryVectorError::SizeOverflow)?;
    if bytes.len() != expected_len {
        return Err(MemoryVectorError::CorruptRecord(
            "Memory-Vector row mapping length",
        ));
    }
    let mut memory_body_ids = Vec::with_capacity(rows);
    for raw in bytes[OBJECT_HEADER_LEN..].chunks_exact(ID_LEN) {
        memory_body_ids.push(MemoryBodyId(raw.try_into().expect("fixed memory-body id")));
    }
    Ok(Some(DecodedMemoryVectors {
        id,
        compatibility_profile_id,
        packed_vector_id,
        memory_body_ids,
    }))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, MemoryVectorError> {
    let raw = bytes
        .get(offset..offset + 4)
        .ok_or(MemoryVectorError::CorruptRecord("Memory-Vector integer"))?;
    Ok(u32::from_le_bytes(raw.try_into().expect("u32 width")))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, MemoryVectorError> {
    let raw = bytes
        .get(offset..offset + 8)
        .ok_or(MemoryVectorError::CorruptRecord("Memory-Vector integer"))?;
    Ok(u64::from_le_bytes(raw.try_into().expect("u64 width")))
}
