mod candidate;
mod candidate_policy;
mod candidate_receipt_policy;
mod candidate_shape;
mod candidate_source;
mod candidate_text;
pub(crate) mod codec;
pub(crate) mod completion;
mod contract;
mod cva;
mod error;
mod evidence;
mod extraction;
mod ledger;
mod model;
mod processor;
pub(crate) mod rebuild;
pub(crate) mod runtime_step;
pub(crate) mod store;
mod synthesis;
mod worker;

pub use contract::{
    INSOMNIA_EXTRACTOR_CONTRACT_VERSION, INSOMNIA_SYSTEM_PROMPT, MAX_INSOMNIA_CANDIDATES,
    insomnia_schema,
};
pub use error::InsomniaError;
pub use evidence::{
    MAX_INSOMNIA_EVIDENCE_BYTES, MAX_INSOMNIA_EVIDENCE_REQUESTS, MAX_INSOMNIA_EVIDENCE_TURNS,
};
pub use extraction::{
    InsomniaCandidate, InsomniaEvidenceResult, InsomniaEvidenceTurn, InsomniaExtraction,
    InsomniaExtractionError, InsomniaExtractor, InsomniaRejection,
};
pub(crate) use extraction::{InsomniaEvidenceRound, InsomniaExtractionStage};
pub use model::{
    EpisodeSchedulingResult, InsomniaAttempt, InsomniaLeaseToken, InsomniaPriority, InsomniaStats,
    InsomniaWork, InsomniaWorkState,
};
pub use processor::{InsomniaProcessError, InsomniaProcessResult};
pub(crate) use runtime_step::RuntimeInsomniaClaim;
pub use worker::{
    DEFAULT_INSOMNIA_LEASE_NS, DEFAULT_INSOMNIA_MAX_ATTEMPTS, DEFAULT_INSOMNIA_POLL_NS,
    DEFAULT_INSOMNIA_RETRY_DELAY_NS, DEFAULT_INSOMNIA_WORKERS, InsomniaDrainResult,
    InsomniaWorkerConfig, InsomniaWorkerError, MAX_INSOMNIA_WORKERS,
};

#[cfg(test)]
mod candidate_policy_tests;
#[cfg(test)]
mod candidate_receipt_policy_tests;
#[cfg(test)]
mod completion_tests;
#[cfg(test)]
mod evidence_flow_tests;
#[cfg(test)]
mod evidence_tests;
#[cfg(test)]
mod extraction_tests;
#[cfg(test)]
mod gold_tests;
#[cfg(test)]
mod grounding_tests;
#[cfg(test)]
mod queue_recovery_tests;
#[cfg(test)]
mod queue_tests;
#[cfg(test)]
mod scheduler_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod worker_tests;
#[cfg(test)]
mod worker_vector_tests;
