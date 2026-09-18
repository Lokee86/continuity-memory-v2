use crate::MemoryEntityMentionKey;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

#[derive(Default)]
struct OwnerQueue {
    order: VecDeque<MemoryEntityMentionKey>,
    queued: HashSet<MemoryEntityMentionKey>,
    not_before: HashMap<MemoryEntityMentionKey, Instant>,
    in_flight: usize,
}

impl OwnerQueue {
    fn push(&mut self, key: MemoryEntityMentionKey) {
        if self.queued.insert(key) {
            self.order.push_back(key);
        }
    }

    fn defer(&mut self, key: MemoryEntityMentionKey, until: Instant) {
        if self.queued.insert(key) {
            self.order.push_back(key);
        }
        self.not_before.insert(key, until);
    }

    fn pop_ready(&mut self, now: Instant) -> Option<MemoryEntityMentionKey> {
        let count = self.order.len();
        for _ in 0..count {
            let key = self.order.pop_front()?;
            if self.not_before.get(&key).is_some_and(|until| *until > now) {
                self.order.push_back(key);
                continue;
            }
            self.queued.remove(&key);
            self.not_before.remove(&key);
            self.in_flight = self.in_flight.saturating_add(1);
            return Some(key);
        }
        None
    }

    fn finish(&mut self) {
        self.in_flight = self.in_flight.saturating_sub(1);
    }

    fn len(&self) -> usize {
        self.queued.len().saturating_add(self.in_flight)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PerceptionOwner {
    Project,
    User,
}

#[derive(Default)]
pub(crate) struct PerceptionQueue {
    project: OwnerQueue,
    user: OwnerQueue,
}

impl PerceptionQueue {
    pub(super) fn push(&mut self, owner: PerceptionOwner, key: MemoryEntityMentionKey) {
        self.owner_mut(owner).push(key);
    }

    pub(super) fn push_all(
        &mut self,
        owner: PerceptionOwner,
        keys: impl IntoIterator<Item = MemoryEntityMentionKey>,
    ) {
        for key in keys {
            self.push(owner, key);
        }
    }

    pub(super) fn defer(
        &mut self,
        owner: PerceptionOwner,
        key: MemoryEntityMentionKey,
        until: Instant,
    ) {
        self.owner_mut(owner).defer(key, until);
    }

    pub(super) fn pop_ready(
        &mut self,
        owner: PerceptionOwner,
        now: Instant,
    ) -> Option<MemoryEntityMentionKey> {
        self.owner_mut(owner).pop_ready(now)
    }

    pub(super) fn finish(&mut self, owner: PerceptionOwner) {
        self.owner_mut(owner).finish();
    }

    pub(super) fn counts(&self) -> (usize, usize) {
        (self.project.len(), self.user.len())
    }

    pub(super) fn clear(&mut self, owner: PerceptionOwner) {
        *self.owner_mut(owner) = OwnerQueue::default();
    }

    fn owner_mut(&mut self, owner: PerceptionOwner) -> &mut OwnerQueue {
        match owner {
            PerceptionOwner::Project => &mut self.project,
            PerceptionOwner::User => &mut self.user,
        }
    }
}
