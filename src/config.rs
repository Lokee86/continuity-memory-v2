use crate::config_codec::{RawConfigObject, decode_config, encode_config};
use crate::config_io::{read_config_file, replace_config_file};
use crate::config_object::{
    FRAGMENT_KEY, OBJECT_FLAGS_NONE, OBJECT_SCHEMA_V1, RETRIEVAL_KEY, decode_fragments,
    decode_retrieval, encode_fragments, encode_retrieval, validate_fragments, validate_retrieval,
};
use crate::model_switchboard::validate_switchboard;
use crate::model_switchboard_codec::{
    EMBEDDING_MODEL_KEY, GENERAL_MODEL_KEY, decode_embedding, decode_general, encode_embedding,
    encode_general,
};
use crate::{
    ConfigError, FragmentConfig, JsonMasterKeyStore, MasterKey, MasterKeyError, MasterKeyStore,
    ModelSwitchboardConfig, RetrievalConfig,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct ContinuityConfig {
    path: PathBuf,
    pub fragments: FragmentConfig,
    pub retrieval: RetrievalConfig,
    pub models: ModelSwitchboardConfig,
    extra_objects: BTreeMap<String, RawConfigObject>,
}

impl ContinuityConfig {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            fragments: FragmentConfig::default(),
            retrieval: RetrievalConfig::default(),
            models: ModelSwitchboardConfig::default(),
            extra_objects: BTreeMap::new(),
        }
    }

    pub fn open(path: impl Into<PathBuf>) -> Result<Self, ConfigError> {
        let path = path.into();
        let bytes = read_config_file(&path)?;
        let mut objects = decode_config(&bytes)?;
        let fragments = match objects.remove(FRAGMENT_KEY) {
            Some(object) => decode_known_fragments(object)?,
            None => FragmentConfig::default(),
        };
        let retrieval = match objects.remove(RETRIEVAL_KEY) {
            Some(object) => decode_known_retrieval(object)?,
            None => RetrievalConfig::default(),
        };
        let general = match objects.remove(GENERAL_MODEL_KEY) {
            Some(object) => Some(decode_known_general(object)?),
            None => None,
        };
        let embedding = match objects.remove(EMBEDDING_MODEL_KEY) {
            Some(object) => Some(decode_known_embedding(object)?),
            None => None,
        };
        let models = ModelSwitchboardConfig { general, embedding };
        validate_switchboard(&models)?;
        Ok(Self {
            path,
            fragments,
            retrieval,
            models,
            extra_objects: objects,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load_or_create_master_key(&self) -> Result<MasterKey, MasterKeyError> {
        JsonMasterKeyStore::new(self.path.with_file_name("continuity.master-key.json"))
            .load_or_create()
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        validate_fragments(self.fragments)?;
        validate_retrieval(self.retrieval)?;
        validate_switchboard(&self.models)?;
        let mut objects = self.extra_objects.clone();
        objects.insert(
            FRAGMENT_KEY.to_owned(),
            RawConfigObject {
                schema: OBJECT_SCHEMA_V1,
                flags: OBJECT_FLAGS_NONE,
                payload: encode_fragments(self.fragments),
            },
        );
        objects.insert(
            RETRIEVAL_KEY.to_owned(),
            RawConfigObject {
                schema: OBJECT_SCHEMA_V1,
                flags: OBJECT_FLAGS_NONE,
                payload: encode_retrieval(self.retrieval),
            },
        );
        if let Some(endpoint) = &self.models.general {
            objects.insert(
                GENERAL_MODEL_KEY.to_owned(),
                known_object(encode_general(endpoint)?),
            );
        }
        if let Some(endpoint) = &self.models.embedding {
            objects.insert(
                EMBEDDING_MODEL_KEY.to_owned(),
                known_object(encode_embedding(endpoint)?),
            );
        }
        let bytes = encode_config(&objects)?;
        replace_config_file(&self.path, &bytes)
    }
}

fn decode_known_fragments(object: RawConfigObject) -> Result<FragmentConfig, ConfigError> {
    validate_known_object(&object)?;
    decode_fragments(&object.payload)
}

fn decode_known_retrieval(object: RawConfigObject) -> Result<RetrievalConfig, ConfigError> {
    validate_known_object(&object)?;
    decode_retrieval(&object.payload)
}

fn decode_known_general(
    object: RawConfigObject,
) -> Result<crate::GeneralModelEndpoint, ConfigError> {
    validate_known_object(&object)?;
    decode_general(&object.payload)
}

fn decode_known_embedding(
    object: RawConfigObject,
) -> Result<crate::EmbeddingModelEndpoint, ConfigError> {
    validate_known_object(&object)?;
    decode_embedding(&object.payload)
}

fn known_object(payload: Vec<u8>) -> RawConfigObject {
    RawConfigObject {
        schema: OBJECT_SCHEMA_V1,
        flags: OBJECT_FLAGS_NONE,
        payload,
    }
}

fn validate_known_object(object: &RawConfigObject) -> Result<(), ConfigError> {
    if object.schema != OBJECT_SCHEMA_V1 || object.flags != OBJECT_FLAGS_NONE {
        return Err(ConfigError::InvalidObject);
    }
    Ok(())
}
