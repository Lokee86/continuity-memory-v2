use super::{ReliquaryRuntimeHost, ReliquaryRuntimeHostError, notify_work, operation};
use crate::{
    InteractionRuntime, ProjectRepositoryManagement, ProjectRevisionCorrelation, ProjectRevisionRef,
};
use std::sync::{Arc, Mutex};

impl ReliquaryRuntimeHost {
    pub(super) fn runtime_for_owner(
        &self,
        owner_id: &str,
    ) -> Result<Arc<Mutex<InteractionRuntime>>, ReliquaryRuntimeHostError> {
        self.execution_for(owner_id)?
            .runtime
            .as_ref()
            .cloned()
            .ok_or_else(|| {
                ReliquaryRuntimeHostError::Operation("Reliquary runtime is unavailable".into())
            })
    }

    pub(super) fn with_runtime_for<T>(
        &self,
        owner_id: &str,
        action: impl FnOnce(&mut InteractionRuntime) -> Result<T, ReliquaryRuntimeHostError>,
    ) -> Result<T, ReliquaryRuntimeHostError> {
        let runtime = self.runtime_for_owner(owner_id)?;
        let mut runtime = runtime
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        action(&mut runtime)
    }

    pub(super) fn wake_owner(&self, owner_id: &str) -> Result<(), ReliquaryRuntimeHostError> {
        notify_work(&self.execution_for(owner_id)?.signal)
    }

    pub fn embedding_route(
        &self,
    ) -> Result<Option<Arc<dyn crate::EmbeddingEndpoint + Send + Sync>>, ReliquaryRuntimeHostError>
    {
        self.routes
            .read()
            .map(|routes| routes.embedding())
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)
    }

    pub fn clear_active_conversations_for(
        &self,
        owner_id: &str,
    ) -> Result<usize, ReliquaryRuntimeHostError> {
        self.with_runtime_for(owner_id, |runtime| {
            let active_ids = runtime
                .conversation_summaries()
                .into_iter()
                .filter(|summary| summary.active)
                .map(|summary| summary.conversation_id)
                .collect::<Vec<_>>();
            let mut changed = 0;
            for conversation_id in active_ids {
                if runtime
                    .cva
                    .set_conversation_active(&conversation_id, false)
                    .map_err(operation)?
                {
                    changed += 1;
                }
            }
            if changed > 0 {
                runtime.cva.sync().map_err(operation)?;
            }
            Ok(changed)
        })
    }

    pub fn close_session_for(
        &self,
        owner_id: &str,
        session_id: &str,
    ) -> Result<crate::InteractionSession, ReliquaryRuntimeHostError> {
        let policy = self.execution_for(owner_id)?.episode_policy;
        let now_ns = crate::insomnia::runtime_step::now_ns();
        let session = self.with_runtime_for(owner_id, |runtime| {
            let has_durable_turn = runtime
                .session(session_id)
                .ok_or_else(|| operation("interaction session is not open"))?
                .leaf_message_id
                .is_some();
            if has_durable_turn {
                runtime
                    .finalize_explicit_session(session_id, policy, now_ns)
                    .map_err(operation)?;
            }
            runtime.close_session(session_id).map_err(operation)
        })?;
        self.wake_owner(owner_id)?;
        Ok(session)
    }

    pub fn sync_for(&self, owner_id: &str) -> Result<(), ReliquaryRuntimeHostError> {
        self.with_runtime_for(owner_id, |runtime| runtime.cva().sync().map_err(operation))
    }

    pub fn latest_project_revision_correlation_for(
        &self,
        owner_id: &str,
    ) -> Result<Option<ProjectRevisionCorrelation>, ReliquaryRuntimeHostError> {
        self.with_runtime_for(owner_id, |runtime| {
            Ok(runtime.cva.latest_project_revision_correlation())
        })
    }

    pub fn correlate_project_revision_with_management_for(
        &self,
        owner_id: &str,
        revision: ProjectRevisionRef,
        management: ProjectRepositoryManagement,
    ) -> Result<(ProjectRevisionCorrelation, bool), ReliquaryRuntimeHostError> {
        self.with_runtime_for(owner_id, |runtime| {
            runtime
                .cva
                .correlate_project_revision_with_management(revision, management)
                .map_err(operation)
        })
    }
}
