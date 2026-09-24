use super::{ReliquaryRuntimeHost, ReliquaryRuntimeHostError, operation};
use crate::compatibility_profile_probe::profile_from_endpoint;
use crate::{
    EmbeddingEndpoint, EmbeddingEndpointError, EmbeddingMode, Memory, MemoryId,
    MemoryRetrievalConfig, MemoryRetrievalResult, VectorNormalization,
};
use std::collections::HashMap;
use std::sync::Arc;

pub const MAX_MEMORY_SEARCH_QUERY_BYTES: usize = 512;

#[derive(Clone, Debug, PartialEq)]
pub struct MemorySearchItem {
    pub memory: Memory,
    pub seed_score: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemorySearchLane {
    pub owner_id: String,
    pub retrieval: MemoryRetrievalResult,
    pub items: Vec<MemorySearchItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemorySearchResult {
    pub reliquary: MemorySearchLane,
    pub phylactery: Option<MemorySearchLane>,
}

impl ReliquaryRuntimeHost {
    pub fn retrieve_reliquary_memories(
        &self,
        query_vector: &[f32],
        config: MemoryRetrievalConfig,
    ) -> Result<MemorySearchLane, ReliquaryRuntimeHostError> {
        let profile = self.ensure_reliquary_embedding_profile()?;
        let runtime = self
            .active_execution()?
            .runtime
            .as_ref()
            .cloned()
            .ok_or_else(|| {
                ReliquaryRuntimeHostError::Operation("Reliquary runtime is unavailable".into())
            })?;
        let mut runtime = runtime
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        let owner_id = runtime.cva.owner_id().ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation(
                "Reliquary does not have durable owner identity".into(),
            )
        })?;
        let retrieval = runtime
            .cva
            .retrieve_memories(profile, query_vector, config)
            .map_err(operation)?;
        hydrate_reliquary(&mut runtime.cva, owner_id, retrieval)
    }

    pub fn retrieve_phylactery_memories(
        &self,
        query_vector: &[f32],
        config: MemoryRetrievalConfig,
    ) -> Result<Option<MemorySearchLane>, ReliquaryRuntimeHostError> {
        if !self.has_phylactery()? {
            return Ok(None);
        }
        let profile = self.ensure_phylactery_memory_profile()?;
        let phylactery = Arc::clone(&self.active_execution()?.phylactery);
        let mut phylactery = phylactery
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        let phy = phylactery.as_mut().ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation("Phylactery is unavailable".into())
        })?;
        let owner_id = phy.owner_id().ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation(
                "Phylactery does not have durable owner identity".into(),
            )
        })?;
        let retrieval = phy
            .retrieve_memories(profile, query_vector, config)
            .map_err(operation)?;
        hydrate_phylactery(phy, owner_id, retrieval).map(Some)
    }

    pub(crate) fn ensure_reliquary_embedding_profile(
        &self,
    ) -> Result<crate::CompatibilityProfileId, ReliquaryRuntimeHostError> {
        if let Some(profile) = self
            .active_execution()?
            .memory_profiles
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?
            .project
        {
            return Ok(profile);
        }
        let candidate = self.runtime_profile_candidate()?;
        let runtime = self
            .active_execution()?
            .runtime
            .as_ref()
            .cloned()
            .ok_or_else(|| {
                ReliquaryRuntimeHostError::Operation("Reliquary runtime is unavailable".into())
            })?;
        let profile = runtime
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?
            .cva
            .accept_runtime_compatibility_profile(candidate)
            .map_err(operation)?;
        self.active_execution()?
            .memory_profiles
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?
            .project = Some(profile.id);
        Ok(profile.id)
    }

    fn ensure_phylactery_memory_profile(
        &self,
    ) -> Result<crate::CompatibilityProfileId, ReliquaryRuntimeHostError> {
        if let Some(profile) = self
            .active_execution()?
            .memory_profiles
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?
            .user
        {
            return Ok(profile);
        }
        let candidate = self.runtime_profile_candidate()?;
        let mut phylactery = self
            .active_execution()?
            .phylactery
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        let profile = phylactery
            .as_mut()
            .ok_or_else(|| {
                ReliquaryRuntimeHostError::Operation("Phylactery is unavailable".into())
            })?
            .accept_runtime_compatibility_profile(candidate)
            .map_err(operation)?;
        self.active_execution()?
            .memory_profiles
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?
            .user = Some(profile.id);
        Ok(profile.id)
    }

    fn runtime_profile_candidate(
        &self,
    ) -> Result<crate::CompatibilityProfile, ReliquaryRuntimeHostError> {
        let endpoint = self
            .active_execution()?
            .routes
            .read()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?
            .embedding()
            .ok_or_else(|| {
                ReliquaryRuntimeHostError::Operation(
                    "memory retrieval requires an embedding route".into(),
                )
            })?;
        profile_from_endpoint(&SharedEmbeddingEndpoint(endpoint)).map_err(operation)
    }

    pub fn retrieve_memory_pair(
        &self,
        query_vector: &[f32],
        config: MemoryRetrievalConfig,
    ) -> Result<MemorySearchResult, ReliquaryRuntimeHostError> {
        std::thread::scope(|scope| {
            let rel = scope.spawn(|| self.retrieve_reliquary_memories(query_vector, config));
            let phy = scope.spawn(|| self.retrieve_phylactery_memories(query_vector, config));
            Ok(MemorySearchResult {
                reliquary: rel
                    .join()
                    .map_err(|_| ReliquaryRuntimeHostError::ThreadPanicked)??,
                phylactery: phy
                    .join()
                    .map_err(|_| ReliquaryRuntimeHostError::ThreadPanicked)??,
            })
        })
    }
}

