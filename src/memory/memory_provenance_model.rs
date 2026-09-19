use crate::{Episode, Memory, ResolvedTurn};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryProvenance {
    pub memory: Memory,
    pub source_episode: Option<Episode>,
    pub source_turn: Option<ResolvedTurn>,
    pub source_episode_turns: Vec<ResolvedTurn>,
    pub content_source_turn: Option<ResolvedTurn>,
    pub grounding_source_turn: Option<ResolvedTurn>,
}

impl MemoryProvenance {
    pub fn has_evidence(&self) -> bool {
        self.source_turn.is_some()
            || !self.source_episode_turns.is_empty()
            || self.content_source_turn.is_some()
            || self.grounding_source_turn.is_some()
    }
}
