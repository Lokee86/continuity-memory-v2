use crate::{ArchiveError, Cva, EpisodeBoundary, EpisodeBuildResult, EpisodeConfig, EpisodeOrigin};

pub const DEFAULT_EPISODE_INACTIVITY_NS: i64 = 15 * 60 * 1_000_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EpisodePolicy {
    pub episode: EpisodeConfig,
    pub inactivity_ns: i64,
}

impl Default for EpisodePolicy {
    fn default() -> Self {
        Self {
            episode: EpisodeConfig::default(),
            inactivity_ns: DEFAULT_EPISODE_INACTIVITY_NS,
        }
    }
}

impl EpisodePolicy {
    pub fn validate(self) -> Result<Self, ArchiveError> {
        if self.episode.max_input_bytes == 0 || self.inactivity_ns <= 0 {
            return Err(ArchiveError::InvalidEpisodeConfig);
        }
        Ok(self)
    }

    pub fn inactive(self, last_source_ns: i64, now_ns: i64) -> bool {
        now_ns.saturating_sub(last_source_ns) >= self.inactivity_ns
    }
}

impl Cva {
    pub fn finalize_inactive_path_episodes(
        &mut self,
        conversation_id: &str,
        leaf_node_id: &str,
        policy: EpisodePolicy,
        origin: EpisodeOrigin,
        now_ns: i64,
    ) -> Result<Option<EpisodeBuildResult>, ArchiveError> {
        let policy = policy.validate()?;
        let last_source_ns = self
            .archive
            .require_node(conversation_id, leaf_node_id, ArchiveError::MissingLeaf)?
            .timestamp_ns;
        if !policy.inactive(last_source_ns, now_ns) {
            return Ok(None);
        }
        self.materialize_path_episodes(
            conversation_id,
            leaf_node_id,
            policy.episode,
            origin,
            Some((EpisodeBoundary::Inactivity, now_ns)),
        )
        .map(Some)
    }

    pub fn finalize_create_memory_path(
        &mut self,
        conversation_id: &str,
        leaf_node_id: &str,
        config: EpisodeConfig,
        now_ns: i64,
    ) -> Result<EpisodeBuildResult, ArchiveError> {
        self.materialize_path_episodes(
            conversation_id,
            leaf_node_id,
            config,
            EpisodeOrigin::Live,
            Some((EpisodeBoundary::CreateMemory, now_ns)),
        )
    }
}
