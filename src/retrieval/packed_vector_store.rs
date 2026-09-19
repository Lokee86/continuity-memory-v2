use crate::packed_vector_codec::{decode_object, encode_format, encode_object};
use crate::{
    Container, ObjectRef, PackedVectorError, PackedVectorId, PackedVectorInfo, PackedVectorStats,
};
use lodestone_packed::{PackedVectors, VectorSchema};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct PackedVectorStore {
    objects: HashMap<PackedVectorId, PackedVectorEntry>,
}

#[derive(Clone, Copy)]
struct PackedVectorEntry {
    info: PackedVectorInfo,
    chunk: ObjectRef,
}

impl PackedVectorStore {
    pub(crate) fn initialize(&self, container: &mut Container) -> Result<(), PackedVectorError> {
        container.append(&encode_format())?;
        Ok(())
    }

    pub(crate) fn put(
        &mut self,
        container: &mut Container,
        packed: PackedVectors,
    ) -> Result<PackedVectorId, PackedVectorError> {
        let id = packed_vector_id(packed.schema(), packed.bytes());
        if let Some(existing) = self.objects.get(&id).copied() {
            let payload = container.read(existing.chunk)?;
            let decoded = decode_object(&payload)?.ok_or(PackedVectorError::MissingObject)?;
            if decoded.id != id
                || decoded.schema != packed.schema()
                || decoded.bytes != packed.bytes()
            {
                return Err(PackedVectorError::HashCollision);
            }
            return Ok(id);
        }
        let payload = encode_object(id, &packed)?;
        let chunk = container.append(&payload)?;
        let info = info(id, packed.schema(), packed.count(), packed.bytes().len())?;
        self.objects.insert(id, PackedVectorEntry { info, chunk });
        Ok(id)
    }

    pub(crate) fn get(
        &self,
        container: &mut Container,
        id: PackedVectorId,
    ) -> Result<PackedVectors, PackedVectorError> {
        let entry = self
            .objects
            .get(&id)
            .ok_or(PackedVectorError::MissingObject)?;
        let payload = container.read(entry.chunk)?;
        let decoded = decode_object(&payload)?.ok_or(PackedVectorError::MissingObject)?;
        if decoded.id != id || packed_vector_id(decoded.schema, decoded.bytes) != id {
            return Err(PackedVectorError::HashCollision);
        }
        PackedVectors::from_bytes(decoded.schema, decoded.bytes.to_vec())
            .map_err(|_| PackedVectorError::CorruptRecord("packed-vector matrix"))
    }

    pub(crate) fn insert_rebuilt(
        &mut self,
        info: PackedVectorInfo,
        chunk: ObjectRef,
    ) -> Result<(), PackedVectorError> {
        if let Some(existing) = self.objects.get(&info.id) {
            if existing.info != info {
                return Err(PackedVectorError::HashCollision);
            }
            return Ok(());
        }
        self.objects
            .insert(info.id, PackedVectorEntry { info, chunk });
        Ok(())
    }

    pub(crate) fn info(&self, id: PackedVectorId) -> Option<PackedVectorInfo> {
        self.objects.get(&id).map(|entry| entry.info)
    }

    pub(crate) fn infos(&self) -> Vec<PackedVectorInfo> {
        let mut infos: Vec<_> = self.objects.values().map(|entry| entry.info).collect();
        infos.sort_by_key(|info| info.id.0);
        infos
    }

    pub(crate) fn stats(&self) -> PackedVectorStats {
        PackedVectorStats {
            objects: self.objects.len(),
            rows: self.objects.values().map(|entry| entry.info.count).sum(),
            matrix_bytes: self.objects.values().map(|entry| entry.info.byte_len).sum(),
        }
    }
}

pub(crate) fn packed_vector_id(schema: VectorSchema, bytes: &[u8]) -> PackedVectorId {
    let mut hash = Sha256::new();
    hash.update(b"CVA-PACKED-VECTOR-V1\0");
    hash.update(schema.dimensions.to_le_bytes());
    hash.update([schema.scalar.tag()]);
    hash.update(bytes);
    PackedVectorId(hash.finalize().into())
}

pub(crate) fn info(
    id: PackedVectorId,
    schema: VectorSchema,
    count: usize,
    byte_len: usize,
) -> Result<PackedVectorInfo, PackedVectorError> {
    Ok(PackedVectorInfo {
        id,
        schema,
        count: u64::try_from(count).map_err(|_| PackedVectorError::SizeOverflow)?,
        byte_len: u64::try_from(byte_len).map_err(|_| PackedVectorError::SizeOverflow)?,
    })
}
