use crate::{Community, CommunityError, CommunityId, CommunitySnapshot, MemoryId};

const SNAPSHOT_MAGIC: &[u8; 8] = b"CVACOMM1";
const SNAPSHOT_SCHEMA: u32 = 1;
const HEADER_LEN: usize = 60;
const COMMUNITY_HEADER_LEN: usize = 36;

pub(crate) fn encode_snapshot(snapshot: &CommunitySnapshot) -> Result<Vec<u8>, CommunityError> {
    let member_count = snapshot
        .communities
        .iter()
        .try_fold(0_usize, |total, community| {
            total.checked_add(community.members.len())
        })
        .ok_or(CommunityError::SizeOverflow)?;
    let member_bytes = member_count
        .checked_mul(32)
        .ok_or(CommunityError::SizeOverflow)?;
    let community_bytes = snapshot
        .communities
        .len()
        .checked_mul(COMMUNITY_HEADER_LEN)
        .ok_or(CommunityError::SizeOverflow)?;
    let capacity = HEADER_LEN
        .checked_add(community_bytes)
        .and_then(|value| value.checked_add(member_bytes))
        .ok_or(CommunityError::SizeOverflow)?;
    let mut out = Vec::with_capacity(capacity);
    out.extend_from_slice(SNAPSHOT_MAGIC);
    out.extend_from_slice(&SNAPSHOT_SCHEMA.to_le_bytes());
    out.extend_from_slice(&snapshot.generation.to_le_bytes());
    out.extend_from_slice(&snapshot.derived_graph_version.to_le_bytes());
    out.extend_from_slice(&snapshot.algorithm_version.to_le_bytes());
    out.extend_from_slice(&snapshot.seed.to_le_bytes());
    out.extend_from_slice(&snapshot.resolution.to_le_bytes());
    out.extend_from_slice(&snapshot.quality.to_le_bytes());
    let count =
        u32::try_from(snapshot.communities.len()).map_err(|_| CommunityError::SizeOverflow)?;
    out.extend_from_slice(&count.to_le_bytes());
    for community in &snapshot.communities {
        out.extend_from_slice(&community.id.0);
        let members =
            u32::try_from(community.members.len()).map_err(|_| CommunityError::SizeOverflow)?;
        out.extend_from_slice(&members.to_le_bytes());
        for member in &community.members {
            out.extend_from_slice(&member.0);
        }
    }
    Ok(out)
}

pub(crate) fn decode_snapshot(bytes: &[u8]) -> Result<Option<CommunitySnapshot>, CommunityError> {
    if !bytes.starts_with(SNAPSHOT_MAGIC) {
        return Ok(None);
    }
    if bytes.len() < HEADER_LEN || read_u32(bytes, 8)? != SNAPSHOT_SCHEMA {
        return Err(CommunityError::CorruptRecord("snapshot header"));
    }
    let generation = read_u64(bytes, 12)?;
    let derived_graph_version = read_u64(bytes, 20)?;
    let algorithm_version = read_u32(bytes, 28)?;
    let seed = read_u64(bytes, 32)?;
    let resolution = read_f64(bytes, 40)?;
    let quality = read_f64(bytes, 48)?;
    let count = usize::try_from(read_u32(bytes, 56)?).map_err(|_| CommunityError::SizeOverflow)?;
    let mut offset = HEADER_LEN;
    let mut communities = Vec::with_capacity(count);
    for _ in 0..count {
        let header = bytes
            .get(offset..offset + COMMUNITY_HEADER_LEN)
            .ok_or(CommunityError::CorruptRecord("community header"))?;
        let id = CommunityId(header[..32].try_into().expect("community id width"));
        let member_count = usize::try_from(u32::from_le_bytes(header[32..36].try_into().unwrap()))
            .map_err(|_| CommunityError::SizeOverflow)?;
        offset = offset
            .checked_add(COMMUNITY_HEADER_LEN)
            .ok_or(CommunityError::SizeOverflow)?;
        let member_bytes = member_count
            .checked_mul(32)
            .ok_or(CommunityError::SizeOverflow)?;
        let end = offset
            .checked_add(member_bytes)
            .ok_or(CommunityError::SizeOverflow)?;
        let raw = bytes
            .get(offset..end)
            .ok_or(CommunityError::CorruptRecord("community members"))?;
        let members = raw
            .chunks_exact(32)
            .map(|chunk| MemoryId(chunk.try_into().expect("memory id width")))
            .collect();
        communities.push(Community { id, members });
        offset = end;
    }
    if offset != bytes.len() {
        return Err(CommunityError::CorruptRecord("snapshot trailing bytes"));
    }
    Ok(Some(CommunitySnapshot {
        generation,
        derived_graph_version,
        algorithm_version,
        seed,
        resolution,
        quality,
        communities,
    }))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, CommunityError> {
    let raw = bytes
        .get(offset..offset + 4)
        .ok_or(CommunityError::CorruptRecord("u32"))?;
    Ok(u32::from_le_bytes(raw.try_into().expect("u32 width")))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, CommunityError> {
    let raw = bytes
        .get(offset..offset + 8)
        .ok_or(CommunityError::CorruptRecord("u64"))?;
    Ok(u64::from_le_bytes(raw.try_into().expect("u64 width")))
}

fn read_f64(bytes: &[u8], offset: usize) -> Result<f64, CommunityError> {
    let raw = bytes
        .get(offset..offset + 8)
        .ok_or(CommunityError::CorruptRecord("f64"))?;
    Ok(f64::from_le_bytes(raw.try_into().expect("f64 width")))
}
