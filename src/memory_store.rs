use crate::memory_codec::{
    MemoryVersion, encode_body, encode_format, encode_record, encode_version,
};
use crate::memory_model::{MemoryRecord, memory_body_bytes, memory_body_id, memory_id};
use crate::{
    ChunkRef, Container, Memory, MemoryBodyId, MemoryDraft, MemoryError, MemoryId, MemoryStats,
};
use std::collections::HashMap;

pub(crate) struct MemoryStore {
    bodies: HashMap<MemoryBodyId, ChunkRef>,
    records: Vec<MemoryRecord>,
    current: HashMap<MemoryId, usize>,
    by_mutation: HashMap<String, usize>,
    next_memory_version: u64,
}

impl MemoryStore {
    pub(crate) fn empty() -> Self {
        Self {
            bodies: HashMap::new(),
            records: Vec::new(),
            current: HashMap::new(),
            by_mutation: HashMap::new(),
            next_memory_version: 1,
        }
    }

    pub(crate) fn initialize(&self, container: &mut Container) -> Result<(), MemoryError> {
        container.append(&encode_format())?;
        Ok(())
    }

    pub(crate) fn publish(
        &mut self,
        container: &mut Container,
        id: Option<MemoryId>,
        expected_revision: u64,
        draft: MemoryDraft,
    ) -> Result<(Memory, bool), MemoryError> {
        validate_draft(&draft)?;
        if let Some(index) = self.by_mutation.get(&draft.mutation_id).copied() {
            let existing = self.resolve_record(container, &self.records[index])?;
            return if same_draft(&existing, &draft) {
                Ok((existing, false))
            } else {
                Err(MemoryError::MutationConflict)
            };
        }
        let id = id.unwrap_or_else(|| memory_id(&draft.mutation_id));
        let current_revision = self
            .current
            .get(&id)
            .map(|index| self.records[*index].revision)
            .unwrap_or(0);
        if current_revision != expected_revision {
            return Err(MemoryError::RevisionConflict);
        }
        let revision = expected_revision
            .checked_add(1)
            .ok_or(MemoryError::VersionExhausted)?;
        let candidate_body_id = memory_body_id(&draft.title, &draft.content);
        if let Some(index) = self.current.get(&id).copied() {
            if self.records[index].body_id != candidate_body_id {
                return Err(MemoryError::SemanticMutation);
            }
        }
        let body_id = self.put_body(container, &draft.title, &draft.content)?;
        let memory_version = self.next_memory_version;
        let next_memory_version = memory_version
            .checked_add(1)
            .ok_or(MemoryError::VersionExhausted)?;
        let mut record = MemoryRecord {
            id,
            revision,
            body_id,
            category: draft.category.clone(),
            memory_type: draft.memory_type.clone(),
            scope: draft.scope.clone(),
            lifecycle_state: draft.lifecycle_state.clone(),
            archived: draft.archived,
            superseded_by: draft.superseded_by,
            parent_id: draft.parent_id,
            source_node_id: draft.source_node_id.clone(),
            content_source_conversation_id: draft.content_source_conversation_id.clone(),
            content_source_node_id: draft.content_source_node_id.clone(),
            source_episode_id: draft.source_episode_id,
            mutation_id: draft.mutation_id.clone(),
            created_at_ns: draft.created_at_ns,
            updated_at_ns: draft.updated_at_ns,
            global_version: 0,
            memory_version,
        };
        let record_chunk = container.append(&encode_record(&record)?)?;
        let global_version = container.allocate_version()?;
        container.append(&encode_version(MemoryVersion {
            global_version,
            memory_version,
            record: record_chunk,
        }))?;
        record.global_version = global_version;
        self.insert(record.clone())?;
        self.next_memory_version = next_memory_version;
        self.resolve_record(container, &record)
            .map(|memory| (memory, true))
    }

    pub(crate) fn memory(
        &self,
        container: &mut Container,
        id: MemoryId,
    ) -> Result<Memory, MemoryError> {
        let index = self.current.get(&id).ok_or(MemoryError::MissingMemory)?;
        self.resolve_record(container, &self.records[*index])
    }

    pub(crate) fn memory_revision(
        &self,
        container: &mut Container,
        id: MemoryId,
        revision: u64,
    ) -> Result<Memory, MemoryError> {
        let record = self
            .records
            .iter()
            .find(|record| record.id == id && record.revision == revision)
            .ok_or(MemoryError::MissingMemory)?;
        self.resolve_record(container, record)
    }

    pub(crate) fn records(&self) -> &[MemoryRecord] {
        &self.records
    }

    pub(crate) fn memory_version(&self) -> u64 {
        self.next_memory_version.saturating_sub(1)
    }

