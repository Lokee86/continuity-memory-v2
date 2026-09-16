use crate::insomnia::store::InsomniaStore;
use crate::memory_store::MemoryStore;
use crate::{
    Archive, Container, Cva, GeneralEndpoint, InsomniaExtraction, InsomniaExtractionError,
    InsomniaExtractor, InsomniaRejection, InsomniaWork, InsomniaWorkState, Memory, MemoryError,
};
use std::fmt;

mod application;
mod prepared;
mod source_validation;

pub(crate) use application::{commit_application, prepare_application, publish_user_application};
pub(crate) use prepared::PreparedApplication;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InsomniaProcessResult {
    pub created: Vec<Memory>,
    pub existing: Vec<Memory>,
    pub user_created: Vec<Memory>,
    pub user_existing: Vec<Memory>,
    pub rejected: Vec<InsomniaRejection>,
}

#[derive(Debug)]
pub enum InsomniaProcessError {
    MissingEpisode,
    InvalidClaim,
    InvalidCandidate(String),
    UserRoutingRequired,
    Phylactery(String),
    Extraction(InsomniaExtractionError),
    Memory(MemoryError),
    Queue(crate::InsomniaError),
    Archive(crate::ArchiveError),
}

impl fmt::Display for InsomniaProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEpisode => write!(f, "Insomnia episode is missing"),
            Self::InvalidClaim => write!(f, "Insomnia work is not an active claim"),
            Self::InvalidCandidate(message) => write!(f, "invalid Insomnia candidate: {message}"),
            Self::UserRoutingRequired => write!(
                f,
                "Insomnia produced user-owned Memory but no Phylactery routing target was supplied"
            ),
            Self::Phylactery(message) => write!(f, "Phylactery routing failed: {message}"),
            Self::Extraction(error) => write!(f, "{error}"),
            Self::Memory(error) => write!(f, "{error}"),
            Self::Queue(error) => write!(f, "{error}"),
            Self::Archive(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for InsomniaProcessError {}

impl From<InsomniaExtractionError> for InsomniaProcessError {
    fn from(value: InsomniaExtractionError) -> Self {
        Self::Extraction(value)
    }
}

impl From<MemoryError> for InsomniaProcessError {
    fn from(value: MemoryError) -> Self {
        Self::Memory(value)
    }
}

impl From<crate::InsomniaError> for InsomniaProcessError {
    fn from(value: crate::InsomniaError) -> Self {
        Self::Queue(value)
    }
}

impl From<crate::ArchiveError> for InsomniaProcessError {
    fn from(value: crate::ArchiveError) -> Self {
        Self::Archive(value)
    }
}

impl Cva {
    pub fn process_claimed_insomnia_episode<E: GeneralEndpoint>(
        &mut self,
        claim: &InsomniaWork,
        extractor: &InsomniaExtractor<E>,
        scope: &str,
        started_at_ns: i64,
        completed_at_ns: i64,
    ) -> Result<InsomniaProcessResult, InsomniaProcessError> {
        let (episode, turns) = self.claimed_episode_input(claim, scope)?;
        let extraction = extractor.extract_with_evidence(self, &episode, &turns)?;
        self.apply_claimed_insomnia_extraction(
            claim,
            &episode,
            &turns,
            extraction,
            scope,
            started_at_ns,
            completed_at_ns,
        )
    }

    pub fn process_claimed_insomnia_episode_routed<E: GeneralEndpoint>(
        &mut self,
        phylactery: &mut crate::Phylactery,
        claim: &InsomniaWork,
        extractor: &InsomniaExtractor<E>,
        scope: &str,
        started_at_ns: i64,
        completed_at_ns: i64,
    ) -> Result<InsomniaProcessResult, InsomniaProcessError> {
        let (episode, turns) = self.claimed_episode_input(claim, scope)?;
        let extraction = extractor.extract_with_evidence(self, &episode, &turns)?;
        let prepared = prepare_application(
            &self.archive,
            &mut self.container,
            &episode,
            &turns,
            extraction,
            scope,
            completed_at_ns,
        )?;
        let user_publication = if prepared.user_drafts.is_empty() {
            None
        } else {
            Some(publish_user_application(phylactery, &prepared.user_drafts)?)
        };
        commit_application(
            &mut self.container,
            &mut self.memories,
            &mut self.insomnia,
            claim,
            prepared,
            user_publication,
            started_at_ns,
            completed_at_ns,
        )
    }

    pub(crate) fn claimed_episode_input(
        &mut self,
        claim: &InsomniaWork,
        scope: &str,
    ) -> Result<(crate::Episode, Vec<crate::ResolvedTurn>), InsomniaProcessError> {
        claimed_episode_input_parts(&self.archive, &mut self.container, claim, scope)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_claimed_insomnia_extraction(
        &mut self,
        claim: &InsomniaWork,
        episode: &crate::Episode,
        turns: &[crate::ResolvedTurn],
        extraction: InsomniaExtraction,
        scope: &str,
        started_at_ns: i64,
        completed_at_ns: i64,
    ) -> Result<InsomniaProcessResult, InsomniaProcessError> {
        apply_claimed_insomnia_extraction_parts(
            &self.archive,
            &mut self.container,
            &mut self.memories,
            &mut self.insomnia,
            claim,
            episode,
            turns,
            extraction,
            scope,
            started_at_ns,
            completed_at_ns,
        )
    }
}

pub(crate) fn claimed_episode_input_parts(
    archive: &Archive,
    container: &mut Container,
    claim: &InsomniaWork,
    scope: &str,
) -> Result<(crate::Episode, Vec<crate::ResolvedTurn>), InsomniaProcessError> {
    validate_claim(claim, scope)?;
    let episode = archive
        .episode(claim.episode_id)
        .cloned()
        .ok_or(InsomniaProcessError::MissingEpisode)?;
    let turns = archive.episode_turns(container, claim.episode_id)?;
    Ok((episode, turns))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_claimed_insomnia_extraction_parts(
    archive: &Archive,
    container: &mut Container,
    memories: &mut MemoryStore,
    insomnia: &mut InsomniaStore,
    claim: &InsomniaWork,
    episode: &crate::Episode,
    turns: &[crate::ResolvedTurn],
    extraction: InsomniaExtraction,
    scope: &str,
    started_at_ns: i64,
    completed_at_ns: i64,
) -> Result<InsomniaProcessResult, InsomniaProcessError> {
    validate_claim(claim, scope)?;
    if claim.episode_id != episode.id {
        return Err(InsomniaProcessError::InvalidClaim);
    }
    let prepared = prepare_application(
        archive,
        container,
        episode,
        turns,
        extraction,
        scope,
        completed_at_ns,
    )?;
    apply_prepared(
        archive,
        container,
        memories,
        insomnia,
        claim,
        prepared,
        started_at_ns,
        completed_at_ns,
    )
}

#[allow(clippy::too_many_arguments)]
fn apply_prepared(
    archive: &Archive,
    container: &mut Container,
    memories: &mut MemoryStore,
    insomnia: &mut InsomniaStore,
    claim: &InsomniaWork,
    prepared: PreparedApplication,
    started_at_ns: i64,
    completed_at_ns: i64,
) -> Result<InsomniaProcessResult, InsomniaProcessError> {
    let _ = archive;
    commit_application(
        container,
        memories,
        insomnia,
        claim,
        prepared,
        None,
        started_at_ns,
        completed_at_ns,
    )
}

fn validate_claim(claim: &InsomniaWork, scope: &str) -> Result<(), InsomniaProcessError> {
    if claim.state != InsomniaWorkState::Processing
        || claim.lease_token.is_none()
        || scope.trim().is_empty()
    {
        Err(InsomniaProcessError::InvalidClaim)
    } else {
        Ok(())
    }
}
