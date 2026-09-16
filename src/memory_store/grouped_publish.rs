use super::*;
use crate::insomnia::completion::InsomniaCompletionBody;
use std::collections::{HashMap, HashSet};

pub(crate) struct PreparedMemoryBatch {
    pub(crate) records: Vec<MemoryRecord>,
    pub(crate) existing: Vec<Memory>,
    pub(crate) bodies: Vec<InsomniaCompletionBody>,
    pub(crate) routing_metadata: Vec<MemoryRoutingMetadata>,
    pub(crate) global_version_start: u64,
}

impl MemoryStore {
    pub(crate) fn stage_grouped_insomnia(
        &mut self,
        container: &mut Container,
        drafts: Vec<(
            MemoryDraft,
            Option<MemoryTemporalInference>,
            Option<MemoryRoutingMetadata>,
        )>,
    ) -> Result<PreparedMemoryBatch, MemoryError> {
        let global_version_start = container.next_version_candidate();
        let mut records = Vec::new();
        let mut existing = Vec::new();
        let mut bodies: Vec<InsomniaCompletionBody> = Vec::new();
        let mut routing_metadata = Vec::new();
        let mut body_indexes: HashMap<MemoryBodyId, usize> = HashMap::new();
        let mut seen_mutations = HashSet::new();
        for (draft, temporal_inference, routing) in drafts {
            validate_draft(&draft)?;
            if !seen_mutations.insert(draft.mutation_id.clone()) {
                return Err(MemoryError::MutationConflict);
            }
            if let Some(index) = self.by_mutation.get(&draft.mutation_id).copied() {
                let memory = self.resolve_record(container, &self.records[index])?;
                if !same_draft(&memory, &draft) {
                    return Err(MemoryError::MutationConflict);
                }
                if let Some(routing) = routing.as_ref()
                    && memory.routing_metadata.as_ref() != Some(routing)
                {
                    return Err(MemoryError::RoutingMetadataConflict);
                }
                existing.push(memory);
                continue;
            }
            let id = memory_id(&draft.mutation_id);
            if self.current.contains_key(&id) {
                return Err(MemoryError::RevisionConflict);
            }
            let body_id = memory_body_id(&draft.title, &draft.content);
            if let Some(routing) = routing.as_ref() {
                validate_prepared_routing(&draft, id, body_id, routing)?;
            }
            if let Some(inference) = &temporal_inference {
                validate_temporal_inference_binding(body_id, draft.source_time_ns, inference)?;
            }
            let body_bytes = memory_body_bytes(&draft.title, &draft.content);
            if self.bodies.contains_key(&body_id) {
                if self.body_bytes(container, body_id)? != body_bytes {
                    return Err(MemoryError::HashCollision);
                }
            } else if let Some(index) = body_indexes.get(&body_id).copied() {
                if bodies[index].bytes != body_bytes {
                    return Err(MemoryError::HashCollision);
                }
            } else {
                body_indexes.insert(body_id, bodies.len());
                bodies.push(InsomniaCompletionBody {
                    id: body_id,
                    bytes: body_bytes,
                });
            }
            let offset = u64::try_from(records.len()).map_err(|_| MemoryError::VersionExhausted)?;
            let memory_version = self
                .next_memory_version
                .checked_add(offset)
                .ok_or(MemoryError::VersionExhausted)?;
            let global_version = global_version_start
                .checked_add(offset)
                .ok_or(MemoryError::VersionExhausted)?;
            records.push(MemoryRecord {
                id,
                revision: 1,
                body_id,
                category: draft.category,
                memory_type: draft.memory_type,
                authority_kind: draft.authority_kind,
                temporal_status: draft.temporal_status,
                scope: draft.scope,
                lifecycle_state: draft.lifecycle_state,
                archived: draft.archived,
                superseded_by: draft.superseded_by,
                parent_id: draft.parent_id,
                source_node_id: draft.source_node_id,
                content_source_conversation_id: draft.content_source_conversation_id,
                content_source_node_id: draft.content_source_node_id,
                grounding_source_conversation_id: draft.grounding_source_conversation_id,
                grounding_source_node_id: draft.grounding_source_node_id,
                source_episode_id: draft.source_episode_id,
                source_time_ns: draft.source_time_ns,
                source_ref: None,
                temporal_inference,
                mutation_id: draft.mutation_id,
                created_at_ns: draft.created_at_ns,
                updated_at_ns: draft.updated_at_ns,
                global_version,
                memory_version,
            });
            if let Some(routing) = routing {
                routing_metadata.push(routing);
            }
        }
        Ok(PreparedMemoryBatch {
            records,
            existing,
            bodies,
            routing_metadata,
            global_version_start,
        })
    }

