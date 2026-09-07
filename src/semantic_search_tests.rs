use crate::{
    Cva, EmbeddingEndpoint, EmbeddingEndpointError, EmbeddingMode, Fragment, FragmentConfig,
    SemanticSearchError, SimulatedEmbeddingEndpoint, VectorNormalization,
};
use std::cell::Cell;
use std::fs;
use std::path::PathBuf;

fn test_path(name: &str) -> PathBuf {
    let unique = uuid::Uuid::new_v4();
    let dir = std::env::temp_dir().join(format!("continuity-search-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

#[derive(Default)]
struct KeywordEndpoint {
    query_inputs: Cell<usize>,
    document_inputs: Cell<usize>,
}

impl KeywordEndpoint {
    fn reset_counts(&self) {
        self.query_inputs.set(0);
        self.document_inputs.set(0);
    }

    fn vector(input: &str) -> Vec<f32> {
        if input.contains("alpha") {
            vec![1.0, 0.0]
        } else if input.contains("near") {
            vec![0.8, 0.6]
        } else if input.contains("beta") {
            vec![0.0, 1.0]
        } else {
            vec![0.6, 0.8]
        }
    }
}

impl EmbeddingEndpoint for KeywordEndpoint {
    fn dimensions(&self) -> u32 {
        2
    }

    fn normalization(&self) -> VectorNormalization {
        VectorNormalization::L2
    }

    fn embed(
        &self,
        mode: EmbeddingMode,
        inputs: &[String],
    ) -> Result<Vec<Vec<f32>>, EmbeddingEndpointError> {
        match mode {
            EmbeddingMode::Query => self
                .query_inputs
                .set(self.query_inputs.get() + inputs.len()),
            EmbeddingMode::Document => self
                .document_inputs
                .set(self.document_inputs.get() + inputs.len()),
        }
        Ok(inputs.iter().map(|input| Self::vector(input)).collect())
    }
}

fn add_conversation(cva: &mut Cva, conversation: &str, text: &str) -> Fragment {
    for index in 0..8 {
        cva.append_node(
            format!("{conversation}-n{index}"),
            conversation.into(),
            (index > 0).then(|| format!("{conversation}-n{}", index - 1)),
            if index % 2 == 0 { "user" } else { "assistant" }.into(),
            index as i64,
            text,
        )
        .unwrap();
    }
    cva.materialize_path_fragments(
        conversation,
        &format!("{conversation}-n7"),
        FragmentConfig::default(),
        false,
    )
    .unwrap()
    .into_iter()
    .next()
    .unwrap()
}

#[test]
fn semantic_search_exactly_ranks_current_generation_and_reopens() {
    let path = test_path("search.cva");
    let endpoint = KeywordEndpoint::default();
    let mut cva = Cva::create(&path).unwrap();
    add_conversation(&mut cva, "alpha-conversation", "alpha evidence");
    add_conversation(&mut cva, "near-conversation", "near evidence");
    add_conversation(&mut cva, "beta-conversation", "beta evidence");
    let profile = cva.establish_compatibility_profile(&endpoint).unwrap();
    let generation = cva
        .build_archive_vector_generation(profile.id, &endpoint)
        .unwrap();
    let archive_version = cva.archive_version();
    let vector_version = cva.vector_version();
    let global_version = cva.container.latest_version();
    endpoint.reset_counts();

    let hits = cva
        .semantic_search(profile.id, &endpoint, "alpha", 3)
        .unwrap();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].fragment.conversation_id, "alpha-conversation");
    assert_eq!(hits[1].fragment.conversation_id, "near-conversation");
    assert!((hits[0].score - 1.0).abs() < 1e-6);
    assert!((hits[1].score - 0.8).abs() < 1e-6);
    assert!(hits.iter().all(|hit| hit.generation_id == generation.id));
    assert_eq!(endpoint.query_inputs.get(), 3);
    assert_eq!(endpoint.document_inputs.get(), 2);
    assert_eq!(cva.archive_version(), archive_version);
    assert_eq!(cva.vector_version(), vector_version);
    assert_eq!(cva.container.latest_version(), global_version);
    cva.sync().unwrap();
    drop(cva);

    let mut reopened = Cva::open(path).unwrap();
    let hits = reopened
        .semantic_search(profile.id, &endpoint, "alpha", 1)
        .unwrap();
    assert_eq!(hits[0].fragment.conversation_id, "alpha-conversation");
    assert_eq!(hits[0].generation_id, generation.id);
}

#[test]
fn semantic_search_uses_newest_generation_for_profile() {
    let path = test_path("current.cva");
    let endpoint = KeywordEndpoint::default();
    let mut cva = Cva::create(path).unwrap();
    add_conversation(&mut cva, "beta-conversation", "beta evidence");
    let profile = cva.establish_compatibility_profile(&endpoint).unwrap();
    let first = cva
        .build_archive_vector_generation(profile.id, &endpoint)
        .unwrap();
    add_conversation(&mut cva, "alpha-conversation", "alpha evidence");
    let second = cva
        .build_archive_vector_generation(profile.id, &endpoint)
        .unwrap();
    assert_ne!(first.id, second.id);

    let hits = cva
        .semantic_search(profile.id, &endpoint, "alpha", 1)
        .unwrap();
    assert_eq!(hits[0].fragment.conversation_id, "alpha-conversation");
    assert_eq!(hits[0].generation_id, second.id);
}

#[test]
fn semantic_search_rejects_invalid_requests_and_incompatible_endpoint() {
    let path = test_path("errors.cva");
    let endpoint = KeywordEndpoint::default();
    let mut cva = Cva::create(path).unwrap();
    add_conversation(&mut cva, "alpha-conversation", "alpha evidence");
    let profile = cva.establish_compatibility_profile(&endpoint).unwrap();

    assert!(matches!(
        cva.semantic_search(profile.id, &endpoint, "alpha", 1),
        Err(SemanticSearchError::MissingGeneration)
    ));
    cva.build_archive_vector_generation(profile.id, &endpoint)
        .unwrap();
    assert!(matches!(
        cva.semantic_search(profile.id, &endpoint, "   ", 1),
        Err(SemanticSearchError::EmptyQuery)
    ));
    assert!(matches!(
        cva.semantic_search(profile.id, &endpoint, "alpha", 0),
        Err(SemanticSearchError::InvalidLimit)
    ));
    let incompatible = SimulatedEmbeddingEndpoint::new(3, VectorNormalization::L2, 9);
    assert!(matches!(
        cva.semantic_search(profile.id, &incompatible, "alpha", 1),
        Err(SemanticSearchError::IncompatibleEndpoint)
    ));
}
