use crate::compatibility_profile_probe::validate_embedding_batch;
use crate::{
    CompatibilityProfile, CompatibilityProfileError, CompatibilityProfileId, Cva, MemoryBodyId,
    MemoryVectorError, PackedVectors, ScalarType, VectorSchema,
};

pub(crate) struct RuntimeMemoryVectorBatch {
    pub(crate) profile: CompatibilityProfile,
    pub(crate) body_ids: Vec<MemoryBodyId>,
    pub(crate) texts: Vec<String>,
}

impl Cva {
    pub(crate) fn accept_runtime_compatibility_profile(
        &mut self,
        candidate: CompatibilityProfile,
    ) -> Result<CompatibilityProfile, CompatibilityProfileError> {
        if let Some(existing) = self.compatibility_profiles.compatible_with(&candidate) {
            return Ok(existing.clone());
        }
        self.compatibility_profiles
            .put(&mut self.container, candidate.clone())?;
        Ok(candidate)
    }

    pub(crate) fn prepare_runtime_memory_vector_batch(
        &mut self,
        profile_id: CompatibilityProfileId,
    ) -> Result<Option<RuntimeMemoryVectorBatch>, MemoryVectorError> {
        let profile = self
            .compatibility_profiles
            .get(profile_id)
            .cloned()
            .ok_or(MemoryVectorError::MissingProfile)?;
        let current = self.memories.current_body_ids();
        if current.is_empty() {
            return Ok(None);
        }
        let body_ids = self.memory_vectors.missing_body_ids(profile_id, &current);
        if body_ids.is_empty() {
            return Ok(None);
        }
        let mut texts = Vec::with_capacity(body_ids.len());
        for body_id in &body_ids {
            texts.push(
                self.memories
                    .body_embedding_text(&mut self.container, *body_id)?,
            );
        }
        Ok(Some(RuntimeMemoryVectorBatch {
            profile,
            body_ids,
            texts,
        }))
    }

    pub(crate) fn commit_runtime_memory_vector_batch(
        &mut self,
        batch: RuntimeMemoryVectorBatch,
        vectors: Vec<Vec<f32>>,
    ) -> Result<usize, MemoryVectorError> {
        validate_embedding_batch(&batch.profile, &vectors, batch.body_ids.len())?;

        let mut body_ids = Vec::with_capacity(batch.body_ids.len());
        let mut pending_vectors = Vec::with_capacity(vectors.len());
        for (body_id, vector) in batch.body_ids.into_iter().zip(vectors) {
            if self
                .memory_vectors
                .location(batch.profile.id, body_id)
                .is_none()
            {
                body_ids.push(body_id);
                pending_vectors.push(vector);
            }
        }
        if body_ids.is_empty() {
            return Ok(0);
        }

        let schema = VectorSchema::new(batch.profile.dimensions, ScalarType::F32)
            .map_err(|_| MemoryVectorError::InvalidEmbeddingMatrix)?;
        let row_bytes = usize::try_from(batch.profile.dimensions)
            .ok()
            .and_then(|dimensions| dimensions.checked_mul(4))
            .ok_or(MemoryVectorError::InvalidEmbeddingMatrix)?;
        let capacity = pending_vectors
            .len()
            .checked_mul(row_bytes)
            .ok_or(MemoryVectorError::InvalidEmbeddingMatrix)?;
        let mut bytes = Vec::with_capacity(capacity);
        for vector in pending_vectors {
            for value in vector {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        let packed = PackedVectors::from_bytes(schema, bytes)
            .map_err(|_| MemoryVectorError::InvalidEmbeddingMatrix)?;
        let packed_vector_id = self.put_packed_vectors(packed)?;
        self.put_memory_vectors(batch.profile.id, packed_vector_id, body_ids.clone())?;
        Ok(body_ids.len())
    }
}
