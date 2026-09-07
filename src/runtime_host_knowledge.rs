use super::{ReliquaryRuntimeHost, ReliquaryRuntimeHostError, operation};
use crate::{CommunitySnapshot, GraphRelation, Memory};

impl ReliquaryRuntimeHost {
    pub fn read_reliquary_knowledge(
        &self,
    ) -> Result<
        (Vec<Memory>, Vec<GraphRelation>, Option<CommunitySnapshot>),
        ReliquaryRuntimeHostError,
    > {
        let runtime = self.runtime.as_ref().cloned().ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation("Reliquary runtime is unavailable".into())
        })?;
        let mut runtime = runtime
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        let memories = runtime
            .cva
            .memory_ids()
            .into_iter()
            .map(|id| runtime.cva.memory(id).map_err(operation))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((
            memories,
            runtime.cva.graph_relations(),
            runtime.cva.community_snapshot(),
        ))
    }

    pub fn read_phylactery_knowledge(
        &self,
    ) -> Result<
        Option<(Vec<Memory>, Vec<GraphRelation>, Option<CommunitySnapshot>)>,
        ReliquaryRuntimeHostError,
    > {
        let mut slot = self
            .phylactery
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        let Some(phylactery) = slot.as_mut() else {
            return Ok(None);
        };
        let memories = phylactery
            .memory_ids()
            .into_iter()
            .map(|id| phylactery.memory(id).map_err(operation))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some((
            memories,
            phylactery.graph_relations(),
            phylactery.community_snapshot(),
        )))
    }
}
