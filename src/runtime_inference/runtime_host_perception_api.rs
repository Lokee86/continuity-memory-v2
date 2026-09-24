use super::{ReliquaryRuntimeHost, ReliquaryRuntimeHostError};

impl ReliquaryRuntimeHost {
    pub fn entity_stats(&self) -> Result<crate::EntityStats, ReliquaryRuntimeHostError> {
        let runtime = self
            .active_execution()?
            .runtime
            .as_ref()
            .ok_or_else(|| {
                ReliquaryRuntimeHostError::Operation("Reliquary runtime is unavailable".into())
            })?
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        Ok(runtime.cva.entity_stats())
    }

    pub fn phylactery_entity_stats(
        &self,
    ) -> Result<Option<crate::EntityStats>, ReliquaryRuntimeHostError> {
        let slot = self
            .active_execution()?
            .phylactery
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        Ok(slot.as_ref().map(crate::Phylactery::entity_stats))
    }
}
