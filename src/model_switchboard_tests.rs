use crate::{
    ConfiguredGeneralEndpoint, CredentialId, CredentialsConfig, EmbeddingModelEndpoint,
    GeneralEndpoint, GeneralModelEndpoint, ModelAuthKind, ModelCapability, ModelProvider,
    ModelSwitchboard, ModelSwitchboardConfig, ReliquaryConfig, VectorNormalization,
};
use std::collections::BTreeMap;
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

fn id(value: &str) -> CredentialId {
    CredentialId::new(value).unwrap()
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
    assert!(ModelProvider::OpenAiCodex.supports(ModelCapability::Insomnia));
    assert!(ModelProvider::OpenAiCodex.supports(ModelCapability::Dream));
    assert!(!ModelProvider::OpenAiCodex.supports(ModelCapability::Embedding));
    assert!(ModelProvider::OpenAiReady.supports(ModelCapability::Insomnia));
    assert!(ModelProvider::OpenAiReady.supports(ModelCapability::Dream));
    assert!(ModelProvider::OpenAiReady.supports(ModelCapability::Embedding));
}

#[test]
fn routes_and_credentials_round_trip_and_attach_auth_headers() {
    let path = test_path("models.cfg");
    let mut config = ReliquaryConfig::new(&path);
    config.models = configured_models();
    config.credentials = configured_credentials();
    config.save().unwrap();

    let reopened = ReliquaryConfig::open(&path).unwrap();
    assert_eq!(reopened.models, config.models);
    assert_eq!(reopened.credentials, config.credentials);

    let switchboard = ModelSwitchboard::new(reopened.models, reopened.credentials).unwrap();
    let mut general = BTreeMap::new();
    switchboard.general_auth().unwrap().apply_to(&mut general);
    assert_eq!(general["Authorization"], "Bearer codex-access");
    assert_eq!(general["ChatGPT-Account-ID"], "account-123");

    let mut insomnia = BTreeMap::new();
    switchboard.insomnia_auth().unwrap().apply_to(&mut insomnia);
    assert_eq!(insomnia["Authorization"], "Bearer ready-key");
    assert!(!insomnia.contains_key("ChatGPT-Account-ID"));

    let mut dream = BTreeMap::new();
    switchboard.dream_auth().unwrap().apply_to(&mut dream);
    assert_eq!(dream["Authorization"], "Bearer ready-key");
    assert!(!dream.contains_key("ChatGPT-Account-ID"));

    let mut embedding = BTreeMap::new();
    switchboard
        .embedding_auth()
        .unwrap()
        .apply_to(&mut embedding);
    assert_eq!(embedding["Authorization"], "Bearer ready-key");
    assert!(!embedding.contains_key("ChatGPT-Account-ID"));
}

#[test]
fn clearing_model_routes_removes_them_from_current_config() {
    let path = test_path("clear-models.cfg");
    let mut config = ReliquaryConfig::new(&path);
    config.models = configured_models();
    config.save().unwrap();
    let configured_len = fs::metadata(&path).unwrap().len();
    config.models = ModelSwitchboardConfig::default();
    config.save().unwrap();
    assert!(fs::metadata(&path).unwrap().len() < configured_len);
    assert_eq!(
        ReliquaryConfig::open(&path).unwrap().models,
        ModelSwitchboardConfig::default()
    );
}

#[test]
fn invalid_provider_routes_are_rejected() {
    let codex_embedding = ModelSwitchboardConfig {
        general: None,
        insomnia: None,
        dream: None,
        embedding: Some(EmbeddingModelEndpoint {
            provider: ModelProvider::OpenAiCodex,
            model: "codex".into(),
            url: None,
            credential_id: id("codex"),
            dimensions: 1024,
            normalization: VectorNormalization::L2,
        }),
    };
    assert!(ModelSwitchboard::new(codex_embedding, CredentialsConfig::default()).is_err());

    let missing_url = ModelSwitchboardConfig {
        general: Some(GeneralModelEndpoint {
            provider: ModelProvider::OpenAiReady,
            model: "model".into(),
            url: None,
            credential_id: id("ready"),
            reasoning_effort: None,
        }),
        insomnia: None,
        dream: None,
        embedding: None,
    };
    assert!(ModelSwitchboard::new(missing_url, CredentialsConfig::default()).is_err());
}