    pub(crate) fn contains_memory(&self, id: MemoryId) -> bool {
        self.current.contains_key(&id)
    }

    pub(crate) fn contains_body(&self, id: MemoryBodyId) -> bool {
        self.bodies.contains_key(&id)
    }

    pub(crate) fn current_body_ids(&self) -> Vec<MemoryBodyId> {
        let mut ids: Vec<_> = self
            .current
            .values()
            .map(|index| self.records[*index].body_id)
            .collect();
        ids.sort_by_key(|id| id.0);
        ids.dedup();
        ids
    }

    pub(crate) fn current_body_id(&self, id: MemoryId) -> Result<MemoryBodyId, MemoryError> {
        let index = self.current.get(&id).ok_or(MemoryError::MissingMemory)?;
        Ok(self.records[*index].body_id)
    }

    pub(crate) fn body_embedding_text(
        &self,
        container: &mut Container,
        id: MemoryBodyId,
    ) -> Result<String, MemoryError> {
        let chunk = self.bodies.get(&id).ok_or(MemoryError::MissingBody)?;
        let body = decode_body_payload(&container.read(*chunk)?)?;
        let (title, content) = decode_memory_body(&body)?;
        Ok(format!("{title}\n\n{content}"))
    }

    pub(crate) fn stats(&self) -> MemoryStats {
        MemoryStats {
            memories: self.current.len(),
            revisions: self.records.len(),
            bodies: self.bodies.len(),
            memory_version: self.memory_version(),
        }
    }

    pub(crate) fn insert_body(&mut self, id: MemoryBodyId, chunk: ChunkRef) -> bool {
        if self.bodies.contains_key(&id) {
            return false;
        }
        self.bodies.insert(id, chunk);
        true
    }

    pub(crate) fn insert_rebuilt(&mut self, record: MemoryRecord) -> Result<(), MemoryError> {
        if record.memory_version != self.next_memory_version {
            return Err(MemoryError::InvalidVersion);
        }
        self.insert(record)?;
        self.next_memory_version = self
            .next_memory_version
            .checked_add(1)
            .ok_or(MemoryError::VersionExhausted)?;
        Ok(())
    }

    pub(crate) fn validate_bodies(&self) -> Result<(), MemoryError> {
        if self
            .records
            .iter()
            .all(|record| self.bodies.contains_key(&record.body_id))
        {
            Ok(())
        } else {
            Err(MemoryError::MissingBody)
        }
    }

    pub(crate) fn validate_provenance(&self, archive: &crate::Archive) -> Result<(), MemoryError> {
        for record in &self.records {
            match (record.source_episode_id, record.source_node_id.as_deref()) {
                (Some(episode_id), Some(node_id)) => {
                    if !archive
                        .episode_contains_node(episode_id, node_id)
                        .map_err(|_| MemoryError::InvalidProvenance)?
                    {
                        return Err(MemoryError::InvalidProvenance);
                    }
                }
                (None, None) => {}
                _ => return Err(MemoryError::InvalidProvenance),
            }
            match (
                record.content_source_conversation_id.as_deref(),
                record.content_source_node_id.as_deref(),
            ) {
                (Some(conversation_id), Some(node_id))
                    if archive.has_node(conversation_id, node_id) => {}
                (None, None) => {}
                _ => return Err(MemoryError::InvalidProvenance),
            }
        }
        Ok(())
    }

    fn insert(&mut self, record: MemoryRecord) -> Result<(), MemoryError> {
        if record.revision == 0 || record.memory_version == 0 || record.global_version == 0 {
            return Err(MemoryError::InvalidVersion);
        }
        if let Some(index) = self.by_mutation.get(&record.mutation_id).copied() {
            return if self.records[index] == record {
                Ok(())
            } else {
                Err(MemoryError::MutationConflict)
            };
        }
        let expected = self
            .current
            .get(&record.id)
            .map(|index| self.records[*index].revision + 1)
            .unwrap_or(1);
        if record.revision != expected {
            return Err(MemoryError::RevisionConflict);
        }
        if let Some(index) = self.current.get(&record.id).copied() {
            if self.records[index].body_id != record.body_id {
                return Err(MemoryError::SemanticMutation);
            }
        }
        let index = self.records.len();
        self.by_mutation.insert(record.mutation_id.clone(), index);
        self.current.insert(record.id, index);
        self.records.push(record);
        Ok(())
    }

    fn put_body(
        &mut self,
        container: &mut Container,
        title: &str,
        content: &str,
    ) -> Result<MemoryBodyId, MemoryError> {
        let id = memory_body_id(title, content);
        let bytes = memory_body_bytes(title, content);
        if let Some(chunk) = self.bodies.get(&id).copied() {
            let stored = decode_body_payload(&container.read(chunk)?)?;
            return if stored == bytes {
                Ok(id)
            } else {
                Err(MemoryError::HashCollision)
            };
        }
        let chunk = container.append(&encode_body(id, &bytes)?)?;
        self.bodies.insert(id, chunk);
        Ok(id)
    }

