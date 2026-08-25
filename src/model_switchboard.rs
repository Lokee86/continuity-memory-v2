use crate::model_auth::{resolve_auth, validate_credentials};
use crate::{ConfigError, CredentialId, CredentialsConfig, ModelRequestAuth, VectorNormalization};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelProvider {
    OpenAiCodex,
    OpenAiReady,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelCapability {
    General,
    Insomnia,
    Dream,
    Embedding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelAuthKind {
    ChatGptDeviceCode,
    ApiKey,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelReasoningEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
    Max,
}

impl ModelReasoningEffort {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Minimal => "minimal",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::XHigh => "xhigh",
            Self::Max => "max",
        }
    }

    pub(crate) fn tag(self) -> u8 {
        match self {
            Self::None => 1,
            Self::Minimal => 2,
            Self::Low => 3,
            Self::Medium => 4,
            Self::High => 5,
            Self::XHigh => 6,
            Self::Max => 7,
        }
    }

    pub(crate) fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::None),
            2 => Some(Self::Minimal),
            3 => Some(Self::Low),
            4 => Some(Self::Medium),
            5 => Some(Self::High),
            6 => Some(Self::XHigh),
            7 => Some(Self::Max),
            _ => None,
        }
    }
}

impl ModelProvider {
    pub fn auth_kind(self) -> ModelAuthKind {
        match self {
            Self::OpenAiCodex => ModelAuthKind::ChatGptDeviceCode,
            Self::OpenAiReady => ModelAuthKind::ApiKey,
        }
    }

    pub fn supports(self, capability: ModelCapability) -> bool {
        match (self, capability) {
            (_, ModelCapability::General | ModelCapability::Insomnia | ModelCapability::Dream) => {
                true
            }
            (Self::OpenAiReady, ModelCapability::Embedding) => true,
            (Self::OpenAiCodex, ModelCapability::Embedding) => false,
        }
    }

    pub(crate) fn tag(self) -> u8 {
        match self {
            Self::OpenAiCodex => 1,
            Self::OpenAiReady => 2,
        }
    }

    pub(crate) fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::OpenAiCodex),
            2 => Some(Self::OpenAiReady),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneralModelEndpoint {
    pub provider: ModelProvider,
    pub model: String,
    pub url: Option<String>,
    pub credential_id: CredentialId,
    pub reasoning_effort: Option<ModelReasoningEffort>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbeddingModelEndpoint {
    pub provider: ModelProvider,
    pub model: String,
    pub url: Option<String>,
    pub credential_id: CredentialId,
    pub dimensions: u32,
    pub normalization: VectorNormalization,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModelSwitchboardConfig {
    pub general: Option<GeneralModelEndpoint>,
    pub insomnia: Option<GeneralModelEndpoint>,
    pub dream: Option<GeneralModelEndpoint>,
    pub embedding: Option<EmbeddingModelEndpoint>,
}

#[derive(Clone, Debug)]
pub struct ModelSwitchboard {
    config: ModelSwitchboardConfig,
    credentials: CredentialsConfig,
}

impl ModelSwitchboard {
    pub fn new(
        config: ModelSwitchboardConfig,
        credentials: CredentialsConfig,
    ) -> Result<Self, ConfigError> {
        validate_switchboard(&config)?;
        validate_credentials(&config, &credentials)?;
        Ok(Self {
            config,
            credentials,
        })
    }

    pub fn general(&self) -> Option<&GeneralModelEndpoint> {
        self.config.general.as_ref()
    }

    pub fn insomnia(&self) -> Option<&GeneralModelEndpoint> {
        self.config
            .insomnia
            .as_ref()
            .or(self.config.general.as_ref())
    }

    pub fn dream(&self) -> Option<&GeneralModelEndpoint> {
        self.config.dream.as_ref().or(self.config.general.as_ref())
    }

    pub fn embedding(&self) -> Option<&EmbeddingModelEndpoint> {
        self.config.embedding.as_ref()
    }

    pub fn general_auth(&self) -> Option<ModelRequestAuth> {
        self.config.general.as_ref().map(|endpoint| {
            resolve_auth(
                endpoint.provider,
                &endpoint.credential_id,
                &self.credentials,
            )
        })
    }

    pub fn insomnia_auth(&self) -> Option<ModelRequestAuth> {
        self.insomnia().map(|endpoint| {
            resolve_auth(
                endpoint.provider,
                &endpoint.credential_id,
                &self.credentials,
            )
        })
    }

    pub fn dream_auth(&self) -> Option<ModelRequestAuth> {
        self.dream().map(|endpoint| {
            resolve_auth(
                endpoint.provider,
                &endpoint.credential_id,
                &self.credentials,
            )
        })
    }

    pub fn embedding_auth(&self) -> Option<ModelRequestAuth> {
        self.config.embedding.as_ref().map(|endpoint| {
            resolve_auth(
                endpoint.provider,
                &endpoint.credential_id,
                &self.credentials,
            )
        })
    }

    pub fn config(&self) -> &ModelSwitchboardConfig {
        &self.config
    }
}

pub(crate) fn validate_switchboard(config: &ModelSwitchboardConfig) -> Result<(), ConfigError> {
    if let Some(endpoint) = &config.general {
        validate_general_endpoint(endpoint)?;
    }
    if let Some(endpoint) = &config.insomnia {
        validate_general_endpoint(endpoint)?;
    }
    if let Some(endpoint) = &config.dream {
        validate_general_endpoint(endpoint)?;
    }
    if let Some(endpoint) = &config.embedding {
        if !endpoint.provider.supports(ModelCapability::Embedding) || endpoint.dimensions == 0 {
            return Err(ConfigError::InvalidModelSwitchboard);
        }
        validate_endpoint(endpoint.provider, &endpoint.model, endpoint.url.as_deref())?;
    }
    Ok(())
}

fn validate_general_endpoint(endpoint: &GeneralModelEndpoint) -> Result<(), ConfigError> {
    if endpoint.provider == ModelProvider::OpenAiReady && endpoint.reasoning_effort.is_some() {
        return Err(ConfigError::InvalidModelSwitchboard);
    }
    validate_endpoint(endpoint.provider, &endpoint.model, endpoint.url.as_deref())
}

fn validate_endpoint(
    provider: ModelProvider,
    model: &str,
    url: Option<&str>,
) -> Result<(), ConfigError> {
    if model.trim().is_empty() {
        return Err(ConfigError::InvalidModelSwitchboard);
    }
    match provider {
        ModelProvider::OpenAiCodex if url.is_some() => Err(ConfigError::InvalidModelSwitchboard),
        ModelProvider::OpenAiReady => match url {
            Some(url) if valid_url(url) => Ok(()),
            _ => Err(ConfigError::InvalidModelSwitchboard),
        },
        ModelProvider::OpenAiCodex => Ok(()),
    }
}

fn valid_url(url: &str) -> bool {
    let trimmed = url.trim();
    trimmed.starts_with("https://") || trimmed.starts_with("http://")
}
