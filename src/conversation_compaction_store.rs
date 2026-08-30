use crate::conversation_compaction_codec::{DecodedCompactionChunk, decode, encode_live};
use crate::{ChunkRef, Container, ConversationCompaction, ConversationCompactionError};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub(crate) enum StoredCompactionChunk {
    Free(ChunkRef),
    Live {
        chunk: ChunkRef,
        record: ConversationCompaction,
        supersedes: Option<String>,
    },
}

impl StoredCompactionChunk {
    pub(crate) fn chunk(&self) -> ChunkRef {
        match self {
            Self::Free(chunk) | Self::Live { chunk, .. } => *chunk,
        }
    }

    pub(crate) fn is_free(&self) -> bool {
        matches!(self, Self::Free(_))
    }
}

#[derive(Default)]
pub(crate) struct ConversationCompactionOpenState {
    chunks: BTreeMap<u64, StoredCompactionChunk>,
    next_generation: u64,
}

impl ConversationCompactionOpenState {
    pub(crate) fn ingest(
        &mut self,
        chunk: ChunkRef,
        payload: &[u8],
    ) -> Result<(), ConversationCompactionError> {
        let Some(decoded) = decode(payload)? else {
            return Ok(());
        };
        let stored = match decoded {
            DecodedCompactionChunk::Free => StoredCompactionChunk::Free(chunk),
            DecodedCompactionChunk::Live {
                record,
                supersedes_through_message_id,
            } => {
                self.next_generation = self
                    .next_generation
                    .max(record.generation.saturating_add(1));
                StoredCompactionChunk::Live {
                    chunk,
                    record,
                    supersedes: supersedes_through_message_id,
                }
            }
        };
        self.chunks.insert(chunk.offset, stored);
        Ok(())
    }

    pub(crate) fn finish(
        self,
        container: &mut Container,
    ) -> Result<ConversationCompactionStore, ConversationCompactionError> {
        let mut store = ConversationCompactionStore {
            chunks: self.chunks,
            next_generation: self.next_generation.max(1),
        };
        store.recover(container)?;
        Ok(store)
    }
}

#[derive(Default)]
pub(crate) struct ConversationCompactionStore {
    pub(crate) chunks: BTreeMap<u64, StoredCompactionChunk>,
    pub(crate) next_generation: u64,
}

impl ConversationCompactionStore {
    pub(crate) fn empty() -> Self {
        Self {
            chunks: BTreeMap::new(),
            next_generation: 1,
        }
    }

    pub(crate) fn all_records(&self) -> Vec<ConversationCompaction> {
        let mut records = self
            .chunks
            .values()
            .filter_map(|chunk| match chunk {
                StoredCompactionChunk::Live { record, .. } => Some(record.clone()),
                StoredCompactionChunk::Free(_) => None,
            })
            .collect::<Vec<_>>();
        records.sort_by_key(|record| record.generation);
        records
    }

    pub(crate) fn for_conversation(&self, conversation_id: &str) -> Vec<ConversationCompaction> {
        let mut records = self
            .chunks
            .values()
            .filter_map(|chunk| match chunk {
                StoredCompactionChunk::Live { record, .. }
                    if record.conversation_id == conversation_id =>
                {
                    Some(record.clone())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        records.sort_by_key(|record| record.generation);
        records
    }

    pub(crate) fn put(
        &mut self,
        container: &mut Container,
        conversation_id: String,
        through_message_id: String,
        summary: String,
        supersedes: Option<&str>,
    ) -> Result<ConversationCompaction, ConversationCompactionError> {
        let generation = self.next_generation;
        self.next_generation = generation
            .checked_add(1)
            .ok_or(ConversationCompactionError::GenerationExhausted)?;
        let record = ConversationCompaction {
            conversation_id,
            through_message_id,
            summary,
            generation,
        };
        let encoded = encode_live(&record, supersedes)?;
        let chunk = self.allocate(container, encoded.len() as u64)?;
        container.write_chunk_prefix(chunk, &encoded)?;
        container.sync()?;
        self.chunks.insert(
            chunk.offset,
            StoredCompactionChunk::Live {
                chunk,
                record: record.clone(),
                supersedes: supersedes.map(str::to_owned),
            },
        );
        if let Some(old_through) = supersedes
            && let Some(old_offset) = self.find_record_offset(&record.conversation_id, old_through)
            && old_offset != chunk.offset
        {
            self.free_and_coalesce(container, old_offset)?;
            container.sync()?;
        }
        Ok(record)
    }

    fn recover(&mut self, container: &mut Container) -> Result<(), ConversationCompactionError> {
        let live = self
            .chunks
            .iter()
            .filter_map(|(offset, chunk)| match chunk {
                StoredCompactionChunk::Live {
                    record, supersedes, ..
                } => Some((*offset, record.clone(), supersedes.clone())),
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut stale = Vec::new();
        for (offset, record, supersedes) in &live {
            if let Some(old) = supersedes
                && let Some(old_offset) = self.find_record_offset(&record.conversation_id, old)
                && old_offset != *offset
            {
                stale.push(old_offset);
            }
            for (other_offset, other, _) in &live {
                if other_offset != offset
                    && other.conversation_id == record.conversation_id
                    && other.through_message_id == record.through_message_id
                    && other.generation < record.generation
                {
                    stale.push(*other_offset);
                }
            }
        }
        stale.sort_unstable();
        stale.dedup();
        for offset in stale {
            if self
                .chunks
                .get(&offset)
                .is_some_and(|chunk| !chunk.is_free())
            {
                self.free_and_coalesce(container, offset)?;
            }
        }
        if !self.chunks.is_empty() {
            container.sync()?;
        }
        Ok(())
    }

    pub(crate) fn find_record_offset(&self, conversation_id: &str, through: &str) -> Option<u64> {
        self.chunks.iter().find_map(|(offset, chunk)| match chunk {
            StoredCompactionChunk::Live { record, .. }
                if record.conversation_id == conversation_id
                    && record.through_message_id == through =>
            {
                Some(*offset)
            }
            _ => None,
        })
    }
}
