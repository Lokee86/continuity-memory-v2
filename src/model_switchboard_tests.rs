use crate::{
    ContinuityConfig, EmbeddingModelEndpoint, GeneralModelEndpoint, ModelAuthKind, ModelCapability,
    ModelProvider, ModelSwitchboard, ModelSwitchboardConfig, VectorNormalization,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn test_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("continuity-switchboard-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

#[test]
fn provider_auth_and_capabilities_are_explicit() {
    assert_eq!(
        ModelProvider::OpenAiCodex.auth_kind(),
        ModelAuthKind::ChatGptDeviceCode
    );
    assert_eq!(
        ModelProvider::OpenAiReady.auth_kind(),
        ModelAuthKind::ApiKey
    );
    assert!(ModelProvider::OpenAiCodex.supports(ModelCapability::General));
    assert!(!ModelProvider::OpenAiCodex.supports(ModelCapability::Embedding));
    assert!(ModelProvider::OpenAiReady.supports(ModelCapability::General));
    assert!(ModelProvider::OpenAiReady.supports(ModelCapability::Embedding));
}

#[test]
fn general_and_embedding_routes_round_trip_through_config() {
    let path = test_path("models.cfg");
    let mut config = ContinuityConfig::new(&path);
    config.models = configured_models();
    config.save().unwrap();

    let reopened = ContinuityConfig::open(&path).unwrap();
    assert_eq!(reopened.models, config.models);

    let switchboard = ModelSwitchboard::new(reopened.models).unwrap();
    assert_eq!(
        switchboard.general().unwrap().provider,
        ModelProvider::OpenAiCodex
    );
    assert_eq!(
        switchboard.embedding().unwrap().provider,
        ModelProvider::OpenAiReady
    );
}

#[test]
fn clearing_model_routes_removes_them_from_current_config() {
    let path = test_path("clear-models.cfg");
    let mut config = ContinuityConfig::new(&path);
    config.models = configured_models();
    config.save().unwrap();
    let configured_len = fs::metadata(&path).unwrap().len();

    config.models = ModelSwitchboardConfig::default();
    config.save().unwrap();
    let cleared_len = fs::metadata(&path).unwrap().len();

    let reopened = ContinuityConfig::open(&path).unwrap();
    assert_eq!(reopened.models, ModelSwitchboardConfig::default());
    assert!(cleared_len < configured_len);
}

#[test]
fn codex_cannot_be_configured_as_embedding_provider() {
    let config = ModelSwitchboardConfig {
        general: None,
        embedding: Some(EmbeddingModelEndpoint {
            provider: ModelProvider::OpenAiCodex,
            model: "codex".into(),
            url: None,
            dimensions: 1024,
            normalization: VectorNormalization::L2,
        }),
    };
    assert!(ModelSwitchboard::new(config).is_err());
}

#[test]
fn openai_ready_requires_an_explicit_http_endpoint() {
    let config = ModelSwitchboardConfig {
        general: Some(GeneralModelEndpoint {
            provider: ModelProvider::OpenAiReady,
            model: "model".into(),
            url: None,
        }),
        embedding: None,
    };
    assert!(ModelSwitchboard::new(config).is_err());
}

fn configured_models() -> ModelSwitchboardConfig {
    ModelSwitchboardConfig {
        general: Some(GeneralModelEndpoint {
            provider: ModelProvider::OpenAiCodex,
            model: "gpt-codex".into(),
            url: None,
        }),
        embedding: Some(EmbeddingModelEndpoint {
            provider: ModelProvider::OpenAiReady,
            model: "qwen/qwen3-embedding-8b".into(),
            url: Some("https://example.test/v1/embeddings".into()),
            dimensions: 1024,
            normalization: VectorNormalization::L2,
        }),
    }
}