#[derive(Clone)]
struct SharedEmbeddingEndpoint(Arc<dyn EmbeddingEndpoint + Send + Sync>);

impl EmbeddingEndpoint for SharedEmbeddingEndpoint {
    fn dimensions(&self) -> u32 {
        self.0.dimensions()
    }

    fn normalization(&self) -> VectorNormalization {
        self.0.normalization()
    }

    fn embed(
        &self,
        mode: EmbeddingMode,
        inputs: &[String],
    ) -> Result<Vec<Vec<f32>>, EmbeddingEndpointError> {
        self.0.embed(mode, inputs)
    }
}

fn hydrate_reliquary(
    cva: &mut crate::Cva,
    owner_id: String,
    retrieval: MemoryRetrievalResult,
) -> Result<MemorySearchLane, ReliquaryRuntimeHostError> {
    let scores = seed_scores(&retrieval);
    let items = retrieval
        .memories
        .iter()
        .map(|id| {
            cva.memory(*id)
                .map(|memory| MemorySearchItem {
                    memory,
                    seed_score: scores.get(id).copied(),
                })
                .map_err(operation)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(MemorySearchLane {
        owner_id,
        retrieval,
        items,
    })
}

fn hydrate_phylactery(
    phylactery: &mut crate::Phylactery,
    owner_id: String,
    retrieval: MemoryRetrievalResult,
) -> Result<MemorySearchLane, ReliquaryRuntimeHostError> {
    let scores = seed_scores(&retrieval);
    let items = retrieval
        .memories
        .iter()
        .map(|id| {
            phylactery
                .memory(*id)
                .map(|memory| MemorySearchItem {
                    memory,
                    seed_score: scores.get(id).copied(),
                })
                .map_err(operation)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(MemorySearchLane {
        owner_id,
        retrieval,
        items,
    })
}

fn seed_scores(retrieval: &MemoryRetrievalResult) -> HashMap<MemoryId, f64> {
    retrieval
        .seeds
        .iter()
        .map(|hit| (hit.memory_id, hit.score))
        .collect()
}
