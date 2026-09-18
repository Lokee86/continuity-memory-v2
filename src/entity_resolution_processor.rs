use crate::{
    Cva, EntityCandidateConfig, EntityResolutionOutcome, EntityResolutionPreparation,
    EntityResolver, EntityResolverError, GeneralEndpoint, MemoryEntityMentionKey, Phylactery,
};

macro_rules! impl_owner {
    ($owner:ty) => {
        impl $owner {
            pub fn resolve_entity_mention<E: GeneralEndpoint>(
                &mut self,
                resolver: &EntityResolver<E>,
                key: MemoryEntityMentionKey,
                config: EntityCandidateConfig,
                now_ns: i64,
            ) -> Result<EntityResolutionOutcome, EntityResolverError> {
                match self.prepare_entity_resolution(key, config)? {
                    EntityResolutionPreparation::Complete(outcome) => Ok(outcome),
                    EntityResolutionPreparation::Ready(prepared) => {
                        let evaluation = resolver.evaluate_prepared(&prepared)?;
                        self.commit_entity_resolution(prepared, evaluation, now_ns)?
                            .ok_or_else(|| {
                                EntityResolverError::InvalidOutput(
                                    "Entity resolution snapshot became stale".into(),
                                )
                            })
                    }
                }
            }
        }
    };
}

impl_owner!(Cva);
impl_owner!(Phylactery);
