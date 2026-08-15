use super::candidate::hex;
use crate::{
    Cva, GeneralEndpoint, INSOMNIA_EXTRACTOR_CONTRACT_VERSION, InsomniaCandidate,
    InsomniaExtraction, InsomniaExtractionError, InsomniaExtractor, InsomniaRejection,
    InsomniaWork, InsomniaWorkState, Memory, MemoryDraft, MemoryError,
};
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InsomniaProcessResult {
    pub created: Vec<Memory>,
    pub existing: Vec<Memory>,
    pub rejected: Vec<InsomniaRejection>,
}

#[derive(Debug)]
pub enum InsomniaProcessError {
    MissingEpisode,
    InvalidClaim,
    InvalidCandidate(String),
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

    pub(crate) fn claimed_episode_input(
        &mut self,
        claim: &InsomniaWork,
        scope: &str,
    ) -> Result<(crate::Episode, Vec<crate::ResolvedTurn>), InsomniaProcessError> {
        if claim.state != InsomniaWorkState::Processing
            || claim.lease_token.is_none()
            || scope.trim().is_empty()
        {
            return Err(InsomniaProcessError::InvalidClaim);
        }
        let episode = self
            .episode(claim.episode_id)
            .cloned()
            .ok_or(InsomniaProcessError::MissingEpisode)?;
        let turns = self.episode_turns(claim.episode_id)?;
        Ok((episode, turns))
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
        if claim.state != InsomniaWorkState::Processing
            || claim.lease_token.is_none()
            || claim.episode_id != episode.id
            || scope.trim().is_empty()
        {
            return Err(InsomniaProcessError::InvalidClaim);
        }
        let mut result = InsomniaProcessResult {
            created: Vec::new(),
            existing: Vec::new(),
            rejected: extraction.rejected,
        };
        for candidate in extraction.candidates {
            if let Err(reason) = self.validate_content_source(
                &candidate,
                &episode.conversation_id,
                turns,
                &extraction.evidence_turns,
            ) {
                result.rejected.push(InsomniaRejection {
                    candidate_key: Some(candidate.key.clone()),
                    reason,
                });
                continue;
            }
            let mutation_id = format!("insomnia:{}:{}", hex(&episode.id.0), candidate.key);
            let draft = MemoryDraft {
                category: candidate.category,
                memory_type: candidate.memory_type,
                title: candidate.title,
                content: candidate.content,
                scope: scope.trim().to_owned(),
                lifecycle_state: "extracted".into(),
                archived: false,
                superseded_by: None,
                parent_id: None,
                source_node_id: Some(candidate.source_node_id),
                content_source_conversation_id: candidate.content_source_conversation_id,
                content_source_node_id: candidate.content_source_node_id,
                source_episode_id: Some(episode.id),
                mutation_id,
                created_at_ns: episode.source_through_ns,
                updated_at_ns: completed_at_ns.max(episode.source_through_ns),
            };
            let (memory, created) = self.publish_memory(None, 0, draft)?;
            if created {
                result.created.push(memory);
            } else {
                result.existing.push(memory);
            }
        }
        let memory_ids = result
            .created
            .iter()
            .chain(&result.existing)
            .map(|memory| memory.id)
            .collect();
        self.complete_insomnia_episode(
            claim.episode_id,
            claim.lease_token.unwrap(),
            started_at_ns,
            completed_at_ns,
            extraction.model,
            INSOMNIA_EXTRACTOR_CONTRACT_VERSION.into(),
            memory_ids,
            result.rejected.len() as u32,
        )?;
        Ok(result)
    }

    fn validate_content_source(
        &mut self,
        candidate: &InsomniaCandidate,
        episode_conversation_id: &str,
        episode_turns: &[crate::ResolvedTurn],
        evidence_turns: &[crate::InsomniaEvidenceTurn],
    ) -> Result<(), String> {
        let Some(conversation_id) = candidate.content_source_conversation_id.as_deref() else {
            return Ok(());
        };
        let node_id = candidate
            .content_source_node_id
            .as_deref()
            .ok_or_else(|| "content source node is missing".to_owned())?;
        let quote = candidate
            .content_source_quote
            .as_deref()
            .ok_or_else(|| "content source quote is missing".to_owned())?;
        let source_is_in_episode = conversation_id == episode_conversation_id
            && episode_turns.iter().any(|turn| turn.node_id == node_id);
        let source_is_in_evidence = evidence_turns
            .iter()
            .any(|turn| turn.conversation_id == conversation_id && turn.node_id == node_id);
        if !source_is_in_episode && !source_is_in_evidence {
            return Err(
                "external content source was not supplied by bounded archive evidence".into(),
            );
        }
        let source = self
            .archive
            .require_node(
                conversation_id,
                node_id,
                crate::ArchiveError::MissingEpisodeNode,
            )
            .map_err(|_| "content source node does not exist".to_owned())?
            .clone();
        if source.role != "assistant" {
            return Err("content source must be an assistant-authored turn".into());
        }
        let authority = episode_turns
            .iter()
            .find(|turn| turn.node_id == candidate.source_node_id)
            .ok_or_else(|| "authority source disappeared from episode".to_owned())?;
        if source.timestamp_ns > authority.timestamp_ns {
            return Err("content source must not postdate user authority".into());
        }
        let content = self
            .archive
            .content(&mut self.container, source.content_id)
            .map_err(|_| "content source body is unavailable".to_owned())?;
        if quote.trim().is_empty() || !content.contains(quote.trim()) {
            return Err("content source quote is not verbatim".into());
        }
        Ok(())
    }
}
