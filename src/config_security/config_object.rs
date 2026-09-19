use crate::{ConfigError, FragmentConfig, RetrievalConfig};

pub(crate) const FRAGMENT_KEY: &str = "archive.fragments";
pub(crate) const RETRIEVAL_KEY: &str = "retrieval.default";
pub(crate) const OBJECT_SCHEMA_V1: u16 = 1;
pub(crate) const OBJECT_FLAGS_NONE: u32 = 0;

pub(crate) fn encode_fragments(config: FragmentConfig) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(16);
    bytes.extend_from_slice(&(config.turns as u64).to_le_bytes());
    bytes.extend_from_slice(&(config.overlap as u64).to_le_bytes());
    bytes
}

pub(crate) fn decode_fragments(bytes: &[u8]) -> Result<FragmentConfig, ConfigError> {
    if bytes.len() != 16 {
        return Err(ConfigError::InvalidFragmentConfig);
    }
    let turns = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
    let overlap = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
    let turns = usize::try_from(turns).map_err(|_| ConfigError::InvalidFragmentConfig)?;
    let overlap = usize::try_from(overlap).map_err(|_| ConfigError::InvalidFragmentConfig)?;
    let config = FragmentConfig { turns, overlap };
    validate_fragments(config)?;
    Ok(config)
}

pub(crate) fn encode_retrieval(config: RetrievalConfig) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(32);
    bytes.extend_from_slice(&(config.candidate_limit as u64).to_le_bytes());
    bytes.extend_from_slice(&(config.result_limit as u64).to_le_bytes());
    bytes.extend_from_slice(&config.lexical_weight.to_le_bytes());
    bytes.extend_from_slice(&config.semantic_weight.to_le_bytes());
    bytes
}

pub(crate) fn decode_retrieval(bytes: &[u8]) -> Result<RetrievalConfig, ConfigError> {
    if bytes.len() != 32 {
        return Err(ConfigError::InvalidRetrievalConfig);
    }
    let candidate_limit = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
    let result_limit = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
    let lexical_weight = f64::from_le_bytes(bytes[16..24].try_into().unwrap());
    let semantic_weight = f64::from_le_bytes(bytes[24..32].try_into().unwrap());
    let config = RetrievalConfig {
        candidate_limit: usize::try_from(candidate_limit)
            .map_err(|_| ConfigError::InvalidRetrievalConfig)?,
        result_limit: usize::try_from(result_limit)
            .map_err(|_| ConfigError::InvalidRetrievalConfig)?,
        lexical_weight,
        semantic_weight,
    };
    validate_retrieval(config)?;
    Ok(config)
}

pub(crate) fn validate_fragments(config: FragmentConfig) -> Result<(), ConfigError> {
    if config.turns == 0 || config.overlap >= config.turns {
        return Err(ConfigError::InvalidFragmentConfig);
    }
    Ok(())
}

pub(crate) fn validate_retrieval(config: RetrievalConfig) -> Result<(), ConfigError> {
    if config.normalized_weights().is_none()
        || config.candidate_limit > crate::MAX_SEMANTIC_SEARCH_LIMIT
    {
        return Err(ConfigError::InvalidRetrievalConfig);
    }
    Ok(())
}
