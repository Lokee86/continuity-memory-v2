use crate::community_store::{CommunityOpenState, CommunityStore};
use crate::compatibility_profile_rebuild::CompatibilityProfileOpenState;
use crate::compatibility_profile_store::CompatibilityProfileStore;
use crate::dream_cooldown::{DreamCooldownStore, DreamPairStore};
use crate::dream_duplicate_index::DuplicateIndex;
use crate::graph_rebuild::GraphOpenState;
use crate::graph_store::GraphStore;
use crate::memory_rebuild::MemoryOpenState;
use crate::memory_store::MemoryStore;
use crate::memory_vector_rebuild::MemoryVectorOpenState;
use crate::memory_vector_store::MemoryVectorStore;
use crate::packed_vector_rebuild::PackedVectorOpenState;
use crate::packed_vector_store::PackedVectorStore;
use crate::{Container, ContainerIdentity, FileKind, Phylactery, PhylacteryError};
use std::path::Path;

impl Phylactery {
    pub fn create(path: impl AsRef<Path>) -> Result<Self, PhylacteryError> {
        let container = Container::create_with_identity(
            path,
            ContainerIdentity {
                file_kind: FileKind::Phylactery,
                scope: None,
            },
        )?;
        Self::initialize(container)
    }

    #[cfg(test)]
    pub(crate) fn create_legacy_typed(path: impl AsRef<Path>) -> Result<Self, PhylacteryError> {
        let container = Container::create_with_legacy_identity(
            path,
            ContainerIdentity {
                file_kind: FileKind::Phylactery,
                scope: None,
            },
        )?;
        Self::initialize(container)
    }

    pub(crate) fn create_with_uuid(
        path: impl AsRef<Path>,
        owner_uuid: [u8; 16],
    ) -> Result<Self, PhylacteryError> {
        let container = Container::create_with_identity_and_uuid(
            path,
            ContainerIdentity {
                file_kind: FileKind::Phylactery,
                scope: None,
            },
            owner_uuid,
        )?;
        Self::initialize(container)
    }

    fn initialize(mut container: Container) -> Result<Self, PhylacteryError> {
        let memories = MemoryStore::empty();
        let mut graph = GraphStore::empty();
        let communities = CommunityStore::default();
        let dream_cooldowns = DreamCooldownStore::default();
        let dream_pairs = DreamPairStore::default();
        let packed_vectors = PackedVectorStore::default();
        let memory_vectors = MemoryVectorStore::default();
        let compatibility_profiles = CompatibilityProfileStore::default();

        memories.initialize(&mut container)?;
        graph.initialize(&mut container)?;
        packed_vectors.initialize(&mut container)?;
        memory_vectors.initialize(&mut container)?;
        compatibility_profiles.initialize(&mut container)?;
        container.sync()?;

        Ok(Self {
            container,
            memories,
            graph,
            communities,
            duplicate_index: DuplicateIndex::empty(),
            dream_cooldowns,
            dream_pairs,
            packed_vectors,
            memory_vectors,
            compatibility_profiles,
        })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, PhylacteryError> {
        let mut memory_state = MemoryOpenState::new();
        let mut graph_state = GraphOpenState::new();
        let mut community_state = CommunityOpenState::new();
        let mut packed_state = PackedVectorOpenState::new();
        let mut memory_vector_state = MemoryVectorOpenState::new();
        let mut profile_state = CompatibilityProfileOpenState::new();
        let mut dream_cooldowns = DreamCooldownStore::default();
        let mut dream_pairs = DreamPairStore::default();

        let container = Container::open_scanned(path, |chunk, payload, latest_global| {
            reject_rel_only_payload(payload)?;
            memory_state.ingest(chunk, payload, latest_global)?;
            graph_state.ingest(chunk, payload, latest_global)?;
            community_state.ingest(payload)?;
            packed_state.ingest(chunk, payload)?;
            memory_vector_state.ingest(chunk, payload)?;
            profile_state.ingest(chunk, payload)?;
            dream_cooldowns.ingest(payload)?;
            dream_pairs.ingest(payload)?;
            Ok::<(), PhylacteryError>(())
        })?;
        if container.identity()
            != Some(ContainerIdentity {
                file_kind: FileKind::Phylactery,
                scope: None,
            })
        {
            return Err(PhylacteryError::InvalidContainerIdentity(
                "file is not a Phylactery",
            ));
        }

        let memories = memory_state.finish()?;
        validate_phylactery_provenance(&memories)?;
        dream_cooldowns.validate(&memories)?;
        dream_pairs.validate(&memories)?;
        let graph = graph_state.finish(&memories)?;
        let communities = community_state.finish(&graph, container.owner_uuid())?;
        validate_global_versions(&memories, &graph)?;
        let packed_vectors = packed_state.finish()?;
        let compatibility_profiles = profile_state.finish()?;
        let memory_vectors =
            memory_vector_state.finish(&memories, &compatibility_profiles, &packed_vectors)?;

        Ok(Self {
            container,
            memories,
            graph,
            communities,
            duplicate_index: DuplicateIndex::empty(),
            dream_cooldowns,
            dream_pairs,
            packed_vectors,
            memory_vectors,
            compatibility_profiles,
        })
    }
}

fn reject_rel_only_payload(payload: &[u8]) -> Result<(), PhylacteryError> {
    const REL_ONLY_PREFIXES: [&[u8; 8]; 23] = [
        b"CVAAFMT2",
        b"CVACONT1",
        b"CVANODE1",
        b"CVABRCH1",
        b"CVAFRAG1",
        b"CVAEPIS1",
        b"CVAFILE1",
        b"CVATURN1",
        b"CVAFMEM1",
        b"CVAAREC1",
        b"CVAINSF1",
        b"CVAINSW1",
        b"CVAINSC2",
        b"CVAINSC1",
        b"CVAINSA1",
        b"CVAAVFM1",
        b"CVAAVEC1",
        b"CVAVGFM2",
        b"CVAVGEN2",
        b"CVAVGRC2",
        b"CVAWKFM1",
        b"CVAWKSP1",
        b"CVAISTR1",
    ];
    if payload.len() >= 8
        && REL_ONLY_PREFIXES
            .iter()
            .any(|prefix| payload[..8] == prefix[..])
    {
        return Err(PhylacteryError::InvalidContainerIdentity(
            "Phylactery contains a Reliquary-only owner record",
        ));
    }
    Ok(())
}

fn validate_phylactery_provenance(memories: &MemoryStore) -> Result<(), PhylacteryError> {
    if memories.records().iter().any(|record| {
        record.source_episode_id.is_some()
            || record.source_node_id.is_some()
            || record.content_source_conversation_id.is_some()
            || record.content_source_node_id.is_some()
            || record.grounding_source_conversation_id.is_some()
            || record.grounding_source_node_id.is_some()
    }) {
        return Err(crate::MemoryError::InvalidProvenance.into());
    }
    Ok(())
}

fn validate_global_versions(
    memories: &MemoryStore,
    graph: &GraphStore,
) -> Result<(), PhylacteryError> {
    let mut versions: Vec<_> = memories
        .records()
        .iter()
        .map(|record| record.global_version)
        .chain(graph.transaction_global_versions().iter().copied())
        .collect();
    versions.sort_unstable();
    if let Some(version) = versions
        .windows(2)
        .find(|pair| pair[0] == pair[1])
        .map(|pair| pair[0])
    {
        return Err(PhylacteryError::SemanticGlobalVersionConflict(version));
    }
    Ok(())
}
