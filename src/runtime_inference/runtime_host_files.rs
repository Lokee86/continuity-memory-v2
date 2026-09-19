use super::{ReliquaryRuntimeHost, ReliquaryRuntimeHostError, operation};
use crate::FileId;

impl ReliquaryRuntimeHost {
    pub fn file_bytes(&self, id: FileId) -> Result<Vec<u8>, ReliquaryRuntimeHostError> {
        let runtime = self.runtime.as_ref().ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation("Reliquary runtime is unavailable".into())
        })?;
        let mut runtime = runtime
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        runtime.cva.file_bytes(id).map_err(operation)
    }
}
