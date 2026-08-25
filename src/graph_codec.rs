use crate::{ChunkRef, GraphError, GraphRelationKind, MemoryId};
use arcana_graph::NodeId;

const FORMAT_MAGIC: &[u8; 8] = b"CVAGFMT1";
const FORMAT_SCHEMA: u32 = 1;
const NODE_MAGIC: &[u8; 8] = b"CVAGNODE";
const MUTATION_MAGIC: &[u8; 8] = b"CVAGMUT1";
const BATCH_MAGIC: &[u8; 8] = b"CVAGBAT1";
const VERSION_MAGIC: &[u8; 8] = b"CVAGVER1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct GraphNodePayload {
    pub memory_id: MemoryId,
    pub node_id: NodeId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct GraphMutationPayload {
    pub source: MemoryId,
    pub target: MemoryId,
    pub kind: GraphRelationKind,
    pub active: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct GraphVersionPayload {
    pub global_version: u64,
    pub graph_version: u64,
    pub mutation: ChunkRef,
}

pub(crate) fn encode_format() -> [u8; 12] {
    let mut out = [0_u8; 12];
    out[..8].copy_from_slice(FORMAT_MAGIC);
    out[8..].copy_from_slice(&FORMAT_SCHEMA.to_le_bytes());
    out
}

pub(crate) fn decode_format(bytes: &[u8]) -> Result<bool, GraphError> {
    if !bytes.starts_with(FORMAT_MAGIC) {
        return Ok(false);
    }
    if bytes.len() != 12 || read_u32(bytes, 8)? != FORMAT_SCHEMA {
        return Err(GraphError::CorruptRecord("format marker"));
    }
    Ok(true)
}

pub(crate) fn encode_node(node: GraphNodePayload) -> [u8; 44] {
    let mut out = [0_u8; 44];
    out[..8].copy_from_slice(NODE_MAGIC);
    out[8..40].copy_from_slice(&node.memory_id.0);
    out[40..44].copy_from_slice(&node.node_id.0.to_le_bytes());
    out
}

pub(crate) fn decode_node(bytes: &[u8]) -> Result<Option<GraphNodePayload>, GraphError> {
    if !bytes.starts_with(NODE_MAGIC) {
        return Ok(None);
    }
    if bytes.len() != 44 {
        return Err(GraphError::CorruptRecord("node mapping"));
    }
    Ok(Some(GraphNodePayload {
        memory_id: MemoryId(bytes[8..40].try_into().expect("memory id width")),
        node_id: NodeId(read_u32(bytes, 40)?),
    }))
}

pub(crate) fn encode_mutation(mutation: GraphMutationPayload) -> [u8; 75] {
    let mut out = [0_u8; 75];
    out[..8].copy_from_slice(MUTATION_MAGIC);
    out[8..40].copy_from_slice(&mutation.source.0);
    out[40..72].copy_from_slice(&mutation.target.0);
    out[72..74].copy_from_slice(&mutation.kind.code().to_le_bytes());
    out[74] = u8::from(mutation.active);
    out
}

pub(crate) fn decode_mutation(bytes: &[u8]) -> Result<Option<GraphMutationPayload>, GraphError> {
    if !bytes.starts_with(MUTATION_MAGIC) {
        return Ok(None);
    }
    if bytes.len() != 75 || bytes[74] > 1 {
        return Err(GraphError::CorruptRecord("relationship mutation"));
    }
    Ok(Some(decode_mutation_body(bytes, 8)?))
}

pub(crate) fn encode_batch(mutations: &[GraphMutationPayload]) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + mutations.len() * 67);
    out.extend_from_slice(BATCH_MAGIC);
    out.extend_from_slice(&(mutations.len() as u32).to_le_bytes());
    for mutation in mutations {
        out.extend_from_slice(&mutation.source.0);
        out.extend_from_slice(&mutation.target.0);
        out.extend_from_slice(&mutation.kind.code().to_le_bytes());
        out.push(u8::from(mutation.active));
    }
    out
}

