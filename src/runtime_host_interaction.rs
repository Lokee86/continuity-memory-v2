use super::{ReliquaryRuntimeHost, ReliquaryRuntimeHostError, operation};
use crate::{
    ConversationSummary, EpisodePolicy, InteractionCompletion, InteractionReceipt, InteractionRole,
    InteractionSession, ResolvedInteractionTurn,
};

impl ReliquaryRuntimeHost {
    fn with_phylactery<T>(
        &self,
        action: impl FnOnce(&mut crate::Phylactery) -> Result<T, ReliquaryRuntimeHostError>,
    ) -> Result<Option<T>, ReliquaryRuntimeHostError> {
        let mut phylactery = self
            .phylactery
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        phylactery.as_mut().map(action).transpose()
    }

    fn with_runtime<T>(
        &self,
        action: impl FnOnce(&mut crate::InteractionRuntime) -> Result<T, ReliquaryRuntimeHostError>,
    ) -> Result<T, ReliquaryRuntimeHostError> {
        let runtime = self.runtime.as_ref().ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation("Reliquary runtime is unavailable".into())
        })?;
        let mut runtime = runtime
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        action(&mut runtime)
    }

    pub fn owner_id(&self) -> Result<Option<String>, ReliquaryRuntimeHostError> {
        self.with_runtime(|runtime| Ok(runtime.cva().owner_id()))
    }

    pub fn phylactery_owner_id(&self) -> Result<Option<String>, ReliquaryRuntimeHostError> {
        Ok(self
            .with_phylactery(|phylactery| Ok(phylactery.owner_id()))?
            .flatten())
    }

    pub fn conversation_summaries(
        &self,
    ) -> Result<Vec<ConversationSummary>, ReliquaryRuntimeHostError> {
        self.with_runtime(|runtime| Ok(runtime.conversation_summaries()))
    }

    pub fn conversation_transcript(
        &self,
        conversation_id: &str,
        leaf_node_id: &str,
    ) -> Result<Vec<ResolvedInteractionTurn>, ReliquaryRuntimeHostError> {
        self.with_runtime(|runtime| {
            runtime
                .conversation_transcript(conversation_id, leaf_node_id)
                .map_err(operation)
        })
    }

    pub fn open_session(
        &self,
        session_id: String,
        resume_from: Option<String>,
    ) -> Result<InteractionSession, ReliquaryRuntimeHostError> {
        self.with_runtime(|runtime| {
            runtime
                .open_session(session_id, resume_from)
                .map_err(operation)
        })
    }

    pub fn close_session(
        &self,
        session_id: &str,
    ) -> Result<InteractionSession, ReliquaryRuntimeHostError> {
        self.with_runtime(|runtime| runtime.close_session(session_id).map_err(operation))
    }

    pub fn begin_message(
        &self,
        session_id: &str,
        message_id: String,
        role: InteractionRole,
        timestamp_ns: i64,
    ) -> Result<(), ReliquaryRuntimeHostError> {
        self.with_runtime(|runtime| {
            runtime
                .begin_message(session_id, message_id, role, timestamp_ns)
                .map_err(operation)
        })
    }

    pub fn append_text(
        &self,
        session_id: &str,
        message_id: &str,
        delta: &str,
    ) -> Result<(), ReliquaryRuntimeHostError> {
        self.with_runtime(|runtime| {
            runtime
                .append_text(session_id, message_id, delta)
                .map_err(operation)
        })
    }

    pub fn append_checkpointed_text(
        &self,
        session_id: &str,
        message_id: &str,
        delta: &str,
    ) -> Result<(), ReliquaryRuntimeHostError> {
        self.with_runtime(|runtime| {
            runtime
                .append_checkpointed_text(session_id, message_id, delta)
                .map_err(operation)
        })
    }

    pub fn interrupt_message(
        &self,
        session_id: &str,
        message_id: &str,
    ) -> Result<(), ReliquaryRuntimeHostError> {
        self.with_runtime(|runtime| {
            runtime
                .interrupt_message(session_id, message_id)
                .map_err(operation)
        })
    }

    pub fn complete_message(
        &self,
        session_id: &str,
        message_id: &str,
    ) -> Result<InteractionReceipt, ReliquaryRuntimeHostError> {
        self.with_runtime(|runtime| {
            runtime
                .complete_message(session_id, message_id)
                .map_err(operation)
        })
    }

    pub fn complete_live_message(
        &self,
        session_id: &str,
        message_id: &str,
        policy: EpisodePolicy,
        now_ns: i64,
    ) -> Result<InteractionCompletion, ReliquaryRuntimeHostError> {
        let completion = self.with_runtime(|runtime| {
            runtime
                .complete_live_message(session_id, message_id, policy, now_ns)
                .map_err(operation)
        })?;
        self.wake()?;
        Ok(completion)
    }

    pub fn sync(&self) -> Result<(), ReliquaryRuntimeHostError> {
        self.with_runtime(|runtime| runtime.cva().sync().map_err(operation))
    }

    pub fn insomnia_stats(&self) -> Result<crate::InsomniaStats, ReliquaryRuntimeHostError> {
        self.with_runtime(|runtime| Ok(runtime.cva().insomnia_stats()))
    }

    pub fn memory_stats(&self) -> Result<crate::MemoryStats, ReliquaryRuntimeHostError> {
        self.with_runtime(|runtime| Ok(runtime.cva().memory_stats()))
    }

    pub fn memory_vector_stats(
        &self,
    ) -> Result<crate::MemoryVectorStats, ReliquaryRuntimeHostError> {
        self.with_runtime(|runtime| Ok(runtime.cva().memory_vector_stats()))
    }

    pub fn phylactery_memory_stats(
        &self,
    ) -> Result<Option<crate::MemoryStats>, ReliquaryRuntimeHostError> {
        self.with_phylactery(|phylactery| Ok(phylactery.memory_stats()))
    }

    pub fn phylactery_memory_vector_stats(
        &self,
    ) -> Result<Option<crate::MemoryVectorStats>, ReliquaryRuntimeHostError> {
        self.with_phylactery(|phylactery| Ok(phylactery.memory_vector_stats()))
    }
}
