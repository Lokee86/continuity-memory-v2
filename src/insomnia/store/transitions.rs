use super::{InsomniaStore, clear_lease};
use crate::{
    Container, EpisodeId, InsomniaAttempt, InsomniaError, InsomniaLeaseToken, InsomniaWork,
    InsomniaWorkState, MemoryId,
};

impl InsomniaStore {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn complete(
        &mut self,
        container: &mut Container,
        episode_id: EpisodeId,
        token: InsomniaLeaseToken,
        started_at_ns: i64,
        completed_at_ns: i64,
        extractor_model: String,
        extractor_version: String,
        memory_ids: Vec<MemoryId>,
        rejected_count: u32,
    ) -> Result<InsomniaWork, InsomniaError> {
        let mut work = self.active_claim(episode_id, token, completed_at_ns)?;
        let attempt = InsomniaAttempt {
            episode_id,
            attempt: work.attempt_count,
            state: InsomniaWorkState::Complete,
            started_at_ns,
            completed_at_ns,
            extractor_model,
            extractor_version,
            memory_ids,
            rejected_count,
            error: None,
        };
        container.append(&super::super::codec::encode_attempt(&attempt)?)?;
        self.attempts.push(attempt);
        work.state = InsomniaWorkState::Complete;
        clear_lease(&mut work);
        work.retry_after_ns = None;
        work.last_error = None;
        work.updated_at_ns = completed_at_ns;
        self.persist_work(container, work.clone())?;
        Ok(work)
    }

    pub(crate) fn fail(
        &mut self,
        container: &mut Container,
        episode_id: EpisodeId,
        token: InsomniaLeaseToken,
        started_at_ns: i64,
        failed_at_ns: i64,
        retry_after_ns: i64,
        reason: String,
    ) -> Result<InsomniaWork, InsomniaError> {
        if reason.trim().is_empty() || retry_after_ns <= failed_at_ns {
            return Err(InsomniaError::InvalidField("retry failure"));
        }
        let mut work = self.active_claim(episode_id, token, failed_at_ns)?;
        let attempt = InsomniaAttempt {
            episode_id,
            attempt: work.attempt_count,
            state: InsomniaWorkState::Failed,
            started_at_ns,
            completed_at_ns: failed_at_ns,
            extractor_model: String::new(),
            extractor_version: String::new(),
            memory_ids: Vec::new(),
            rejected_count: 0,
            error: Some(reason.clone()),
        };
        container.append(&super::super::codec::encode_attempt(&attempt)?)?;
        self.attempts.push(attempt);
        work.state = InsomniaWorkState::Failed;
        clear_lease(&mut work);
        work.retry_after_ns = Some(retry_after_ns);
        work.last_error = Some(reason);
        work.updated_at_ns = failed_at_ns;
        self.persist_work(container, work.clone())?;
        Ok(work)
    }

    pub(crate) fn terminal(
        &mut self,
        container: &mut Container,
        episode_id: EpisodeId,
        token: InsomniaLeaseToken,
        started_at_ns: i64,
        terminal_at_ns: i64,
        reason: String,
    ) -> Result<InsomniaWork, InsomniaError> {
        if reason.trim().is_empty() {
            return Err(InsomniaError::InvalidField("terminal reason"));
        }
        let mut work = self.active_claim(episode_id, token, terminal_at_ns)?;
        let attempt = InsomniaAttempt {
            episode_id,
            attempt: work.attempt_count,
            state: InsomniaWorkState::Terminal,
            started_at_ns,
            completed_at_ns: terminal_at_ns,
            extractor_model: String::new(),
            extractor_version: String::new(),
            memory_ids: Vec::new(),
            rejected_count: 0,
            error: Some(reason.clone()),
        };
        container.append(&super::super::codec::encode_attempt(&attempt)?)?;
        self.attempts.push(attempt);
        work.state = InsomniaWorkState::Terminal;
        clear_lease(&mut work);
        work.retry_after_ns = None;
        work.last_error = Some(reason);
        work.updated_at_ns = terminal_at_ns;
        self.persist_work(container, work.clone())?;
        Ok(work)
    }
}
