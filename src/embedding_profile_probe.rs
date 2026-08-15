use crate::{
    EMBEDDING_PROBE_SUITE_VERSION, EmbeddingEndpoint, EmbeddingEndpointDescriptor, EmbeddingMode,
    EmbeddingProfile, EmbeddingProfileError, EmbeddingProfileId, VectorNormalization,
};
use sha2::{Digest, Sha256};

const QUERY_PROBES: [&str; 2] = [
    "continuity probe query: cedar orbit",
    "continuity probe query: violet engine",
];
const DOCUMENT_PROBES: [&str; 2] = [
    "continuity probe document: quiet harbor",
    "continuity probe document: copper lantern",
];

pub(crate) fn profile_from_endpoint(
    endpoint: &impl EmbeddingEndpoint,
) -> Result<EmbeddingProfile, EmbeddingProfileError> {
    let descriptor = endpoint.descriptor();
    validate_descriptor(&descriptor)?;
    let fingerprint = behavior_fingerprint(endpoint, &descriptor)?;
    let mut profile = EmbeddingProfile {
        id: EmbeddingProfileId([0; 32]),
        provider: descriptor.provider,
        model: descriptor.model,
        revision: descriptor.revision,
        dimensions: descriptor.dimensions,
        normalization: descriptor.normalization,
        probe_suite_version: EMBEDDING_PROBE_SUITE_VERSION,
        behavior_fingerprint: fingerprint,
    };
    profile.id = embedding_profile_id(&profile);
    Ok(profile)
}

pub(crate) fn verify_endpoint(
    profile: &EmbeddingProfile,
    endpoint: &impl EmbeddingEndpoint,
) -> Result<(), EmbeddingProfileError> {
    let observed = profile_from_endpoint(endpoint)?;
    if observed != *profile {
        return Err(EmbeddingProfileError::EndpointMismatch);
    }
    Ok(())
}

pub(crate) fn validate_embedding_batch(
    profile: &EmbeddingProfile,
    vectors: &[Vec<f32>],
    expected: usize,
) -> Result<(), EmbeddingProfileError> {
    if vectors.len() != expected {
        return Err(EmbeddingProfileError::InvalidProfile(
            "embedding count mismatch",
        ));
    }
    let dimensions =
        usize::try_from(profile.dimensions).map_err(|_| EmbeddingProfileError::SizeOverflow)?;
    for vector in vectors {
        if vector.len() != dimensions || vector.iter().any(|value| !value.is_finite()) {
            return Err(EmbeddingProfileError::InvalidProfile(
                "invalid embedding vector",
            ));
        }
        if profile.normalization == VectorNormalization::L2 {
            let norm = vector
                .iter()
                .map(|value| (*value as f64) * (*value as f64))
                .sum::<f64>()
                .sqrt();
            if (norm - 1.0).abs() > 1e-4 {
                return Err(EmbeddingProfileError::InvalidProfile(
                    "embedding not normalized",
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn embedding_profile_id(profile: &EmbeddingProfile) -> EmbeddingProfileId {
    let mut hash = Sha256::new();
    hash.update(b"CVA-EMBEDDING-PROFILE-V1\0");
    for value in [&profile.provider, &profile.model, &profile.revision] {
        hash.update(value.as_bytes());
        hash.update([0]);
    }
    hash.update(profile.dimensions.to_le_bytes());
    hash.update([profile.normalization.tag()]);
    hash.update(profile.probe_suite_version.to_le_bytes());
    hash.update(profile.behavior_fingerprint);
    EmbeddingProfileId(hash.finalize().into())
}

fn behavior_fingerprint(
    endpoint: &impl EmbeddingEndpoint,
    descriptor: &EmbeddingEndpointDescriptor,
) -> Result<[u8; 32], EmbeddingProfileError> {
    let mut hash = Sha256::new();
    hash.update(b"CONTINUITY-EMBEDDING-PROBE-V1\0");
    for (mode, probes) in [
        (EmbeddingMode::Query, QUERY_PROBES.as_slice()),
        (EmbeddingMode::Document, DOCUMENT_PROBES.as_slice()),
    ] {
        let inputs: Vec<String> = probes.iter().map(|probe| (*probe).to_string()).collect();
        let vectors = endpoint.embed(mode, &inputs)?;
        let shell = EmbeddingProfile {
            id: EmbeddingProfileId([0; 32]),
            provider: descriptor.provider.clone(),
            model: descriptor.model.clone(),
            revision: descriptor.revision.clone(),
            dimensions: descriptor.dimensions,
            normalization: descriptor.normalization,
            probe_suite_version: EMBEDDING_PROBE_SUITE_VERSION,
            behavior_fingerprint: [0; 32],
        };
        validate_embedding_batch(&shell, &vectors, probes.len())?;
        hash.update([match mode {
            EmbeddingMode::Query => 1,
            EmbeddingMode::Document => 2,
        }]);
        for vector in vectors {
            for value in vector {
                hash.update(value.to_le_bytes());
            }
        }
    }
    Ok(hash.finalize().into())
}

fn validate_descriptor(
    descriptor: &EmbeddingEndpointDescriptor,
) -> Result<(), EmbeddingProfileError> {
    if descriptor.provider.is_empty() || descriptor.model.is_empty() {
        return Err(EmbeddingProfileError::InvalidProfile(
            "empty provider/model",
        ));
    }
    if descriptor.dimensions == 0 {
        return Err(EmbeddingProfileError::InvalidProfile("zero dimensions"));
    }
    Ok(())
}
