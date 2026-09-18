use crate::entity_candidate_model::{
    EntityCandidate, EntityCandidateConfig, EntityCandidateError, EntityCandidateSet,
    MAX_ENTITY_CANDIDATE_CONTEXT_TERMS, MAX_ENTITY_CANDIDATE_GRAPH_NEIGHBORS,
    MAX_ENTITY_CANDIDATE_LEXICAL_MEMORIES, MAX_ENTITY_CANDIDATE_SUPPORT_MEMORIES,
    MAX_ENTITY_CANDIDATE_SURFACE_MATCHES,
};
use crate::lexical_search::lexical_terms;
use crate::{
    Cva, EntityId, GraphDirection, GraphError, Memory, MemoryEntityMention, MemoryEntityMentionKey,
    MemoryId, MemoryTextField, Phylactery,
};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
struct CandidateEvidence {
    exact_surface: bool,
    lexical_memories: HashMap<MemoryId, f64>,
    graph_memories: HashSet<MemoryId>,
}

macro_rules! impl_owner {
    ($owner:ty) => {
        impl $owner {
            pub fn entity_candidates_for_mention(
                &mut self,
                key: MemoryEntityMentionKey,
                config: EntityCandidateConfig,
            ) -> Result<EntityCandidateSet, EntityCandidateError> {
                validate_config(config)?;
                let mention = mention_for_key(self.memory_routing_metadata(key.memory_id), key)?;
                let source = self.memory(key.memory_id)?;
                let context_query = context_query(&source, &mention);

                let mut evidence = HashMap::<EntityId, CandidateEvidence>::new();
                for entity in self.entity_candidates_for_surface(
                    &mention.text,
                    MAX_ENTITY_CANDIDATE_SURFACE_MATCHES,
                ) {
                    evidence.entry(entity.id).or_default().exact_surface = true;
                }

                let lexical_hits = if config.lexical_memory_limit == 0 || context_query.is_empty() {
                    Vec::new()
                } else {
                    let mut hits = self.memory_lexical_candidates(
                        &context_query,
                        config.lexical_memory_limit.saturating_add(1),
                    )?;
                    hits.retain(|hit| hit.memory_id != key.memory_id);
                    hits.truncate(config.lexical_memory_limit);
                    hits
                };
                for hit in &lexical_hits {
                    for entity_id in self.entity_associations_for_memory(hit.memory_id) {
                        evidence
                            .entry(entity_id)
                            .or_default()
                            .lexical_memories
                            .entry(hit.memory_id)
                            .and_modify(|score| *score = score.max(hit.score))
                            .or_insert(hit.score);
                    }
                }

                let graph_neighbors =
                    graph_neighbor_ids(self, key.memory_id, config.graph_neighbor_limit)?;
                for memory_id in &graph_neighbors {
                    for entity_id in self.entity_associations_for_memory(*memory_id) {
                        evidence
                            .entry(entity_id)
                            .or_default()
                            .graph_memories
                            .insert(*memory_id);
                    }
                }

                let mut candidates = evidence
                    .into_iter()
                    .map(|(entity_id, evidence)| {
                        Ok(finalize_candidate(
                            self.entity(entity_id)?,
                            evidence,
                            config.support_memory_limit,
                        ))
                    })
                    .collect::<Result<Vec<_>, EntityCandidateError>>()?;
                candidates.sort_by(candidate_order);
                candidates.truncate(config.max_candidates);

                Ok(EntityCandidateSet {
                    key,
                    mention,
                    candidates,
                    lexical_memories_examined: lexical_hits.len(),
                    graph_neighbors_examined: graph_neighbors.len(),
                })
            }
        }
    };
}

impl_owner!(Cva);
impl_owner!(Phylactery);

fn validate_config(config: EntityCandidateConfig) -> Result<(), EntityCandidateError> {
    if !(1..=crate::MAX_ENTITY_RESOLUTION_CANDIDATES).contains(&config.max_candidates)
        || config.lexical_memory_limit > MAX_ENTITY_CANDIDATE_LEXICAL_MEMORIES
        || config.graph_neighbor_limit > MAX_ENTITY_CANDIDATE_GRAPH_NEIGHBORS
        || config.support_memory_limit > MAX_ENTITY_CANDIDATE_SUPPORT_MEMORIES
    {
        return Err(EntityCandidateError::InvalidConfig);
    }
    Ok(())
}

fn mention_for_key(
    metadata: Option<&crate::MemoryRoutingMetadata>,
    key: MemoryEntityMentionKey,
) -> Result<MemoryEntityMention, EntityCandidateError> {
    metadata
        .and_then(|metadata| {
            metadata.entity_mentions.iter().find(|mention| {
                mention.field == key.field
                    && mention.start_byte == key.start_byte
                    && mention.end_byte == key.end_byte
            })
        })
        .cloned()
        .ok_or(EntityCandidateError::MissingMention(key))
}

trait CandidateOwner {
    fn graph_neighbors_for_candidate(
        &self,
        memory_id: MemoryId,
        direction: GraphDirection,
    ) -> Result<Vec<crate::GraphNeighbor>, GraphError>;
}

