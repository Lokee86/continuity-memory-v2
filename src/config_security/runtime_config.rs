use crate::DEFAULT_DREAM_INFERENCE_CONCURRENCY;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeConfig {
    pub dream_inference_concurrency: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            dream_inference_concurrency: DEFAULT_DREAM_INFERENCE_CONCURRENCY,
        }
    }
}