#[test]
fn insomnia_route_falls_back_to_general_when_unset() {
    let mut models = configured_models();
    models.insomnia = None;
    let switchboard = ModelSwitchboard::new(models, configured_credentials()).unwrap();
    assert_eq!(switchboard.insomnia(), switchboard.general());

    let mut auth = BTreeMap::new();
    switchboard.insomnia_auth().unwrap().apply_to(&mut auth);
    assert_eq!(auth["Authorization"], "Bearer codex-access");
    assert_eq!(auth["ChatGPT-Account-ID"], "account-123");
}

#[test]
fn dream_route_falls_back_to_general_when_unset() {
    let mut models = configured_models();
    models.dream = None;
    let switchboard = ModelSwitchboard::new(models, configured_credentials()).unwrap();
    assert_eq!(switchboard.dream(), switchboard.general());

    let mut auth = BTreeMap::new();
    switchboard.dream_auth().unwrap().apply_to(&mut auth);
    assert_eq!(auth["Authorization"], "Bearer codex-access");
    assert_eq!(auth["ChatGPT-Account-ID"], "account-123");
}

#[test]
fn configured_dream_endpoint_uses_dedicated_route() {
    let switchboard = ModelSwitchboard::new(configured_models(), configured_credentials()).unwrap();
    let endpoint = ConfiguredGeneralEndpoint::from_dream_switchboard(&switchboard).unwrap();
    assert_eq!(endpoint.model(), "dream-model");
}

#[test]
fn legacy_general_schema_decodes_without_reasoning() {
    let mut bytes = vec![ModelProvider::OpenAiCodex.tag(), 0, 0, 0];
    for value in ["legacy-codex", "", "codex"] {
        bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
        bytes.extend_from_slice(value.as_bytes());
    }
    let endpoint = crate::model_switchboard_codec::decode_general_v2(&bytes).unwrap();
    assert_eq!(endpoint.provider, ModelProvider::OpenAiCodex);
    assert_eq!(endpoint.model, "legacy-codex");
    assert_eq!(endpoint.reasoning_effort, None);
}

#[test]
fn missing_or_wrong_credential_kind_is_rejected() {
    let models = configured_models();
    assert!(ModelSwitchboard::new(models.clone(), CredentialsConfig::default()).is_err());

    let mut wrong = CredentialsConfig::default();
    wrong.insert_api_key(id("codex"), "wrong-kind").unwrap();
    wrong.insert_api_key(id("ready"), "ready-key").unwrap();
    assert!(ModelSwitchboard::new(models, wrong).is_err());
}

fn configured_models() -> ModelSwitchboardConfig {
    ModelSwitchboardConfig {
        general: Some(GeneralModelEndpoint {
            provider: ModelProvider::OpenAiCodex,
            model: "gpt-codex".into(),
            url: None,
            credential_id: id("codex"),
            reasoning_effort: Some(crate::ModelReasoningEffort::Low),
        }),
        insomnia: Some(GeneralModelEndpoint {
            provider: ModelProvider::OpenAiReady,
            model: "insomnia-model".into(),
            url: Some("https://example.test/v1/chat/completions".into()),
            credential_id: id("ready"),
            reasoning_effort: None,
        }),
        dream: Some(GeneralModelEndpoint {
            provider: ModelProvider::OpenAiReady,
            model: "dream-model".into(),
            url: Some("https://example.test/v1/chat/completions".into()),
            credential_id: id("ready"),
            reasoning_effort: None,
        }),
        embedding: Some(EmbeddingModelEndpoint {
            provider: ModelProvider::OpenAiReady,
            model: "qwen/qwen3-embedding-8b".into(),
            url: Some("https://example.test/v1/embeddings".into()),
            credential_id: id("ready"),
            dimensions: 1024,
            normalization: VectorNormalization::L2,
        }),
    }
}

fn configured_credentials() -> CredentialsConfig {
    let mut credentials = CredentialsConfig::default();
    credentials
        .insert_api_key(id("ready"), "ready-key")
        .unwrap();
    credentials
        .insert_chatgpt_oauth(
            id("codex"),
            "codex-id",
            "codex-access",
            "codex-refresh",
            Some("account-123".into()),
        )
        .unwrap();
    credentials
}
