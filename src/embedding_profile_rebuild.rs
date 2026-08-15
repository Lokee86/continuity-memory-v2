use crate::embedding_profile_codec::{decode_format, decode_profile};
use crate::embedding_profile_store::EmbeddingProfileStore;
use crate::{ChunkRef, EmbeddingProfileError};

pub(crate) struct EmbeddingProfileOpenState {
    store: EmbeddingProfileStore,
    format_seen: bool,
}

impl EmbeddingProfileOpenState {
    pub(crate) fn new() -> Self {
        Self {
            store: EmbeddingProfileStore::default(),
            format_seen: false,
        }
    }

    pub(crate) fn ingest(
        &mut self,
        _chunk: ChunkRef,
        payload: &[u8],
    ) -> Result<(), EmbeddingProfileError> {
        if decode_format(payload)? {
            if self.format_seen {
                return Err(EmbeddingProfileError::ConflictingFormat);
            }
            self.format_seen = true;
            return Ok(());
        }
        if let Some(profile) = decode_profile(payload)? {
            self.store.insert_rebuilt(profile)?;
        }
        Ok(())
    }

    pub(crate) fn finish(self) -> Result<EmbeddingProfileStore, EmbeddingProfileError> {
        if !self.format_seen {
            return Err(EmbeddingProfileError::MissingFormat);
        }
        Ok(self.store)
    }
}
