use crate::dream_candidate_ranking::{
    lexical_score, rank_lanes, select_candidates, ScoredCandidate,
};
use crate::dream_source_time::source_timestamp_ns;
use crate::dream_temporal::analyze_memory_temporal;
use crate::dream_temporal_match::temporal_matches;
use crate::{
    CompatibilityProfileId, Cva, DreamCandidateConfig, DreamCandidateError, DreamCandidateSet,
    DreamMemoryContext, GraphRelation, Memory, MemoryBodyId, MemoryId, MemoryVectorId, ScalarType,
    MAX_DREAM_CANDIDATE_LIMIT,
};
use std::collections::HashMap;

impl Cva {
    pub fn dream_memory_context(
        &mut self,
        memory_id: MemoryId,
    ) -> Result<DreamMemoryContext, DreamCandidateError> {
        let memory = self.memories.memory(&mut self.container, memory_id)?;
        let body_id = self.memories.current_body_id(memory_id)?;
        let relations = self.graph.active_relations();
        Ok(self.build_dream_memory_context(memory, body_id, &relations))
    }

    pub fn dream_candidates(
        &mut self,
        compatibility_profile_id: CompatibilityProfileId,
        source_id: MemoryId,
        config: DreamCandidateConfig,
    ) -> Result<DreamCandidateSet, DreamCandidateError> {
        validate_config(config)?;
        let source = self.memories.memory(&mut self.container, source_id)?;
        let source_body = self.memories.current_body_id(source_id)?;
        let vectors = self.load_memory_vectors(compatibility_profile_id)?;
        let source_vector = vectors
            .get(&source_body)
            .ok_or(DreamCandidateError::MissingSourceVector)?;
        let relations = self.graph.active_relations();
        let source_context = self.build_dream_memory_context(source, source_body, &relations);

        let mut scored = Vec::new();
        for id in self.memories.current_ids() {
            if id == source_id {
                continue;
            }
            let memory = self.memories.memory(&mut self.container, id)?;
            if memory.archived {
                continue;
            }
            let body_id = self.memories.current_body_id(id)?;
            let semantic_score = vectors
                .get(&body_id)
                .map(|candidate| cosine(source_vector, candidate))
                .transpose()?;
            let lexical_score = lexical_score(&source_context.memory, &memory);
            let context = self.build_dream_memory_context(memory, body_id, &relations);
            let (temporal_matches, temporal_score) =
                temporal_matches(&source_context.temporal, &context.temporal);
            scored.push(ScoredCandidate {
                context,
                semantic_score,
                lexical_score,
                semantic_rank: None,
                prior_rank: None,
                lexical_rank: None,
                temporal_score,
                temporal_rank: None,
                temporal_matches,
                fused_score: 0.0,
            });
        }
        rank_lanes(&source_context, &mut scored, config);
        Ok(DreamCandidateSet {
            source: source_context,
            candidates: select_candidates(scored, config),
        })
    }

    fn build_dream_memory_context(
        &self,
        memory: Memory,
        body_id: MemoryBodyId,
        relations: &[GraphRelation],
    ) -> DreamMemoryContext {
        let source_timestamp_ns = source_timestamp_ns(self, &memory);
        let graph_relations = relations
            .iter()
            .copied()
            .filter(|relation| relation.source == memory.id || relation.target == memory.id)
            .collect();
        let temporal = analyze_memory_temporal(&memory, source_timestamp_ns);
        DreamMemoryContext {
            memory,
            body_id,
            source_timestamp_ns,
            graph_relations,
            temporal,
        }
    }

    fn load_memory_vectors(
        &mut self,
        profile_id: CompatibilityProfileId,
    ) -> Result<HashMap<MemoryBodyId, Vec<f32>>, DreamCandidateError> {
        let mut by_set: HashMap<MemoryVectorId, Vec<(MemoryBodyId, u64)>> = HashMap::new();
        for body_id in self.memories.current_body_ids() {
            if let Some(location) = self.memory_vectors.location(profile_id, body_id) {
                by_set
                    .entry(location.set_id)
                    .or_default()
                    .push((body_id, location.row));
            }
        }
        let mut output = HashMap::new();
        for (set_id, locations) in by_set {
            let set = self.memory_vectors.get(&mut self.container, set_id)?;
            let packed = self
                .packed_vectors
                .get(&mut self.container, set.packed_vector_id)?;
            if packed.schema().scalar != ScalarType::F32 {
                return Err(DreamCandidateError::UnsupportedScalar(
                    packed.schema().scalar,
                ));
            }
            for (body_id, row) in locations {
                let ordinal = usize::try_from(row)
                    .map_err(|_| DreamCandidateError::CorruptVector("row ordinal"))?;
                let bytes = packed
                    .row(ordinal)
                    .ok_or(DreamCandidateError::CorruptVector("missing row"))?;
                output.insert(body_id, decode_f32_row(bytes)?);
            }
        }
        Ok(output)
    }
}

fn validate_config(config: DreamCandidateConfig) -> Result<(), DreamCandidateError> {
    if config.limit == 0
        || config.limit > MAX_DREAM_CANDIDATE_LIMIT
        || config.semantic_limit > MAX_DREAM_CANDIDATE_LIMIT
        || config.lexical_limit > MAX_DREAM_CANDIDATE_LIMIT
        || config.temporal_limit > MAX_DREAM_CANDIDATE_LIMIT
        || config.prior_semantic_quota > config.limit
    {
        return Err(DreamCandidateError::InvalidConfig);
    }
    Ok(())
}

fn decode_f32_row(bytes: &[u8]) -> Result<Vec<f32>, DreamCandidateError> {
    if !bytes.len().is_multiple_of(4) {
        return Err(DreamCandidateError::CorruptVector("f32 row width"));
    }
    bytes
        .chunks_exact(4)
        .map(|chunk| {
            let value = f32::from_le_bytes(chunk.try_into().expect("f32 width"));
            value
                .is_finite()
                .then_some(value)
                .ok_or(DreamCandidateError::CorruptVector("non-finite vector"))
        })
        .collect()
}

fn cosine(left: &[f32], right: &[f32]) -> Result<f64, DreamCandidateError> {
    if left.len() != right.len() || left.is_empty() {
        return Err(DreamCandidateError::CorruptVector("vector dimensions"));
    }
    let mut dot = 0.0_f64;
    let mut left_norm = 0.0_f64;
    let mut right_norm = 0.0_f64;
    for (left, right) in left.iter().zip(right) {
        dot += *left as f64 * *right as f64;
        left_norm += (*left as f64).powi(2);
        right_norm += (*right as f64).powi(2);
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        return Err(DreamCandidateError::CorruptVector("zero vector"));
    }
    Ok(dot / (left_norm.sqrt() * right_norm.sqrt()))
}
