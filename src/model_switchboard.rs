use crate::{ConfigError, VectorNormalization};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelProvider {
    OpenAiCodex,
    OpenAiReady,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelCapability {
    General,
    Embedding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelAuthKind {
    ChatGptDeviceCode,
    ApiKey,
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
            (_, ModelCapability::General) => true,
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbeddingModelEndpoint {
    pub provider: ModelProvider,
    pub model: String,
    pub url: Option<String>,
    pub dimensions: u32,
    pub normalization: VectorNormalization,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModelSwitchboardConfig {
    pub general: Option<GeneralModelEndpoint>,
    pub embedding: Option<EmbeddingModelEndpoint>,
}

#[derive(Clone, Debug)]
pub struct ModelSwitchboard {
    config: ModelSwitchboardConfig,
}

impl ModelSwitchboard {
    pub fn new(config: ModelSwitchboardConfig) -> Result<Self, ConfigError> {
        validate_switchboard(&config)?;
        Ok(Self { config })
    }

    pub fn general(&self) -> Option<&GeneralModelEndpoint> {
        self.config.general.as_ref()
    }

    pub fn embedding(&self) -> Option<&EmbeddingModelEndpoint> {
        self.config.embedding.as_ref()
    }

    pub fn config(&self) -> &ModelSwitchboardConfig {
        &self.config
    }
}

pub(crate) fn validate_switchboard(config: &ModelSwitchboardConfig) -> Result<(), ConfigError> {
    if let Some(endpoint) = &config.general {
        validate_general(endpoint)?;
    }
    if let Some(endpoint) = &config.embedding {
        validate_embedding(endpoint)?;
    }
    Ok(())
}

fn validate_general(endpoint: &GeneralModelEndpoint) -> Result<(), ConfigError> {
    validate_endpoint(endpoint.provider, &endpoint.model, endpoint.url.as_deref())
}

fn validate_embedding(endpoint: &EmbeddingModelEndpoint) -> Result<(), ConfigError> {
    if !endpoint.provider.supports(ModelCapability::Embedding) || endpoint.dimensions == 0 {
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
