use crate::archive_vector_codec::{decode_object, encode_format, encode_object};
use crate::packed_vector_store::PackedVectorStore;
use crate::{
    Archive, ArchiveVectorError, ArchiveVectorId, ArchiveVectorInfo, ArchiveVectorSet,
    ArchiveVectorStats, ChunkRef, Container, FragmentId, PackedVectorId,
};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(crate) struct ArchiveVectorStore {
    objects: HashMap<ArchiveVectorId, ArchiveVectorEntry>,
}

#[derive(Clone, Copy)]
struct ArchiveVectorEntry {
    info: ArchiveVectorInfo,
    chunk: ChunkRef,
}

impl ArchiveVectorStore {
    pub(crate) fn initialize(&self, container: &mut Container) -> Result<(), ArchiveVectorError> {
        container.append(&encode_format())?;
        Ok(())
    }

    pub(crate) fn put(
        &mut self,
        container: &mut Container,
        archive: &Archive,
        packed_vectors: &PackedVectorStore,
        packed_vector_id: PackedVectorId,
        fragment_ids: Vec<FragmentId>,
    ) -> Result<ArchiveVectorId, ArchiveVectorError> {
        validate_mapping(archive, packed_vectors, packed_vector_id, &fragment_ids)?;
        let id = archive_vector_id(packed_vector_id, &fragment_ids);
        if let Some(existing) = self.objects.get(&id).copied() {
            let payload = container.read(existing.chunk)?;
            let decoded = decode_object(&payload)?.ok_or(ArchiveVectorError::MissingObject)?;
            if decoded.id != id
                || decoded.packed_vector_id != packed_vector_id
                || decoded.fragment_ids != fragment_ids
            {
                return Err(ArchiveVectorError::HashCollision);
            }
            return Ok(id);
        }
        let payload = encode_object(id, packed_vector_id, &fragment_ids)?;
        let chunk = container.append(&payload)?;
        let info = ArchiveVectorInfo {
            id,
            packed_vector_id,
            rows: u64::try_from(fragment_ids.len())
                .map_err(|_| ArchiveVectorError::SizeOverflow)?,
        };
        self.objects.insert(id, ArchiveVectorEntry { info, chunk });
        Ok(id)
    }

    pub(crate) fn get(
        &self,
        container: &mut Container,
        id: ArchiveVectorId,
    ) -> Result<ArchiveVectorSet, ArchiveVectorError> {
        let entry = self
            .objects
            .get(&id)
            .ok_or(ArchiveVectorError::MissingObject)?;
        let payload = container.read(entry.chunk)?;
        let decoded = decode_object(&payload)?.ok_or(ArchiveVectorError::MissingObject)?;
        if decoded.id != id
            || archive_vector_id(decoded.packed_vector_id, &decoded.fragment_ids) != id
        {
            return Err(ArchiveVectorError::HashCollision);
        }
        Ok(ArchiveVectorSet {
            id,
            packed_vector_id: decoded.packed_vector_id,
            fragment_ids: decoded.fragment_ids,
        })
    }

    pub(crate) fn insert_rebuilt(
        &mut self,
        info: ArchiveVectorInfo,
        chunk: ChunkRef,
    ) -> Result<(), ArchiveVectorError> {
        if let Some(existing) = self.objects.get(&info.id) {
            if existing.info != info {
                return Err(ArchiveVectorError::HashCollision);
            }
            return Ok(());
        }
        self.objects
            .insert(info.id, ArchiveVectorEntry { info, chunk });
        Ok(())
    }

    pub(crate) fn infos(&self) -> Vec<ArchiveVectorInfo> {
        let mut infos: Vec<_> = self.objects.values().map(|entry| entry.info).collect();
        infos.sort_by_key(|info| info.id.0);
        infos
    }

    pub(crate) fn stats(&self) -> ArchiveVectorStats {
        ArchiveVectorStats {
            objects: self.objects.len(),
            rows: self.objects.values().map(|entry| entry.info.rows).sum(),
        }
    }
}

pub(crate) fn validate_mapping(
    archive: &Archive,
    packed_vectors: &PackedVectorStore,
    packed_vector_id: PackedVectorId,
    fragment_ids: &[FragmentId],
) -> Result<(), ArchiveVectorError> {
    let packed = packed_vectors
        .info(packed_vector_id)
        .ok_or(ArchiveVectorError::MissingPackedVector)?;
    let rows = u64::try_from(fragment_ids.len()).map_err(|_| ArchiveVectorError::SizeOverflow)?;
    if packed.count != rows {
        return Err(ArchiveVectorError::RowCountMismatch);
    }
    let mut seen = HashSet::with_capacity(fragment_ids.len());
    for fragment_id in fragment_ids {
        if archive.fragments.get(*fragment_id).is_none() {
            return Err(ArchiveVectorError::MissingFragment);
        }
        if !seen.insert(*fragment_id) {
            return Err(ArchiveVectorError::DuplicateFragment);
        }
    }
    Ok(())
}

pub(crate) fn archive_vector_id(
    packed_vector_id: PackedVectorId,
    fragment_ids: &[FragmentId],
) -> ArchiveVectorId {
    let mut hash = Sha256::new();
    hash.update(b"CVA-ARCHIVE-VECTORS-V1\0");
    hash.update(packed_vector_id.0);
    for fragment_id in fragment_ids {
        hash.update(fragment_id.0);
    }
    ArchiveVectorId(hash.finalize().into())
}
