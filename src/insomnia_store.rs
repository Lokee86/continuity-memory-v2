use crate::insomnia_codec::{encode_attempt, encode_format, encode_work};
use crate::{
    Archive, Container, EpisodeId, InsomniaAttempt, InsomniaError, InsomniaLeaseToken,
    InsomniaPriority, InsomniaStats, InsomniaWork, InsomniaWorkState, MemoryId,
};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::HashMap;

pub(crate) struct InsomniaStore {
    work: HashMap<EpisodeId, InsomniaWork>,
    attempts: Vec<InsomniaAttempt>,
}

impl InsomniaStore {
    pub(crate) fn empty() -> Self {
        Self {
            work: HashMap::new(),
            attempts: Vec::new(),
        }
    }

    pub(crate) fn initialize(&self, container: &mut Container) -> Result<(), InsomniaError> {
        container.append(&encode_format())?;
        Ok(())
    }

    pub(crate) fn queue(
        &mut self,
        container: &mut Container,
        episode_id: EpisodeId,
        priority: InsomniaPriority,
        now_ns: i64,
    ) -> Result<(InsomniaWork, bool), InsomniaError> {
        if let Some(existing) = self.work.get(&episode_id) {
            return Ok((existing.clone(), false));
        }
        let work = InsomniaWork {
            episode_id,
            priority,
            state: InsomniaWorkState::Pending,
            attempt_count: 0,
            lease_owner: None,
            lease_token: None,
            lease_expires_ns: None,
            retry_after_ns: None,
            last_error: None,
            updated_at_ns: now_ns,
        };
        self.persist_work(container, work.clone())?;
        Ok((work, true))
    }

    pub(crate) fn claim_next(
        &mut self,
        container: &mut Container,
        archive: &Archive,
        worker_id: &str,
        now_ns: i64,
        lease_duration_ns: i64,
    ) -> Result<Option<InsomniaWork>, InsomniaError> {
        if worker_id.trim().is_empty() || lease_duration_ns <= 0 {
            return Err(InsomniaError::InvalidField("worker lease"));
        }
        let episode_id = self
            .work
            .values()
            .filter(|work| eligible(work, now_ns))
            .min_by(|left, right| compare_work(archive, left, right))
            .map(|work| work.episode_id);
        let Some(episode_id) = episode_id else {
            return Ok(None);
        };
        let current = self.work.get(&episode_id).unwrap().clone();
        let attempt_count = current.attempt_count.saturating_add(1);
        let token = lease_token(episode_id, worker_id, attempt_count, now_ns);
        let mut claimed = current;
        claimed.state = InsomniaWorkState::Processing;
        claimed.attempt_count = attempt_count;
        claimed.lease_owner = Some(worker_id.trim().to_owned());
        claimed.lease_token = Some(token);
        claimed.lease_expires_ns = Some(now_ns.saturating_add(lease_duration_ns));
        claimed.retry_after_ns = None;
        claimed.last_error = None;
        claimed.updated_at_ns = now_ns;
        self.persist_work(container, claimed.clone())?;
        Ok(Some(claimed))
    }

