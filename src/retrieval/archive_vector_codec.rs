use crate::{ArchiveVectorError, ArchiveVectorId, FragmentId, PackedVectorId};

const FORMAT_MAGIC: &[u8; 8] = b"CVAAVFM1";
const FORMAT_SCHEMA: u32 = 1;
const OBJECT_MAGIC: &[u8; 8] = b"CVAAVEC1";
const OBJECT_HEADER_LEN: usize = 80;
const ID_LEN: usize = 32;

pub(crate) struct DecodedArchiveVectors {
    pub id: ArchiveVectorId,
    pub packed_vector_id: PackedVectorId,
    pub fragment_ids: Vec<FragmentId>,
}

pub(crate) fn encode_format() -> Vec<u8> {
    let mut bytes = Vec::with_capacity(12);
    bytes.extend_from_slice(FORMAT_MAGIC);
    bytes.extend_from_slice(&FORMAT_SCHEMA.to_le_bytes());
    bytes
}

pub(crate) fn decode_format(bytes: &[u8]) -> Result<bool, ArchiveVectorError> {
    if !bytes.starts_with(FORMAT_MAGIC) {
        return Ok(false);
    }
    if bytes.len() != 12 || read_u32(bytes, 8)? != FORMAT_SCHEMA {
        return Err(ArchiveVectorError::CorruptRecord(
            "Archive-Vector format marker",
        ));
    }
    Ok(true)
}

pub(crate) fn encode_object(
    id: ArchiveVectorId,
    packed_vector_id: PackedVectorId,
    fragment_ids: &[FragmentId],
) -> Result<Vec<u8>, ArchiveVectorError> {
    let rows = u64::try_from(fragment_ids.len()).map_err(|_| ArchiveVectorError::SizeOverflow)?;
    let mapping_bytes = fragment_ids
        .len()
        .checked_mul(ID_LEN)
        .ok_or(ArchiveVectorError::SizeOverflow)?;
    let mut bytes = Vec::with_capacity(
        OBJECT_HEADER_LEN
            .checked_add(mapping_bytes)
            .ok_or(ArchiveVectorError::SizeOverflow)?,
    );
    bytes.extend_from_slice(OBJECT_MAGIC);
    bytes.extend_from_slice(&id.0);
    bytes.extend_from_slice(&packed_vector_id.0);
    bytes.extend_from_slice(&rows.to_le_bytes());
    for fragment_id in fragment_ids {
        bytes.extend_from_slice(&fragment_id.0);
    }
    Ok(bytes)
}

pub(crate) fn decode_object(
    bytes: &[u8],
) -> Result<Option<DecodedArchiveVectors>, ArchiveVectorError> {
    if !bytes.starts_with(OBJECT_MAGIC) {
        return Ok(None);
    }
    if bytes.len() < OBJECT_HEADER_LEN {
        return Err(ArchiveVectorError::CorruptRecord(
            "Archive-Vector object header",
        ));
    }
    let id = ArchiveVectorId(bytes[8..40].try_into().expect("fixed Archive-Vector id"));
    let packed_vector_id =
        PackedVectorId(bytes[40..72].try_into().expect("fixed packed-vector id"));
    let rows = read_u64(bytes, 72)?;
    let rows = usize::try_from(rows).map_err(|_| ArchiveVectorError::SizeOverflow)?;
    let mapping_bytes = rows
        .checked_mul(ID_LEN)
        .ok_or(ArchiveVectorError::SizeOverflow)?;
    let expected_len = OBJECT_HEADER_LEN
        .checked_add(mapping_bytes)
        .ok_or(ArchiveVectorError::SizeOverflow)?;
    if bytes.len() != expected_len {
        return Err(ArchiveVectorError::CorruptRecord(
            "Archive-Vector row mapping length",
        ));
    }
    let mut fragment_ids = Vec::with_capacity(rows);
    for raw in bytes[OBJECT_HEADER_LEN..].chunks_exact(ID_LEN) {
        fragment_ids.push(FragmentId(raw.try_into().expect("fixed fragment id")));
    }
    Ok(Some(DecodedArchiveVectors {
        id,
        packed_vector_id,
        fragment_ids,
    }))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, ArchiveVectorError> {
    let raw = bytes
        .get(offset..offset + 4)
        .ok_or(ArchiveVectorError::CorruptRecord("Archive-Vector integer"))?;
    Ok(u32::from_le_bytes(raw.try_into().expect("u32 width")))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, ArchiveVectorError> {
    let raw = bytes
        .get(offset..offset + 8)
        .ok_or(ArchiveVectorError::CorruptRecord("Archive-Vector integer"))?;
    Ok(u64::from_le_bytes(raw.try_into().expect("u64 width")))
}
