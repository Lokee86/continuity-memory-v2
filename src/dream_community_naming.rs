use crate::dream_community_naming_context::{build_payload, parse_name, representative_ids};
use crate::dream_community_naming_schema::{
    DREAM_COMMUNITY_NAMING_SYSTEM_PROMPT, dream_community_naming_schema,
};
use crate::{
    COMMUNITY_ALGORITHM_VERSION, COMMUNITY_NAMING_CONTRACT_VERSION, CommunitySemanticName,
    CommunitySemanticNameSource, CompatibilityProfileId, Cva,
    DEFAULT_MEMORY_RETRIEVAL_SUBCENTROIDS, DreamCommunityNamingError, GeneralEndpoint, Memory,
    Phylactery,
};

const DREAM_COMMUNITY_NAMING_RETRY_LIMIT: usize = 2;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DreamCommunityNamingResult {
    pub attempted: usize,
    pub named: Vec<CommunitySemanticName>,
    pub remaining: usize,
}

pub struct DreamCommunityNamer<E> {
    endpoint: E,
    system_prompt: String,
}

impl<E: GeneralEndpoint> DreamCommunityNamer<E> {
    pub fn new(endpoint: E) -> Self {
        Self {
            endpoint,
            system_prompt: DREAM_COMMUNITY_NAMING_SYSTEM_PROMPT.to_owned(),
        }
    }

    pub fn with_system_prompt(endpoint: E, system_prompt: impl Into<String>) -> Self {
        Self {
            endpoint,
            system_prompt: system_prompt.into(),
        }
    }

    pub fn model(&self) -> &str {
        self.endpoint.model()
    }

    pub fn name_communities(
        &self,
        cva: &mut Cva,
        compatibility_profile_id: CompatibilityProfileId,
        limit: usize,
    ) -> Result<DreamCommunityNamingResult, DreamCommunityNamingError> {
        if limit == 0 {
            return Err(DreamCommunityNamingError::InvalidConfig("limit"));
        }
        let snapshot = current_snapshot(cva.community_snapshot(), cva.memory_graph_version())?;
        let index = cva.build_memory_retrieval_index(
            compatibility_profile_id,
            DEFAULT_MEMORY_RETRIEVAL_SUBCENTROIDS,
        )?;
        let pending: Vec<_> = snapshot
            .communities
            .iter()
            .filter(|community| needs_name(cva.community_semantic_name(community.id).as_ref()))
            .take(limit)
            .cloned()
            .collect();
        let mut named = Vec::with_capacity(pending.len());
        for community in &pending {
            let representatives = representative_ids(community, &index);
            let memories = representatives
                .iter()
                .map(|memory_id| cva.memory(*memory_id))
                .collect::<Result<Vec<_>, _>>()?;
            let record = CommunitySemanticName {
                community_id: community.id,
                baseline_community_id: community.id,
                contract_version: COMMUNITY_NAMING_CONTRACT_VERSION,
                source: CommunitySemanticNameSource::Dream,
                name: self.generate_name(&memories)?,
                representative_memories: representatives,
            };
            cva.publish_community_semantic_name(record.clone())?;
            named.push(record);
        }
        let remaining = snapshot
            .communities
            .iter()
            .filter(|community| needs_name(cva.community_semantic_name(community.id).as_ref()))
            .count();
        Ok(DreamCommunityNamingResult {
            attempted: pending.len(),
            named,
            remaining,
        })
    }

    pub fn name_phylactery_communities(
        &self,
        phylactery: &mut Phylactery,
        compatibility_profile_id: CompatibilityProfileId,
        limit: usize,
    ) -> Result<DreamCommunityNamingResult, DreamCommunityNamingError> {
        if limit == 0 {
            return Err(DreamCommunityNamingError::InvalidConfig("limit"));
        }
        let snapshot = current_snapshot(
            phylactery.community_snapshot(),
            phylactery.memory_graph_version(),
        )?;
        let index = phylactery.build_memory_retrieval_index(
            compatibility_profile_id,
            DEFAULT_MEMORY_RETRIEVAL_SUBCENTROIDS,
        )?;
        let pending: Vec<_> = snapshot
            .communities
            .iter()
            .filter(|community| {
                needs_name(phylactery.community_semantic_name(community.id).as_ref())
            })
            .take(limit)
            .cloned()
            .collect();
        let mut named = Vec::with_capacity(pending.len());
        for community in &pending {
            let representatives = representative_ids(community, &index);
            let memories = representatives
                .iter()
                .map(|memory_id| phylactery.memory(*memory_id))
                .collect::<Result<Vec<_>, _>>()?;
            let record = CommunitySemanticName {
                community_id: community.id,
                baseline_community_id: community.id,
                contract_version: COMMUNITY_NAMING_CONTRACT_VERSION,
                source: CommunitySemanticNameSource::Dream,
                name: self.generate_name(&memories)?,
                representative_memories: representatives,
            };
            phylactery.publish_community_semantic_name(record.clone())?;
            named.push(record);
        }
        let remaining = snapshot
            .communities
            .iter()
            .filter(|community| {
                needs_name(phylactery.community_semantic_name(community.id).as_ref())
            })
            .count();
        Ok(DreamCommunityNamingResult {
            attempted: pending.len(),
            named,
            remaining,
        })
    }

    fn generate_name(&self, memories: &[Memory]) -> Result<String, DreamCommunityNamingError> {
        let payload = build_payload(memories)?;
        for attempt in 0..=DREAM_COMMUNITY_NAMING_RETRY_LIMIT {
            match self.endpoint.complete_json(
                &self.system_prompt,
                &payload,
                "dream_community_name",
                &dream_community_naming_schema(),
            ) {
                Ok(output) => match parse_name(&output) {
                    Ok(name) => return Ok(name),
                    Err(error) if attempt == DREAM_COMMUNITY_NAMING_RETRY_LIMIT => {
                        return Err(error);
                    }
                    Err(_) => continue,
                },
                Err(error) if error.is_backpressure() => {
                    return Err(DreamCommunityNamingError::Inference(error));
                }
                Err(error) if attempt == DREAM_COMMUNITY_NAMING_RETRY_LIMIT => {
                    return Err(DreamCommunityNamingError::Inference(error));
                }
                Err(_) => continue,
            }
        }
        unreachable!("Dream community naming retry loop always returns")
    }
}

fn current_snapshot(
    snapshot: Option<crate::CommunitySnapshot>,
    graph_version: u64,
) -> Result<crate::CommunitySnapshot, DreamCommunityNamingError> {
    snapshot
        .filter(|snapshot| {
            snapshot.derived_graph_version == graph_version
                && snapshot.algorithm_version == COMMUNITY_ALGORITHM_VERSION
        })
        .ok_or(DreamCommunityNamingError::MissingCurrentSnapshot)
}

fn needs_name(name: Option<&CommunitySemanticName>) -> bool {
    match name {
        None => true,
        Some(name) if name.source == CommunitySemanticNameSource::User => false,
        Some(name) => name.contract_version < COMMUNITY_NAMING_CONTRACT_VERSION,
    }
}
