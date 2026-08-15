mod candidate;
pub(crate) mod codec;
mod contract;
mod cva;
mod error;
mod extraction;
mod model;
mod processor;
pub(crate) mod rebuild;
pub(crate) mod store;

pub use contract::{
    INSOMNIA_EXTRACTOR_CONTRACT_VERSION, INSOMNIA_SYSTEM_PROMPT, MAX_INSOMNIA_CANDIDATES,
    insomnia_schema,
};
pub use error::InsomniaError;
pub use extraction::{
    InsomniaCandidate, InsomniaExtraction, InsomniaExtractionError, InsomniaExtractor,
    InsomniaRejection,
};
pub use model::{
    EpisodeSchedulingResult, InsomniaAttempt, InsomniaLeaseToken, InsomniaPriority, InsomniaStats,
    InsomniaWork, InsomniaWorkState,
};
pub use processor::{InsomniaProcessError, InsomniaProcessResult};

#[cfg(test)]
mod extraction_tests;
#[cfg(test)]
mod queue_recovery_tests;
#[cfg(test)]
mod queue_tests;
#[cfg(test)]
mod test_support;