impl CandidateOwner for Cva {
    fn graph_neighbors_for_candidate(
        &self,
        memory_id: MemoryId,
        direction: GraphDirection,
    ) -> Result<Vec<crate::GraphNeighbor>, GraphError> {
        self.graph_neighbors(memory_id, direction)
    }
}

impl CandidateOwner for Phylactery {
    fn graph_neighbors_for_candidate(
        &self,
        memory_id: MemoryId,
        direction: GraphDirection,
    ) -> Result<Vec<crate::GraphNeighbor>, GraphError> {
        self.graph_neighbors(memory_id, direction)
    }
}

fn graph_neighbor_ids(
    owner: &impl CandidateOwner,
    memory_id: MemoryId,
    limit: usize,
) -> Result<Vec<MemoryId>, EntityCandidateError> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let mut ids = Vec::new();
    for direction in [GraphDirection::Outgoing, GraphDirection::Incoming] {
        match owner.graph_neighbors_for_candidate(memory_id, direction) {
            Ok(neighbors) => ids.extend(neighbors.into_iter().map(|neighbor| neighbor.memory_id)),
            Err(GraphError::MissingNode(id)) if id == memory_id => {}
            Err(error) => return Err(error.into()),
        }
    }
    ids.sort_by_key(|id| id.0);
    ids.dedup();
    ids.truncate(limit);
    Ok(ids)
}

fn finalize_candidate(
    entity: crate::Entity,
    evidence: CandidateEvidence,
    support_limit: usize,
) -> EntityCandidate {
    let mut lexical: Vec<_> = evidence.lexical_memories.into_iter().collect();
    lexical.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.0.cmp(&right.0.0))
    });
    let best_lexical_score = lexical.first().map(|(_, score)| *score).unwrap_or(0.0);
    let lexical_memory_hits = lexical.len();
    let mut graph: Vec<_> = evidence.graph_memories.into_iter().collect();
    graph.sort_by_key(|id| id.0);
    let graph_neighbor_hits = graph.len();

    let mut supporting_memory_ids = Vec::new();
    for (memory_id, _) in &lexical {
        push_unique(&mut supporting_memory_ids, *memory_id, support_limit);
    }
    for memory_id in graph {
        push_unique(&mut supporting_memory_ids, memory_id, support_limit);
    }

    EntityCandidate {
        entity,
        exact_surface: evidence.exact_surface,
        lexical_memory_hits,
        graph_neighbor_hits,
        best_lexical_score,
        supporting_memory_ids,
    }
}

fn push_unique(values: &mut Vec<MemoryId>, value: MemoryId, limit: usize) {
    if values.len() < limit && !values.contains(&value) {
        values.push(value);
    }
}

fn candidate_order(left: &EntityCandidate, right: &EntityCandidate) -> std::cmp::Ordering {
    let left_lanes =
        usize::from(left.lexical_memory_hits > 0) + usize::from(left.graph_neighbor_hits > 0);
    let right_lanes =
        usize::from(right.lexical_memory_hits > 0) + usize::from(right.graph_neighbor_hits > 0);
    right
        .exact_surface
        .cmp(&left.exact_surface)
        .then_with(|| right_lanes.cmp(&left_lanes))
        .then_with(|| right.best_lexical_score.total_cmp(&left.best_lexical_score))
        .then_with(|| right.lexical_memory_hits.cmp(&left.lexical_memory_hits))
        .then_with(|| right.graph_neighbor_hits.cmp(&left.graph_neighbor_hits))
        .then_with(|| left.entity.id.cmp(&right.entity.id))
}

fn context_query(memory: &Memory, mention: &MemoryEntityMention) -> String {
    let field = match mention.field {
        MemoryTextField::Title => memory.title.as_str(),
        MemoryTextField::Content => memory.content.as_str(),
    };
    let start = usize::try_from(mention.start_byte).unwrap_or(usize::MAX);
    let end = usize::try_from(mention.end_byte).unwrap_or(usize::MAX);
    let local = local_context(field, start, end);

    let mut terms = Vec::new();
    let mut seen = HashSet::new();
    append_terms(&mention.text, &mut terms, &mut seen);
    if mention.field == MemoryTextField::Content {
        append_terms(&memory.title, &mut terms, &mut seen);
    }
    append_terms(&local, &mut terms, &mut seen);
    terms.truncate(MAX_ENTITY_CANDIDATE_CONTEXT_TERMS);
    terms.join(" ")
}

fn append_terms(value: &str, terms: &mut Vec<String>, seen: &mut HashSet<String>) {
    for term in lexical_terms(value) {
        if seen.insert(term.clone()) {
            terms.push(term);
        }
    }
}

fn local_context(value: &str, start: usize, end: usize) -> String {
    let (Some(before), Some(after)) = (value.get(..start), value.get(end..)) else {
        return String::new();
    };
    let mut prefix: Vec<_> = before.chars().rev().take(96).collect();
    prefix.reverse();
    let suffix: String = after.chars().take(96).collect();
    format!("{} {}", prefix.into_iter().collect::<String>(), suffix)
}
