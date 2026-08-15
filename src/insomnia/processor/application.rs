use super::{InsomniaProcessError, InsomniaProcessResult};
use crate::cva_memory_publish::publish_memory_parts;
use crate::insomnia::candidate::hex;
use crate::insomnia::store::InsomniaStore;
use crate::memory_store::MemoryStore;
use crate::{
    Archive, Container, Episode, INSOMNIA_EXTRACTOR_CONTRACT_VERSION, InsomniaCandidate,
    InsomniaExtraction, InsomniaRejection, InsomniaWork, MemoryDraft,
};

pub(crate) struct PreparedApplication {
    pub(crate) drafts: Vec<MemoryDraft>,
    pub(crate) rejected: Vec<InsomniaRejection>,
    pub(crate) model: String,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_application(
    archive: &Archive,
    container: &mut Container,
    episode: &Episode,
    turns: &[crate::ResolvedTurn],
    extraction: InsomniaExtraction,
    scope: &str,
    completed_at_ns: i64,
) -> Result<PreparedApplication, InsomniaProcessError> {
    let mut rejected = extraction.rejected;
    let mut drafts = Vec::with_capacity(extraction.candidates.len());
    for candidate in extraction.candidates {
        if let Err(reason) = validate_content_source(
            archive,
            container,
            &candidate,
            &episode.conversation_id,
            turns,
            &extraction.evidence_turns,
        ) {
            rejected.push(InsomniaRejection {
                candidate_key: Some(candidate.key.clone()),
                reason,
            });
            continue;
        }
        drafts.push(MemoryDraft {
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
            mutation_id: format!("insomnia:{}:{}", hex(&episode.id.0), candidate.key),
            created_at_ns: episode.source_through_ns,
            updated_at_ns: completed_at_ns.max(episode.source_through_ns),
        });
    }
    Ok(PreparedApplication {
        drafts,
        rejected,
        model: extraction.model,
    })
}

pub(crate) fn publish_draft(
    archive: &Archive,
    container: &mut Container,
    memories: &mut MemoryStore,
    draft: MemoryDraft,
    result: &mut InsomniaProcessResult,
) -> Result<(), InsomniaProcessError> {
    let (memory, created) = publish_memory_parts(archive, memories, container, None, 0, draft)?;
    if created {
        result.created.push(memory);
    } else {
        result.existing.push(memory);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn finish_application(
    container: &mut Container,
    memories: &MemoryStore,
    insomnia: &mut InsomniaStore,
    claim: &InsomniaWork,
    started_at_ns: i64,
    completed_at_ns: i64,
    model: String,
    result: &InsomniaProcessResult,
) -> Result<(), InsomniaProcessError> {
    let memory_ids: Vec<_> = result
        .created
        .iter()
        .chain(&result.existing)
        .map(|memory| memory.id)
        .collect();
    if memory_ids.iter().any(|id| !memories.contains_memory(*id)) {
        return Err(crate::InsomniaError::InvalidTransition.into());
    }
    insomnia.complete(
        container,
        claim.episode_id,
        claim.lease_token.unwrap(),
        started_at_ns,
        completed_at_ns,
        model,
        INSOMNIA_EXTRACTOR_CONTRACT_VERSION.into(),
        memory_ids,
        result.rejected.len() as u32,
    )?;
    Ok(())
}

fn validate_content_source(
    archive: &Archive,
    container: &mut Container,
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
        return Err("external content source was not supplied by bounded archive evidence".into());
    }
    let source = archive
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
    let content = archive
        .content(container, source.content_id)
        .map_err(|_| "content source body is unavailable".to_owned())?;
    if quote.trim().is_empty() || !content.contains(quote.trim()) {
        return Err("content source quote is not verbatim".into());
    }
    Ok(())
}
