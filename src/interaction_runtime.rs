use crate::interaction_session::SessionState;
use crate::{
    ArchiveError, ConversationSummary, Cva, CvaError, EpisodePolicy, EpisodeSchedulingResult,
    IngestedTurn, InteractionError, InteractionStreamStatus, InteractionTurn,
    InteractionTurnStatus, ResolvedInteractionTurn, ResolvedTurn,
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

    pub fn conversation_transcript(
        &mut self,
        conversation_id: &str,
        leaf_node_id: &str,
    ) -> Result<Vec<ResolvedInteractionTurn>, ArchiveError> {
        let durable = self.cva.conversation_turns(conversation_id, leaf_node_id)?;
        let path_ids = durable
            .iter()
            .map(|turn| turn.node_id.clone())
            .collect::<std::collections::HashSet<_>>();
        let mut transcript = durable
            .into_iter()
            .map(|turn| ResolvedInteractionTurn {
                message_id: turn.node_id,
                role: turn.role,
                timestamp_ns: turn.timestamp_ns,
                content: turn.content,
                status: InteractionTurnStatus::Complete,
            })
            .collect::<Vec<_>>();
        for record in self.cva.interaction_streams_for_session(conversation_id) {
            if self
                .cva
                .archive()
                .has_node(conversation_id, &record.message_id)
            {
                continue;
            }
            if let Some(parent) = record.parent_message_id.as_deref() {
                if !path_ids.contains(parent) {
                    continue;
                }
            }
            let actively_streaming = self
                .sessions
                .get(conversation_id)
                .and_then(|state| state.in_flight.as_ref())
                .is_some_and(|message| message.message_id == record.message_id);
            let status = match (record.status, actively_streaming) {
                (InteractionStreamStatus::Streaming, true) => InteractionTurnStatus::Streaming,
                _ => InteractionTurnStatus::Interrupted,
            };
            transcript.push(ResolvedInteractionTurn {
                message_id: record.message_id,
                role: record.role.archive_role().into(),
                timestamp_ns: record.timestamp_ns,
                content: record.content,
                status,
            });
        }
        transcript.sort_by_key(|turn| turn.timestamp_ns);
        Ok(transcript)
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
