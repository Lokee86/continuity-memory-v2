use crate::interaction_session::SessionState;
use crate::{
    ArchiveError, ConversationSummary, Cva, CvaError, EpisodePolicy, EpisodeSchedulingResult,
    IngestedTurn, InteractionError, InteractionTurn, ResolvedTurn,
};
use std::collections::HashMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InteractionReceipt {
    pub turn: IngestedTurn,
    pub archive_version: u64,
}

#[derive(Debug)]
pub struct InteractionCompletion {
    pub receipt: InteractionReceipt,
    pub scheduling: Result<EpisodeSchedulingResult, InteractionError>,
}

pub struct InteractionRuntime {
    pub(crate) cva: Cva,
    pub(crate) sessions: HashMap<String, SessionState>,
}

impl InteractionRuntime {
    pub fn new(cva: Cva) -> Self {
        Self {
            cva,
            sessions: HashMap::new(),
        }
    }

    pub fn accept_turn(&mut self, turn: InteractionTurn) -> Result<InteractionReceipt, CvaError> {
        let turn = self.cva.ingest_turn(turn.into())?;
        self.cva.sync()?;
        Ok(InteractionReceipt {
            turn,
            archive_version: self.cva.archive_version(),
        })
    }

    pub fn schedule_session(
        &mut self,
        session_id: &str,
        policy: EpisodePolicy,
        now_ns: i64,
    ) -> Result<EpisodeSchedulingResult, InteractionError> {
        let leaf = self.session_leaf(session_id)?;
        let result = self
            .cva
            .materialize_live_path_and_queue(session_id, &leaf, policy, now_ns)?;
        self.cva.sync()?;
        Ok(result)
    }

    pub fn finalize_inactive_session(
        &mut self,
        session_id: &str,
        policy: EpisodePolicy,
        now_ns: i64,
    ) -> Result<Option<EpisodeSchedulingResult>, InteractionError> {
        let leaf = self.session_leaf(session_id)?;
        let result = self
            .cva
            .finalize_inactive_path_and_queue(session_id, &leaf, policy, now_ns)?;
        if result.is_some() {
            self.cva.sync()?;
        }
        Ok(result)
    }

    pub fn conversation_summaries(&self) -> Vec<ConversationSummary> {
        self.cva.conversation_summaries()
    }

    pub fn conversation_turns(
        &mut self,
        conversation_id: &str,
        leaf_node_id: &str,
    ) -> Result<Vec<ResolvedTurn>, ArchiveError> {
        self.cva.conversation_turns(conversation_id, leaf_node_id)
    }

    pub fn cva(&self) -> &Cva {
        &self.cva
    }

    pub fn into_cva(self) -> Cva {
        self.cva
    }

    fn session_leaf(&self, session_id: &str) -> Result<String, InteractionError> {
        self.sessions
            .get(session_id)
            .ok_or(InteractionError::UnknownSession)?
            .leaf_message_id
            .clone()
            .ok_or(InteractionError::SessionHasNoDurableTurn)
    }
}
