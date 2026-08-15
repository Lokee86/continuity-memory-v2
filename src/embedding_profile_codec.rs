use crate::{EmbeddingProfile, EmbeddingProfileError, EmbeddingProfileId, VectorNormalization};

const FORMAT_MAGIC: &[u8; 8] = b"CVAEPFM1";
const FORMAT_SCHEMA: u32 = 1;
const PROFILE_MAGIC: &[u8; 8] = b"CVAEPRO1";
const FIXED_LEN: usize = 84;

pub(crate) fn encode_format() -> [u8; 12] {
    let mut out = [0_u8; 12];
    out[..8].copy_from_slice(FORMAT_MAGIC);
    out[8..].copy_from_slice(&FORMAT_SCHEMA.to_le_bytes());
    out
}

pub(crate) fn decode_format(bytes: &[u8]) -> Result<bool, EmbeddingProfileError> {
    if !bytes.starts_with(FORMAT_MAGIC) {
        return Ok(false);
    }
    if bytes.len() != 12 || read_u32(bytes, 8)? != FORMAT_SCHEMA {
        return Err(EmbeddingProfileError::CorruptRecord(
            "profile format marker",
        ));
    }
    Ok(true)
}

pub(crate) fn encode_profile(profile: &EmbeddingProfile) -> Result<Vec<u8>, EmbeddingProfileError> {
    let mut out = Vec::with_capacity(
        FIXED_LEN + 12 + profile.provider.len() + profile.model.len() + profile.revision.len(),
    );
    out.extend_from_slice(PROFILE_MAGIC);
    out.extend_from_slice(&profile.id.0);
    out.extend_from_slice(&profile.dimensions.to_le_bytes());
    out.push(profile.normalization.tag());
    out.extend_from_slice(&[0; 3]);
    out.extend_from_slice(&profile.probe_suite_version.to_le_bytes());
    out.extend_from_slice(&profile.behavior_fingerprint);
    write_string(&mut out, &profile.provider)?;
    write_string(&mut out, &profile.model)?;
    write_string(&mut out, &profile.revision)?;
    Ok(out)
}

pub(crate) fn decode_profile(
    bytes: &[u8],
) -> Result<Option<EmbeddingProfile>, EmbeddingProfileError> {
    if !bytes.starts_with(PROFILE_MAGIC) {
        return Ok(None);
    }
    if bytes.len() < FIXED_LEN || bytes[45..48] != [0, 0, 0] {
        return Err(EmbeddingProfileError::CorruptRecord("profile header"));
    }
    let id = EmbeddingProfileId(bytes[8..40].try_into().expect("profile id"));
    let dimensions = read_u32(bytes, 40)?;
    let normalization = VectorNormalization::from_tag(bytes[44]).ok_or(
        EmbeddingProfileError::CorruptRecord("profile normalization"),
    )?;
    let probe_suite_version = read_u32(bytes, 48)?;
    let behavior_fingerprint = bytes[52..84].try_into().expect("fingerprint");
    let mut offset = FIXED_LEN;
    let provider = read_string(bytes, &mut offset)?;
    let model = read_string(bytes, &mut offset)?;
    let revision = read_string(bytes, &mut offset)?;
    if offset != bytes.len() {
        return Err(EmbeddingProfileError::CorruptRecord(
            "profile trailing bytes",
        ));
    }
    Ok(Some(EmbeddingProfile {
        id,
        provider,
        model,
        revision,
        dimensions,
        normalization,
        probe_suite_version,
        behavior_fingerprint,
    }))
}

fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), EmbeddingProfileError> {
    let len = u32::try_from(value.len()).map_err(|_| EmbeddingProfileError::SizeOverflow)?;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn read_string(bytes: &[u8], offset: &mut usize) -> Result<String, EmbeddingProfileError> {
    let len = usize::try_from(read_u32(bytes, *offset)?)
        .map_err(|_| EmbeddingProfileError::SizeOverflow)?;
    *offset = offset
        .checked_add(4)
        .ok_or(EmbeddingProfileError::SizeOverflow)?;
    let end = offset
        .checked_add(len)
        .ok_or(EmbeddingProfileError::SizeOverflow)?;
    let raw = bytes
        .get(*offset..end)
        .ok_or(EmbeddingProfileError::CorruptRecord("profile string"))?;
    *offset = end;
    String::from_utf8(raw.to_vec())
        .map_err(|_| EmbeddingProfileError::CorruptRecord("profile utf8"))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, EmbeddingProfileError> {
    let raw = bytes
        .get(offset..offset + 4)
        .ok_or(EmbeddingProfileError::CorruptRecord("profile integer"))?;
    Ok(u32::from_le_bytes(raw.try_into().expect("u32 width")))
}
