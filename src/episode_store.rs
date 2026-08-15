use crate::episode_codec::encode_episode;
use crate::episode_model::episode_id;
use crate::{Archive, ArchiveError, Container, Episode, EpisodeId, Node};

impl Archive {
    pub(crate) fn put_episode(
        &mut self,
        container: &mut Container,
        mut episode: Episode,
    ) -> Result<bool, ArchiveError> {
        episode.id = episode_id(
            &episode.conversation_id,
            &episode.start_node_id,
            &episode.end_node_id,
        );
        self.validate_episode(&episode)?;
        if let Some(existing) = self.episodes.get(episode.id) {
            return if existing == &episode {
                Ok(false)
            } else {
                Err(ArchiveError::ConflictingEpisode)
            };
        }
        let record = container.append(&encode_episode(&episode)?)?;
        self.publish_record(container, record)?;
        self.episodes.insert(episode)
    }

    pub fn episodes(&self) -> Vec<Episode> {
        self.episodes.iter().cloned().collect()
    }

    pub fn episodes_for_conversation(&self, conversation_id: &str) -> Vec<Episode> {
        self.episodes
            .for_conversation(conversation_id)
            .into_iter()
            .cloned()
            .collect()
    }

    pub fn episode(&self, id: EpisodeId) -> Option<&Episode> {
        self.episodes.get(id)
    }

    pub(crate) fn episode_contains_node(
        &self,
        episode_id: EpisodeId,
        node_id: &str,
    ) -> Result<bool, ArchiveError> {
        let episode = self
            .episodes
            .get(episode_id)
            .ok_or(ArchiveError::InvalidEpisodeRange)?;
        let nodes = self.branch_nodes(&episode.conversation_id, &episode.end_node_id)?;
        let start = nodes
            .iter()
            .position(|node| node.id == episode.start_node_id)
            .ok_or(ArchiveError::InvalidEpisodeRange)?;
        Ok(nodes[start..].iter().any(|node| node.id == node_id))
    }

    pub(crate) fn validate_episodes(&self) -> Result<(), ArchiveError> {
        for episode in self.episodes.iter() {
            self.validate_episode(episode)?;
        }
        Ok(())
    }

    fn validate_episode(&self, episode: &Episode) -> Result<(), ArchiveError> {
        if episode.conversation_id.is_empty()
            || episode.start_node_id.is_empty()
            || episode.end_node_id.is_empty()
            || episode.finalized_at_ns < episode.source_through_ns
        {
            return Err(ArchiveError::InvalidEpisodeRange);
        }
        let start = self.require_node(
            &episode.conversation_id,
            &episode.start_node_id,
            ArchiveError::MissingEpisodeNode,
        )?;
        let end = self.require_node(
            &episode.conversation_id,
            &episode.end_node_id,
            ArchiveError::MissingEpisodeNode,
        )?;
        if end.timestamp_ns != episode.source_through_ns {
            return Err(ArchiveError::InvalidEpisodeRange);
        }
        if !self.node_descends_from(end, start)? {
            return Err(ArchiveError::InvalidEpisodeRange);
        }
        Ok(())
    }

    fn node_descends_from(&self, end: &Node, start: &Node) -> Result<bool, ArchiveError> {
        let nodes = self.branch_nodes(&end.conversation_id, &end.id)?;
        Ok(nodes.iter().any(|node| node.id == start.id))
    }
}
