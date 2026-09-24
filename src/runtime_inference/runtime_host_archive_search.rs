use super::{ReliquaryRuntimeHost, ReliquaryRuntimeHostError, operation};
use crate::ArchiveSearchHit;

impl ReliquaryRuntimeHost {
    pub fn search_archive(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ArchiveSearchHit>, ReliquaryRuntimeHostError> {
        let runtime = self.execution.runtime.as_ref().cloned().ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation("Reliquary runtime is unavailable".into())
        })?;
        runtime
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?
            .cva
            .search_archive(query, limit)
            .map_err(operation)
    }
}
