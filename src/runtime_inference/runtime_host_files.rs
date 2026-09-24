use super::{ReliquaryRuntimeHost, ReliquaryRuntimeHostError, operation};
use crate::FileId;

impl ReliquaryRuntimeHost {
    pub fn file_bytes(&self, id: FileId) -> Result<Vec<u8>, ReliquaryRuntimeHostError> {
        let owner_id = self.active_rel_id().ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation("Reliquary runtime has no active REL".into())
        })?;
        self.file_bytes_for(&owner_id, id)
    }

    pub fn file_bytes_for(
        &self,
        owner_id: &str,
        id: FileId,
    ) -> Result<Vec<u8>, ReliquaryRuntimeHostError> {
        self.with_runtime_for(owner_id, |runtime| {
            runtime.cva.file_bytes(id).map_err(operation)
        })
    }
}
