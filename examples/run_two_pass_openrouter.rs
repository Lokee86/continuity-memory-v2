use continuity_memory::{ContinuityConfig, ModelSwitchboard};
use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("openrouter launcher failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<u8, Box<dyn std::error::Error>> {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config = ContinuityConfig::open(repo.join("continuity.cfg"))?;
    let switchboard = ModelSwitchboard::new(config.models, config.credentials)?;
    let auth = switchboard
        .embedding_auth()
        .ok_or("configured embedding/OpenRouter credential is missing")?;
    let api_key = auth
        .authorization_header()
        .strip_prefix("Bearer ")
        .ok_or("unexpected authorization header")?;

    let mut command = Command::new("python");
    command
        .arg(repo.join("tools/run_insomnia_two_pass.py"))
        .arg("--endpoint")
        .arg("https://openrouter.ai/api/v1/chat/completions")
        .env("NOUS_API_KEY", api_key)
        .current_dir(&repo);
    command.args(env::args().skip(1));
    let status = command.status()?;
    Ok(status.code().unwrap_or(1).clamp(0, 255) as u8)
}