pub(crate) fn decode_batch(bytes: &[u8]) -> Result<Option<Vec<GraphMutationPayload>>, GraphError> {
    if !bytes.starts_with(BATCH_MAGIC) {
        return Ok(None);
    }
    if bytes.len() < 12 {
        return Err(GraphError::CorruptRecord("relationship batch"));
    }
    let count = read_u32(bytes, 8)? as usize;
    let expected = 12_usize
        .checked_add(
            count
                .checked_mul(67)
                .ok_or(GraphError::CorruptRecord("relationship batch"))?,
        )
        .ok_or(GraphError::CorruptRecord("relationship batch"))?;
    if count == 0 || bytes.len() != expected {
        return Err(GraphError::CorruptRecord("relationship batch"));
    }
    let mut mutations = Vec::with_capacity(count);
    for index in 0..count {
        mutations.push(decode_mutation_body(bytes, 12 + index * 67)?);
    }
    Ok(Some(mutations))
}

fn decode_mutation_body(bytes: &[u8], offset: usize) -> Result<GraphMutationPayload, GraphError> {
    let active_offset = offset + 66;
    if bytes
        .get(active_offset)
        .copied()
        .is_none_or(|active| active > 1)
    {
        return Err(GraphError::CorruptRecord("relationship mutation"));
    }
    let code = read_u16(bytes, offset + 64)?;
    let kind = GraphRelationKind::from_code(code).ok_or(GraphError::UnknownRelationKind(code))?;
    Ok(GraphMutationPayload {
        source: MemoryId(
            bytes[offset..offset + 32]
                .try_into()
                .map_err(|_| GraphError::CorruptRecord("relationship mutation"))?,
        ),
        target: MemoryId(
            bytes[offset + 32..offset + 64]
                .try_into()
                .map_err(|_| GraphError::CorruptRecord("relationship mutation"))?,
        ),
        kind,
        active: bytes[active_offset] != 0,
    })
}

pub(crate) fn encode_version(version: GraphVersionPayload) -> [u8; 40] {
    let mut out = [0_u8; 40];
    out[..8].copy_from_slice(VERSION_MAGIC);
    out[8..16].copy_from_slice(&version.global_version.to_le_bytes());
    out[16..24].copy_from_slice(&version.graph_version.to_le_bytes());
    out[24..32].copy_from_slice(&version.mutation.offset.to_le_bytes());
    out[32..40].copy_from_slice(&version.mutation.len.to_le_bytes());
    out
}

pub(crate) fn decode_version(bytes: &[u8]) -> Result<Option<GraphVersionPayload>, GraphError> {
    if !bytes.starts_with(VERSION_MAGIC) {
        return Ok(None);
    }
    if bytes.len() != 40 {
        return Err(GraphError::CorruptRecord("relationship version"));
    }
    Ok(Some(GraphVersionPayload {
        global_version: read_u64(bytes, 8)?,
        graph_version: read_u64(bytes, 16)?,
        mutation: ChunkRef {
            offset: read_u64(bytes, 24)?,
            len: read_u64(bytes, 32)?,
        },
    }))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, GraphError> {
    let raw = bytes
        .get(offset..offset + 2)
        .ok_or(GraphError::CorruptRecord("integer"))?;
    Ok(u16::from_le_bytes(raw.try_into().expect("u16 width")))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, GraphError> {
    let raw = bytes
        .get(offset..offset + 4)
        .ok_or(GraphError::CorruptRecord("integer"))?;
    Ok(u32::from_le_bytes(raw.try_into().expect("u32 width")))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, GraphError> {
    let raw = bytes
        .get(offset..offset + 8)
        .ok_or(GraphError::CorruptRecord("integer"))?;
    Ok(u64::from_le_bytes(raw.try_into().expect("u64 width")))
}