    fn resolve_record(
        &self,
        container: &mut Container,
        record: &MemoryRecord,
    ) -> Result<Memory, MemoryError> {
        let chunk = self
            .bodies
            .get(&record.body_id)
            .ok_or(MemoryError::MissingBody)?;
        let body = decode_body_payload(&container.read(*chunk)?)?;
        let (title, content) = decode_memory_body(&body)?;
        Ok(Memory {
            id: record.id,
            revision: record.revision,
            category: record.category.clone(),
            memory_type: record.memory_type.clone(),
            title,
            content,
            scope: record.scope.clone(),
            lifecycle_state: record.lifecycle_state.clone(),
            archived: record.archived,
            superseded_by: record.superseded_by,
            parent_id: record.parent_id,
            source_node_id: record.source_node_id.clone(),
            content_source_conversation_id: record.content_source_conversation_id.clone(),
            content_source_node_id: record.content_source_node_id.clone(),
            source_episode_id: record.source_episode_id,
            mutation_id: record.mutation_id.clone(),
            created_at_ns: record.created_at_ns,
            updated_at_ns: record.updated_at_ns,
            global_version: record.global_version,
            memory_version: record.memory_version,
        })
    }
}

fn decode_body_payload(record: &[u8]) -> Result<Vec<u8>, MemoryError> {
    crate::memory_codec::decode_body(record)?
        .map(|(_, body)| body)
        .ok_or(MemoryError::CorruptBody)
}

fn decode_memory_body(bytes: &[u8]) -> Result<(String, String), MemoryError> {
    if bytes.len() < 16 {
        return Err(MemoryError::CorruptBody);
    }
    let title_len = u64::from_le_bytes(bytes[0..8].try_into().unwrap()) as usize;
    let title_end = 8usize
        .checked_add(title_len)
        .ok_or(MemoryError::FieldTooLarge)?;
    let content_len_end = title_end.checked_add(8).ok_or(MemoryError::FieldTooLarge)?;
    let title = bytes.get(8..title_end).ok_or(MemoryError::CorruptBody)?;
    let content_len_raw = bytes
        .get(title_end..content_len_end)
        .ok_or(MemoryError::CorruptBody)?;
    let content_len = u64::from_le_bytes(content_len_raw.try_into().unwrap()) as usize;
    let content_end = content_len_end
        .checked_add(content_len)
        .ok_or(MemoryError::FieldTooLarge)?;
    let content = bytes
        .get(content_len_end..content_end)
        .ok_or(MemoryError::CorruptBody)?;
    if content_end != bytes.len() {
        return Err(MemoryError::CorruptBody);
    }
    Ok((
        String::from_utf8(title.to_vec()).map_err(|_| MemoryError::InvalidUtf8)?,
        String::from_utf8(content.to_vec()).map_err(|_| MemoryError::InvalidUtf8)?,
    ))
}

fn validate_draft(draft: &MemoryDraft) -> Result<(), MemoryError> {
    for (value, field) in [
        (&draft.category, "category"),
        (&draft.memory_type, "memory type"),
        (&draft.title, "title"),
        (&draft.content, "content"),
        (&draft.scope, "scope"),
        (&draft.lifecycle_state, "lifecycle state"),
        (&draft.mutation_id, "mutation id"),
    ] {
        if value.trim().is_empty() {
            return Err(MemoryError::InvalidField(field));
        }
    }
    if draft.updated_at_ns < draft.created_at_ns {
        return Err(MemoryError::InvalidField("updated_at_ns"));
    }
    Ok(())
}

fn same_draft(memory: &Memory, draft: &MemoryDraft) -> bool {
    memory.category == draft.category
        && memory.memory_type == draft.memory_type
        && memory.title == draft.title
        && memory.content == draft.content
        && memory.scope == draft.scope
        && memory.lifecycle_state == draft.lifecycle_state
        && memory.archived == draft.archived
        && memory.superseded_by == draft.superseded_by
        && memory.parent_id == draft.parent_id
        && memory.source_node_id == draft.source_node_id
        && memory.content_source_conversation_id == draft.content_source_conversation_id
        && memory.content_source_node_id == draft.content_source_node_id
        && memory.source_episode_id == draft.source_episode_id
        && memory.mutation_id == draft.mutation_id
        && memory.created_at_ns == draft.created_at_ns
        && memory.updated_at_ns == draft.updated_at_ns
}
