use crate::{
    GraphError, GraphRelationKind, GraphRelationOrigin, MemoryId, ObjectRef, SemanticNodeKind,
    SemanticNodeRef,
};
use arcana::NodeId;

const FORMAT_MAGIC: &[u8; 8] = b"CVAGFMT1";
const FORMAT_SCHEMA: u32 = 1;
const NODE_V1_MAGIC: &[u8; 8] = b"CVAGNODE";
const NODE_V2_MAGIC: &[u8; 8] = b"CVAGNOD2";
const MUTATION_MAGIC: &[u8; 8] = b"CVAGMUT1";
const BATCH_MAGIC: &[u8; 8] = b"CVAGBAT1";
const VERSION_MAGIC: &[u8; 8] = b"CVAGVER1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct GraphNodePayload {
    pub semantic_node: SemanticNodeRef,
    pub node_id: NodeId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct GraphMutationPayload {
    pub source: MemoryId,
    pub target: MemoryId,
    pub kind: GraphRelationKind,
    pub active: bool,
    pub origin: GraphRelationOrigin,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct GraphVersionPayload {
    pub global_version: u64,
    pub graph_version: u64,
    pub mutation: ObjectRef,
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

pub(crate) fn encode_node(node: GraphNodePayload) -> Vec<u8> {
    if let Some(memory_id) = node.semantic_node.as_memory() {
        let mut out = vec![0_u8; 44];
        out[..8].copy_from_slice(NODE_V1_MAGIC);
        out[8..40].copy_from_slice(&memory_id.0);
        out[40..44].copy_from_slice(&node.node_id.0.to_le_bytes());
        return out;
    }

    let mut out = vec![0_u8; 45];
    out[..8].copy_from_slice(NODE_V2_MAGIC);
    out[8] = node.semantic_node.kind.code();
    out[9..41].copy_from_slice(&node.semantic_node.id);
    out[41..45].copy_from_slice(&node.node_id.0.to_le_bytes());
    out
}

pub(crate) fn decode_node(bytes: &[u8]) -> Result<Option<GraphNodePayload>, GraphError> {
    if bytes.starts_with(NODE_V1_MAGIC) {
        if bytes.len() != 44 {
            return Err(GraphError::CorruptRecord("node mapping"));
        }
        return Ok(Some(GraphNodePayload {
            semantic_node: SemanticNodeRef::memory(MemoryId(
                bytes[8..40].try_into().expect("memory id width"),
            )),
            node_id: NodeId(read_u32(bytes, 40)?),
        }));
    }
    if !bytes.starts_with(NODE_V2_MAGIC) {
        return Ok(None);
    }
    if bytes.len() != 45 {
        return Err(GraphError::CorruptRecord("semantic node mapping"));
    }
    let kind = SemanticNodeKind::from_code(bytes[8])
        .ok_or(GraphError::CorruptRecord("semantic node kind"))?;
    Ok(Some(GraphNodePayload {
        semantic_node: SemanticNodeRef {
            kind,
            id: bytes[9..41].try_into().expect("semantic node id width"),
        },
        node_id: NodeId(read_u32(bytes, 41)?),
    }))
}

pub(crate) fn encode_mutation(mutation: GraphMutationPayload) -> [u8; 76] {
    let mut out = [0_u8; 76];
    out[..8].copy_from_slice(MUTATION_MAGIC);
    out[8..40].copy_from_slice(&mutation.source.0);
    out[40..72].copy_from_slice(&mutation.target.0);
    out[72..74].copy_from_slice(&mutation.kind.code().to_le_bytes());
    out[74] = u8::from(mutation.active);
    out[75] = mutation.origin.code();
    out
}

pub(crate) fn decode_mutation(bytes: &[u8]) -> Result<Option<GraphMutationPayload>, GraphError> {
    if !bytes.starts_with(MUTATION_MAGIC) {
        return Ok(None);
    }
    let has_origin = match bytes.len() {
        75 => false,
        76 => true,
        _ => return Err(GraphError::CorruptRecord("relationship mutation")),
    };
    Ok(Some(decode_mutation_body(bytes, 8, has_origin)?))
}

pub(crate) fn encode_batch(mutations: &[GraphMutationPayload]) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + mutations.len() * 68);
    out.extend_from_slice(BATCH_MAGIC);
    out.extend_from_slice(&(mutations.len() as u32).to_le_bytes());
    for mutation in mutations {
        out.extend_from_slice(&mutation.source.0);
        out.extend_from_slice(&mutation.target.0);
        out.extend_from_slice(&mutation.kind.code().to_le_bytes());
        out.push(u8::from(mutation.active));
        out.push(mutation.origin.code());
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
    if count == 0 {
        return Err(GraphError::CorruptRecord("relationship batch"));
    }
    let legacy_expected = 12_usize
        .checked_add(
            count
                .checked_mul(67)
                .ok_or(GraphError::CorruptRecord("relationship batch"))?,
        )
        .ok_or(GraphError::CorruptRecord("relationship batch"))?;
    let current_expected = 12_usize
        .checked_add(
            count
                .checked_mul(68)
                .ok_or(GraphError::CorruptRecord("relationship batch"))?,
        )
        .ok_or(GraphError::CorruptRecord("relationship batch"))?;
    let (stride, has_origin) = if bytes.len() == current_expected {
        (68, true)
    } else if bytes.len() == legacy_expected {
        (67, false)
    } else {
        return Err(GraphError::CorruptRecord("relationship batch"));
    };
    let mut mutations = Vec::with_capacity(count);
    for index in 0..count {
        mutations.push(decode_mutation_body(
            bytes,
            12 + index * stride,
            has_origin,
        )?);
    }
    Ok(Some(mutations))
}

fn decode_mutation_body(
    bytes: &[u8],
    offset: usize,
    has_origin: bool,
) -> Result<GraphMutationPayload, GraphError> {
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
    let origin = if has_origin {
        let code = bytes
            .get(offset + 67)
            .copied()
            .ok_or(GraphError::CorruptRecord("relationship mutation"))?;
        GraphRelationOrigin::from_code(code)
            .ok_or(GraphError::CorruptRecord("relationship mutation"))?
    } else {
        GraphRelationOrigin::Dream
    };
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
        origin,
    })
}

pub(crate) fn encode_version(version: GraphVersionPayload) -> [u8; 40] {
    let mut out = [0_u8; 40];
    out[..8].copy_from_slice(VERSION_MAGIC);
    out[8..16].copy_from_slice(&version.global_version.to_le_bytes());
    out[16..24].copy_from_slice(&version.graph_version.to_le_bytes());
    out[24..40].copy_from_slice(&version.mutation.legacy_bytes());
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
        mutation: ObjectRef::from_legacy_bytes(bytes[24..40].try_into().unwrap()),
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
