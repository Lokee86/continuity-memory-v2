use crate::archive_lookup::DenseLookup;
use crate::{ArchiveError, ChunkRef, ContentId, Fragment, FragmentId};
use std::collections::HashMap;
use std::hash::BuildHasher;
use std::mem::size_of;

#[derive(Default)]
pub(crate) struct ContentIndex {
    records: HashMap<ContentId, ChunkRef>,
}

impl ContentIndex {
    pub(crate) fn len(&self) -> usize {
        self.records.len()
    }

    pub(crate) fn get(&self, id: ContentId) -> Option<ChunkRef> {
        self.records.get(&id).copied()
    }

    pub(crate) fn contains(&self, id: ContentId) -> bool {
        self.records.contains_key(&id)
    }

    pub(crate) fn insert(&mut self, id: ContentId, chunk: ChunkRef) -> bool {
        if self.records.contains_key(&id) {
            return false;
        }
        self.records.insert(id, chunk);
        true
    }

    pub(crate) fn record_capacity(&self) -> usize {
        self.records.capacity()
    }

    pub(crate) fn lookup_capacity(&self) -> usize {
        0
    }

    pub(crate) fn retained_heap_bytes(&self) -> usize {
        self.records.capacity() * size_of::<(ContentId, ChunkRef)>()
    }
}

#[derive(Default)]
pub(crate) struct FragmentIndex {
    records: Vec<Fragment>,
    lookup: DenseLookup,
}

impl FragmentIndex {
    pub(crate) fn len(&self) -> usize {
        self.records.len()
    }

    pub(crate) fn get(&self, id: FragmentId) -> Option<&Fragment> {
        let hash = self.lookup.hash(&id);
        self.lookup
            .find(hash, |index| self.records[index].id == id)
            .map(|index| &self.records[index])
    }

    pub(crate) fn insert(&mut self, fragment: Fragment) -> Result<bool, ArchiveError> {
        if let Some(existing) = self.get(fragment.id) {
            return if existing == &fragment {
                Ok(false)
            } else {
                Err(ArchiveError::ConflictingFragment)
            };
        }
        let hash = self.lookup.hash(&fragment.id);
        let index = self.records.len();
        self.records.push(fragment);
        let records = &self.records;
        self.lookup.insert(hash, index, |state, found| {
            state.hash_one(&records[found].id)
        });
        Ok(true)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &Fragment> {
        self.records.iter()
    }

    pub(crate) fn record_capacity(&self) -> usize {
        self.records.capacity()
    }

    pub(crate) fn lookup_capacity(&self) -> usize {
        self.lookup.slot_capacity()
    }

    pub(crate) fn string_heap_bytes(&self) -> usize {
        self.records
            .iter()
            .map(|fragment| {
                fragment.conversation_id.capacity()
                    + fragment.start_node_id.capacity()
                    + fragment.end_node_id.capacity()
            })
            .sum()
    }

    pub(crate) fn retained_heap_bytes(&self) -> usize {
        self.records.capacity() * size_of::<Fragment>()
            + self.lookup.retained_heap_bytes()
            + self.string_heap_bytes()
    }
}
