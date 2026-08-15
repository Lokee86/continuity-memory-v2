use sha2::{Digest, Sha256};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmbeddingMode {
    Query,
    Document,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VectorNormalization {
    None,
    L2,
}

impl VectorNormalization {
    pub(crate) fn tag(self) -> u8 {
        match self {
            Self::None => 0,
            Self::L2 => 1,
        }
    }

    pub(crate) fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            0 => Some(Self::None),
            1 => Some(Self::L2),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbeddingEndpointDescriptor {
    pub provider: String,
    pub model: String,
    pub revision: String,
    pub dimensions: u32,
    pub normalization: VectorNormalization,
}

#[derive(Debug)]
pub enum EmbeddingEndpointError {
    InvalidDescriptor(&'static str),
    InvalidResponse(&'static str),
    Failure(String),
}

impl fmt::Display for EmbeddingEndpointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "embedding endpoint error: {self:?}")
    }
}

impl std::error::Error for EmbeddingEndpointError {}

pub trait EmbeddingEndpoint {
    fn descriptor(&self) -> EmbeddingEndpointDescriptor;

    fn embed(
        &self,
        mode: EmbeddingMode,
        inputs: &[String],
    ) -> Result<Vec<Vec<f32>>, EmbeddingEndpointError>;
}

#[derive(Clone, Debug)]
pub struct SimulatedEmbeddingEndpoint {
    descriptor: EmbeddingEndpointDescriptor,
    seed: u64,
}

impl SimulatedEmbeddingEndpoint {
    pub fn new(descriptor: EmbeddingEndpointDescriptor, seed: u64) -> Self {
        Self { descriptor, seed }
    }
}

impl EmbeddingEndpoint for SimulatedEmbeddingEndpoint {
    fn descriptor(&self) -> EmbeddingEndpointDescriptor {
        self.descriptor.clone()
    }

    fn embed(
        &self,
        mode: EmbeddingMode,
        inputs: &[String],
    ) -> Result<Vec<Vec<f32>>, EmbeddingEndpointError> {
        if self.descriptor.dimensions == 0 {
            return Err(EmbeddingEndpointError::InvalidDescriptor("zero dimensions"));
        }
        inputs
            .iter()
            .map(|input| self.embed_one(mode, input))
            .collect()
    }
}

impl SimulatedEmbeddingEndpoint {
    fn embed_one(
        &self,
        mode: EmbeddingMode,
        input: &str,
    ) -> Result<Vec<f32>, EmbeddingEndpointError> {
        let dimensions = usize::try_from(self.descriptor.dimensions)
            .map_err(|_| EmbeddingEndpointError::InvalidDescriptor("dimensions overflow"))?;
        let mut output = Vec::with_capacity(dimensions);
        let mode_tag = match mode {
            EmbeddingMode::Query => 1_u8,
            EmbeddingMode::Document => 2_u8,
        };
        let mut block = 0_u32;
        while output.len() < dimensions {
            let mut hash = Sha256::new();
            hash.update(b"CONTINUITY-SIMULATED-EMBEDDING-V1\0");
            hash.update(self.seed.to_le_bytes());
            hash.update([mode_tag]);
            hash.update(block.to_le_bytes());
            hash.update(input.as_bytes());
            let digest = hash.finalize();
            for raw in digest.chunks_exact(4) {
                if output.len() == dimensions {
                    break;
                }
                let value = u32::from_le_bytes(raw.try_into().expect("u32 width"));
                output.push((value as f64 / u32::MAX as f64 * 2.0 - 1.0) as f32);
            }
            block = block.checked_add(1).ok_or(EmbeddingEndpointError::Failure(
                "simulated block overflow".into(),
            ))?;
        }
        if self.descriptor.normalization == VectorNormalization::L2 {
            let norm = output
                .iter()
                .map(|value| (*value as f64) * (*value as f64))
                .sum::<f64>()
                .sqrt();
            if norm == 0.0 {
                return Err(EmbeddingEndpointError::InvalidResponse("zero vector"));
            }
            for value in &mut output {
                *value = (*value as f64 / norm) as f32;
            }
        }
        Ok(output)
    }
}
