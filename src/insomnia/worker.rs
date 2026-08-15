use crate::{
    CompatibilityProfileError, CompatibilityProfileId, Cva, EmbeddingEndpoint, GeneralEndpoint,
    InsomniaError, InsomniaExtractor, InsomniaProcessError, MemoryVectorError, MemoryVectorId,
};
use std::fmt;

mod runtime;

pub const DEFAULT_INSOMNIA_WORKERS: usize = 16;
pub const MAX_INSOMNIA_WORKERS: usize = 64;
pub const DEFAULT_INSOMNIA_LEASE_NS: i64 = 15 * 60 * 1_000_000_000;
pub const DEFAULT_INSOMNIA_RETRY_DELAY_NS: i64 = 5 * 1_000_000_000;
pub const DEFAULT_INSOMNIA_POLL_NS: i64 = 50 * 1_000_000;
pub const DEFAULT_INSOMNIA_MAX_ATTEMPTS: u32 = 5;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InsomniaWorkerConfig {
    pub workers: usize,
    pub scope: String,
    pub worker_id_prefix: String,
    pub lease_duration_ns: i64,
    pub retry_delay_ns: i64,
    pub poll_interval_ns: i64,
    pub max_attempts: u32,
}

impl Default for InsomniaWorkerConfig {
    fn default() -> Self {
        Self {
            workers: DEFAULT_INSOMNIA_WORKERS,
            scope: "private".into(),
            worker_id_prefix: "insomnia".into(),
            lease_duration_ns: DEFAULT_INSOMNIA_LEASE_NS,
            retry_delay_ns: DEFAULT_INSOMNIA_RETRY_DELAY_NS,
            poll_interval_ns: DEFAULT_INSOMNIA_POLL_NS,
            max_attempts: DEFAULT_INSOMNIA_MAX_ATTEMPTS,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InsomniaDrainResult {
    pub workers: usize,
    pub peak_active_workers: usize,
    pub claimed_attempts: usize,
    pub completed_episodes: usize,
    pub failed_attempts: usize,
    pub terminal_episodes: usize,
    pub memories_created: usize,
    pub memories_existing: usize,
    pub rejected_candidates: usize,
    pub evidence_turns: usize,
    pub memory_vector_profile_id: Option<CompatibilityProfileId>,
    pub memory_vectors_embedded: usize,
    pub memory_vectors_already_present: usize,
    pub memory_vector_set_id: Option<MemoryVectorId>,
}

#[derive(Debug)]
pub enum InsomniaWorkerError {
    InvalidConfig(&'static str),
    Queue(InsomniaError),
    Process(InsomniaProcessError),
    CompatibilityProfile(CompatibilityProfileError),
    MemoryVectors(MemoryVectorError),
    LockPoisoned,
    ThreadPanicked,
}

impl fmt::Display for InsomniaWorkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(field) => {
                write!(f, "invalid Insomnia worker configuration: {field}")
            }
            Self::Queue(error) => write!(f, "{error}"),
            Self::Process(error) => write!(f, "{error}"),
            Self::CompatibilityProfile(error) => write!(f, "{error}"),
            Self::MemoryVectors(error) => write!(f, "{error}"),
            Self::LockPoisoned => write!(f, "Insomnia worker CVA lock is poisoned"),
            Self::ThreadPanicked => write!(f, "Insomnia worker thread panicked"),
        }
    }
}

impl std::error::Error for InsomniaWorkerError {}

impl From<InsomniaError> for InsomniaWorkerError {
    fn from(value: InsomniaError) -> Self {
        Self::Queue(value)
    }
}

impl From<InsomniaProcessError> for InsomniaWorkerError {
    fn from(value: InsomniaProcessError) -> Self {
        Self::Process(value)
    }
}

impl From<CompatibilityProfileError> for InsomniaWorkerError {
    fn from(value: CompatibilityProfileError) -> Self {
        Self::CompatibilityProfile(value)
    }
}

impl From<MemoryVectorError> for InsomniaWorkerError {
    fn from(value: MemoryVectorError) -> Self {
        Self::MemoryVectors(value)
    }
}

impl Cva {
    pub fn drain_insomnia_backlog<E: GeneralEndpoint, V: EmbeddingEndpoint>(
        &mut self,
        extractor: &InsomniaExtractor<E>,
        embedding_endpoint: &V,
        config: InsomniaWorkerConfig,
    ) -> Result<InsomniaDrainResult, InsomniaWorkerError> {
        validate_config(&config)?;
        let mut result = runtime::drain(self, extractor, &config)?;
        if self.memory_stats().memories == 0 {
            return Ok(result);
        }
        let profile = self.establish_compatibility_profile(embedding_endpoint)?;
        let vectors = self.build_missing_memory_vectors(profile.id, embedding_endpoint)?;
        result.memory_vector_profile_id = Some(profile.id);
        result.memory_vectors_embedded = vectors.embedded;
        result.memory_vectors_already_present = vectors.already_present;
        result.memory_vector_set_id = vectors.created_set;
        Ok(result)
    }
}

fn validate_config(config: &InsomniaWorkerConfig) -> Result<(), InsomniaWorkerError> {
    if config.workers == 0 || config.workers > MAX_INSOMNIA_WORKERS {
        return Err(InsomniaWorkerError::InvalidConfig("workers"));
    }
    if config.scope.trim().is_empty() || config.worker_id_prefix.trim().is_empty() {
        return Err(InsomniaWorkerError::InvalidConfig("scope/worker ID"));
    }
    if config.lease_duration_ns <= 0
        || config.retry_delay_ns <= 0
        || config.poll_interval_ns <= 0
        || config.max_attempts == 0
    {
        return Err(InsomniaWorkerError::InvalidConfig("timing/attempt limits"));
    }
    Ok(())
}