    pub(crate) fn renew(
        &mut self,
        container: &mut Container,
        episode_id: EpisodeId,
        token: InsomniaLeaseToken,
        now_ns: i64,
        lease_duration_ns: i64,
    ) -> Result<InsomniaWork, InsomniaError> {
        if lease_duration_ns <= 0 {
            return Err(InsomniaError::InvalidField("lease duration"));
        }
        let mut work = self.active_claim(episode_id, token, now_ns)?;
        work.lease_expires_ns = Some(now_ns.saturating_add(lease_duration_ns));
        work.updated_at_ns = now_ns;
        self.persist_work(container, work.clone())?;
        Ok(work)
    }

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
        container.append(&encode_attempt(&attempt)?)?;
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
        container.append(&encode_attempt(&attempt)?)?;
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
        container.append(&encode_attempt(&attempt)?)?;
        self.attempts.push(attempt);
        work.state = InsomniaWorkState::Terminal;
        clear_lease(&mut work);
        work.retry_after_ns = None;
        work.last_error = Some(reason);
        work.updated_at_ns = terminal_at_ns;
        self.persist_work(container, work.clone())?;
        Ok(work)
    }

    pub(crate) fn work(&self, episode_id: EpisodeId) -> Option<&InsomniaWork> {
        self.work.get(&episode_id)
    }

    pub(crate) fn attempts(&self, episode_id: EpisodeId) -> Vec<InsomniaAttempt> {
        self.attempts
            .iter()
            .filter(|attempt| attempt.episode_id == episode_id)
            .cloned()
            .collect()
    }

    pub(crate) fn stats(&self) -> InsomniaStats {
        let mut stats = InsomniaStats {
            total: self.work.len(),
            pending: 0,
            processing: 0,
            complete: 0,
            failed: 0,
            terminal: 0,
            attempts: self.attempts.len(),
        };
        for work in self.work.values() {
            match work.state {
                InsomniaWorkState::Pending => stats.pending += 1,
                InsomniaWorkState::Processing => stats.processing += 1,
                InsomniaWorkState::Complete => stats.complete += 1,
                InsomniaWorkState::Failed => stats.failed += 1,
                InsomniaWorkState::Terminal => stats.terminal += 1,
            }
        }
        stats
    }

    pub(crate) fn apply_work(&mut self, mut work: InsomniaWork) {
        if work.state == InsomniaWorkState::Processing {
            work.state = InsomniaWorkState::Pending;
            clear_lease(&mut work);
        }
        self.work.insert(work.episode_id, work);
    }

    pub(crate) fn apply_attempt(&mut self, attempt: InsomniaAttempt) {
        self.attempts.push(attempt);
    }

    pub(crate) fn validate(
        &self,
        archive: &Archive,
        memories: &crate::memory_store::MemoryStore,
    ) -> Result<(), InsomniaError> {
        if !self
            .work
            .keys()
            .all(|episode_id| archive.episode(*episode_id).is_some())
            || !self
                .attempts
                .iter()
                .all(|attempt| archive.episode(attempt.episode_id).is_some())
        {
            return Err(InsomniaError::MissingEpisode);
        }
        if self
            .attempts
            .iter()
            .flat_map(|attempt| attempt.memory_ids.iter())
            .all(|id| memories.contains_memory(*id))
        {
            Ok(())
        } else {
            Err(InsomniaError::InvalidTransition)
        }
    }

    fn active_claim(
        &self,
        episode_id: EpisodeId,
        token: InsomniaLeaseToken,
        now_ns: i64,
    ) -> Result<InsomniaWork, InsomniaError> {
        let work = self
            .work
            .get(&episode_id)
            .ok_or(InsomniaError::MissingWork)?;
        if work.state != InsomniaWorkState::Processing || work.lease_token != Some(token) {
            return Err(InsomniaError::InvalidLease);
        }
        if work
            .lease_expires_ns
            .is_none_or(|expires| expires <= now_ns)
        {
            return Err(InsomniaError::LeaseExpired);
        }
        Ok(work.clone())
    }

    fn persist_work(
        &mut self,
        container: &mut Container,
        work: InsomniaWork,
    ) -> Result<(), InsomniaError> {
        container.append(&encode_work(&work)?)?;
        self.work.insert(work.episode_id, work);
        Ok(())
    }
}

fn clear_lease(work: &mut InsomniaWork) {
    work.lease_owner = None;
    work.lease_token = None;
    work.lease_expires_ns = None;
}

fn eligible(work: &InsomniaWork, now_ns: i64) -> bool {
    match work.state {
        InsomniaWorkState::Pending => true,
        InsomniaWorkState::Failed => work.retry_after_ns.is_some_and(|at| at <= now_ns),
        InsomniaWorkState::Processing => work.lease_expires_ns.is_none_or(|at| at <= now_ns),
        InsomniaWorkState::Complete | InsomniaWorkState::Terminal => false,
    }
}

fn compare_work(archive: &Archive, left: &InsomniaWork, right: &InsomniaWork) -> Ordering {
    left.priority.cmp(&right.priority).then_with(|| {
        let left_episode = archive.episode(left.episode_id).unwrap();
        let right_episode = archive.episode(right.episode_id).unwrap();
        left_episode
            .source_through_ns
            .cmp(&right_episode.source_through_ns)
            .then_with(|| {
                left_episode
                    .conversation_id
                    .cmp(&right_episode.conversation_id)
            })
            .then_with(|| left_episode.start_node_id.cmp(&right_episode.start_node_id))
            .then_with(|| left.episode_id.0.cmp(&right.episode_id.0))
    })
}

fn lease_token(
    episode_id: EpisodeId,
    worker_id: &str,
    attempt: u32,
    now_ns: i64,
) -> InsomniaLeaseToken {
    let mut hash = Sha256::new();
    hash.update(b"continuity-insomnia-lease\0");
    hash.update(episode_id.0);
    hash.update((worker_id.len() as u64).to_le_bytes());
    hash.update(worker_id.as_bytes());
    hash.update(attempt.to_le_bytes());
    hash.update(now_ns.to_le_bytes());
    InsomniaLeaseToken(hash.finalize().into())
}
