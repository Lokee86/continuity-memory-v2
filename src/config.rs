use crate::config_codec::{RawConfigObject, decode_config, encode_config};
use crate::config_io::{read_config_file, replace_config_file};
use crate::config_object::{
    FRAGMENT_KEY, OBJECT_FLAGS_NONE, OBJECT_SCHEMA_V1, RETRIEVAL_KEY, decode_fragments,
    decode_retrieval, encode_fragments, encode_retrieval, validate_fragments, validate_retrieval,
};
use crate::{ConfigError, FragmentConfig, RetrievalConfig};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct ContinuityConfig {
    path: PathBuf,
    pub fragments: FragmentConfig,
    pub retrieval: RetrievalConfig,
    extra_objects: BTreeMap<String, RawConfigObject>,
}

impl ContinuityConfig {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            fragments: FragmentConfig::default(),
            retrieval: RetrievalConfig::default(),
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
        Ok(Self {
            path,
            fragments,
            retrieval,
            extra_objects: objects,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        validate_fragments(self.fragments)?;
        validate_retrieval(self.retrieval)?;
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

fn validate_known_object(object: &RawConfigObject) -> Result<(), ConfigError> {
    if object.schema != OBJECT_SCHEMA_V1 || object.flags != OBJECT_FLAGS_NONE {
        return Err(ConfigError::InvalidObject);
    }
    Ok(())
}
