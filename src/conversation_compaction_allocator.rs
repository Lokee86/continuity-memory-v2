use crate::conversation_compaction_codec::free_prefix;
use crate::conversation_compaction_store::{ConversationCompactionStore, StoredCompactionChunk};
use crate::{ChunkRef, Container, ConversationCompactionError};

const MIN_ALLOC_PAYLOAD: u64 = 256;
const ALLOC_GRANULARITY: u64 = 256;
const CHUNK_HEADER_LEN: u64 = 8;

impl ConversationCompactionStore {
    pub(crate) fn allocate(
        &mut self,
        container: &mut Container,
        required: u64,
    ) -> Result<ChunkRef, ConversationCompactionError> {
        let desired = round_up(required.max(MIN_ALLOC_PAYLOAD), ALLOC_GRANULARITY)?;
        let free = self
            .chunks
            .values()
            .filter_map(|chunk| match chunk {
                StoredCompactionChunk::Free(chunk) if chunk.len >= desired => Some(*chunk),
                _ => None,
            })
            .min_by_key(|chunk| chunk.len);
        let Some(free) = free else {
            let payload_len = usize::try_from(desired)
                .map_err(|_| ConversationCompactionError::RecordTooLarge)?;
            return Ok(container.append(&vec![0_u8; payload_len])?);
        };

        self.chunks.remove(&free.offset);
        let remainder = free
            .len
            .saturating_sub(desired)
            .saturating_sub(CHUNK_HEADER_LEN);
        if remainder >= MIN_ALLOC_PAYLOAD {
            let (allocated, leftover) = container.split_chunk(free, desired, &free_prefix())?;
            self.chunks
                .insert(leftover.offset, StoredCompactionChunk::Free(leftover));
            Ok(allocated)
        } else {
            Ok(free)
        }
    }

    pub(crate) fn free_and_coalesce(
        &mut self,
        container: &mut Container,
        offset: u64,
    ) -> Result<(), ConversationCompactionError> {
        let Some(old) = self.chunks.remove(&offset) else {
            return Ok(());
        };
        let chunk = old.chunk();
        container.write_chunk_prefix(chunk, &free_prefix())?;
        self.chunks
            .insert(chunk.offset, StoredCompactionChunk::Free(chunk));
        self.coalesce_at(container, chunk.offset)
    }

    fn coalesce_at(
        &mut self,
        container: &mut Container,
        mut offset: u64,
    ) -> Result<(), ConversationCompactionError> {
        loop {
            let current = self
                .chunks
                .get(&offset)
                .map(StoredCompactionChunk::chunk)
                .unwrap();
            if let Some(left) = self.left_free(offset)
                && adjacent(left, current)
            {
                self.chunks.remove(&left.offset);
                self.chunks.remove(&current.offset);
                let merged = container.merge_adjacent_chunks(left, current)?;
                container.write_chunk_prefix(merged, &free_prefix())?;
                offset = merged.offset;
                self.chunks
                    .insert(offset, StoredCompactionChunk::Free(merged));
                continue;
            }
            if let Some(right) = self.right_free(offset)
                && adjacent(current, right)
            {
                self.chunks.remove(&current.offset);
                self.chunks.remove(&right.offset);
                let merged = container.merge_adjacent_chunks(current, right)?;
                container.write_chunk_prefix(merged, &free_prefix())?;
                self.chunks
                    .insert(offset, StoredCompactionChunk::Free(merged));
                continue;
            }
            let tail = self
                .chunks
                .get(&offset)
                .map(StoredCompactionChunk::chunk)
                .unwrap();
            if container.truncate_tail_chunk(tail)? {
                self.chunks.remove(&offset);
            }
            return Ok(());
        }
    }

    fn left_free(&self, offset: u64) -> Option<ChunkRef> {
        self.chunks
            .range(..offset)
            .next_back()
            .and_then(|(_, chunk)| chunk.is_free().then(|| chunk.chunk()))
    }

    fn right_free(&self, offset: u64) -> Option<ChunkRef> {
        let next = offset.checked_add(1)?;
        self.chunks
            .range(next..)
            .next()
            .and_then(|(_, chunk)| chunk.is_free().then(|| chunk.chunk()))
    }
}

fn adjacent(left: ChunkRef, right: ChunkRef) -> bool {
    left.offset
        .checked_add(CHUNK_HEADER_LEN)
        .and_then(|value| value.checked_add(left.len))
        == Some(right.offset)
}

fn round_up(value: u64, granularity: u64) -> Result<u64, ConversationCompactionError> {
    value
        .checked_add(granularity - 1)
        .map(|value| value / granularity * granularity)
        .ok_or(ConversationCompactionError::RecordTooLarge)
}
