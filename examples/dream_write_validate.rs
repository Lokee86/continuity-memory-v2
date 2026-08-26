#[path = "dream_write_validate/duplicate.rs"]
mod duplicate;
#[path = "dream_write_validate/scenarios.rs"]
mod scenarios;
#[path = "dream_write_validate/support.rs"]
mod support;

use reliquary_memory::{
    GeneralEndpoint, GeneralEndpointError, ModelSwitchboard, OpenAiReadyGeneralEndpoint,
    ReliquaryConfig,
};
use serde_json::Value;
use std::env;
use std::error::Error;
use std::path::PathBuf;

#[derive(Clone)]
pub(crate) struct DiagnosticEndpoint {
    inner: OpenAiReadyGeneralEndpoint,
}

impl GeneralEndpoint for DiagnosticEndpoint {
    fn model(&self) -> &str {
        self.inner.model()
    }

    fn complete_json(
        &self,
        system_prompt: &str,
        user_payload: &str,
        schema_name: &str,
        schema: &Value,
    ) -> Result<Value, GeneralEndpointError> {
        let value = self
            .inner
            .complete_json(system_prompt, user_payload, schema_name, schema)?;
        println!("MODEL_OUTPUT\t{schema_name}\t{value}");
        Ok(value)
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("dream write validation failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let config_path = args
        .next()
        .map(PathBuf::from)
        .ok_or("usage: dream_write_validate <config> <output-dir>")?;
    let output_dir = args
        .next()
        .map(PathBuf::from)
        .ok_or("missing output directory")?;
    let config = ReliquaryConfig::open(config_path)?;
    let switchboard = ModelSwitchboard::new(config.models, config.credentials)?;
    let endpoint = DiagnosticEndpoint {
        inner: OpenAiReadyGeneralEndpoint::from_dream_switchboard(&switchboard)?,
    };
    println!("model={}", endpoint.model());

    scenarios::temporal_only(&endpoint, &output_dir)?;
    scenarios::supersession(&endpoint, &output_dir)?;
    duplicate::run(&endpoint, &output_dir)?;
    println!("WRITE_VALIDATION_SUMMARY\ttemporal=pass\tsupersession=pass\tduplicate_chain=pass");
    Ok(())
}