    pub(crate) fn apply_grouped_bodies(
        &mut self,
        chunk: ObjectRef,
        bodies: &[InsomniaCompletionBody],
    ) -> Result<(), MemoryError> {
        for body in bodies {
            let (title, content) = decode_memory_body(&body.bytes)?;
            if memory_body_id(&title, &content) != body.id {
                return Err(MemoryError::CorruptBody);
            }
            self.insert_body(body.id, chunk);
        }
        Ok(())
    }

    pub(crate) fn apply_grouped_records(
        &mut self,
        container: &mut Container,
        records: &[MemoryRecord],
    ) -> Result<Vec<Memory>, MemoryError> {
        let mut created = Vec::with_capacity(records.len());
        for record in records {
            if !self.bodies.contains_key(&record.body_id) {
                return Err(MemoryError::MissingBody);
            }
            self.insert_rebuilt(record.clone())?;
            created.push(self.resolve_record(container, record)?);
        }
        Ok(created)
    }

    pub(crate) fn apply_grouped_routing_metadata(
        &mut self,
        metadata: &[MemoryRoutingMetadata],
    ) -> Result<(), MemoryError> {
        for value in metadata {
            self.insert_routing_metadata_rebuilt(value.clone())?;
        }
        Ok(())
    }
}

fn validate_prepared_routing(
    draft: &MemoryDraft,
    memory_id: MemoryId,
    body_id: MemoryBodyId,
    metadata: &MemoryRoutingMetadata,
) -> Result<(), MemoryError> {
    if metadata.memory_id != memory_id || metadata.body_id != body_id {
        return Err(MemoryError::InvalidField("Memory routing metadata binding"));
    }
    if metadata.entity_mentions.len() > MAX_MEMORY_ENTITY_MENTIONS
        || metadata.lexical_terms.len() > MAX_MEMORY_LEXICAL_TERMS
    {
        return Err(MemoryError::InvalidField("Memory routing metadata count"));
    }
    for mention in &metadata.entity_mentions {
        if mention.text.is_empty()
            || mention.text.len() > MAX_MEMORY_ROUTING_TEXT_BYTES
            || mention.start_byte >= mention.end_byte
        {
            return Err(MemoryError::InvalidField("Memory entity mention"));
        }
        let source = match mention.field {
            MemoryTextField::Title => draft.title.as_str(),
            MemoryTextField::Content => draft.content.as_str(),
        };
        let start = mention.start_byte as usize;
        let end = mention.end_byte as usize;
        if !source.is_char_boundary(start)
            || !source.is_char_boundary(end)
            || source.get(start..end) != Some(mention.text.as_str())
        {
            return Err(MemoryError::InvalidField("Memory entity mention span"));
        }
    }
    for term in &metadata.lexical_terms {
        if term.trim().is_empty()
            || term.len() > MAX_MEMORY_ROUTING_TEXT_BYTES
            || (!draft.title.contains(term) && !draft.content.contains(term))
        {
            return Err(MemoryError::InvalidField("Memory lexical term grounding"));
        }
    }
    Ok(())
}
