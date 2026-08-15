use crate::config_args::{ConfigCommand, CredentialCommand, ModelCommand};
use crate::util::{credential_id, normalization, provider, read_secret};
use anyhow::{Result, anyhow};
use continuity_memory::{
    ContinuityConfig, EmbeddingModelEndpoint, GeneralModelEndpoint, ModelSwitchboard,
};
use std::path::Path;

pub fn run(path: &Path, command: ConfigCommand) -> Result<()> {
    match command {
        ConfigCommand::Show => show(path),
        ConfigCommand::Verify => verify(path),
        ConfigCommand::Credential { command } => credential(path, command),
        ConfigCommand::Model { command } => model(path, command),
    }
}

fn show(path: &Path) -> Result<()> {
    let config = load(path)?;
    println!("config: {}", path.display());
    println!(
        "fragments: turns={} overlap={}",
        config.fragments.turns, config.fragments.overlap
    );
    println!(
        "retrieval: candidates={} results={} lexical={} semantic={}",
        config.retrieval.candidate_limit,
        config.retrieval.result_limit,
        config.retrieval.lexical_weight,
        config.retrieval.semantic_weight
    );
    show_general(config.models.general.as_ref());
    show_embedding(config.models.embedding.as_ref());
    println!("credentials:");
    for id in config.credentials.ids() {
        let kind = config.credentials.get(id).map(|value| value.auth_kind());
        println!("  {}  {:?}", id.as_str(), kind.unwrap());
    }
    Ok(())
}

fn verify(path: &Path) -> Result<()> {
    let config = ContinuityConfig::open(path)?;
    ModelSwitchboard::new(config.models, config.credentials)?;
    println!("config ok: {}", path.display());
    Ok(())
}

fn credential(path: &Path, command: CredentialCommand) -> Result<()> {
    let mut config = load(path)?;
    match command {
        CredentialCommand::List => {
            for id in config.credentials.ids() {
                let credential = config.credentials.get(id).unwrap();
                println!("{}\t{:?}", id.as_str(), credential.auth_kind());
            }
            return Ok(());
        }
        CredentialCommand::AddApiKey { id, stdin } => {
            let id = credential_id(id)?;
            let secret = read_secret("API key: ", stdin)?;
            config.credentials.insert_api_key(id.clone(), secret)?;
            config.save()?;
            println!("saved API-key credential: {}", id.as_str());
        }
        CredentialCommand::AddCodexTokens { id, account_id } => {
            let id = credential_id(id)?;
            let id_token = read_secret("ID token: ", false)?;
            let access_token = read_secret("Access token: ", false)?;
            let refresh_token = read_secret("Refresh token: ", false)?;
            config.credentials.insert_chatgpt_oauth(
                id.clone(),
                id_token,
                access_token,
                refresh_token,
                account_id,
            )?;
            config.save()?;
            println!("saved ChatGPT OAuth credential: {}", id.as_str());
        }
        CredentialCommand::Remove { id } => {
            let id = credential_id(id)?;
            if config.credentials.remove(&id).is_none() {
                return Err(anyhow!("credential not found: {}", id.as_str()));
            }
            ModelSwitchboard::new(config.models.clone(), config.credentials.clone())?;
            config.save()?;
            println!("removed credential: {}", id.as_str());
        }
    }
    Ok(())
}

fn model(path: &Path, command: ModelCommand) -> Result<()> {
    let mut config = load(path)?;
    match command {
        ModelCommand::SetGeneral {
            provider: p,
            model,
            credential,
            url,
        } => {
            config.models.general = Some(GeneralModelEndpoint {
                provider: provider(p),
                model,
                url,
                credential_id: credential_id(credential)?,
            });
            validate_runtime(&config)?;
        }
        ModelCommand::ClearGeneral => config.models.general = None,
        ModelCommand::SetEmbedding {
            provider: p,
            model,
            credential,
            url,
            dimensions,
            normalization: n,
        } => {
            config.models.embedding = Some(EmbeddingModelEndpoint {
                provider: provider(p),
                model,
                url: Some(url),
                credential_id: credential_id(credential)?,
                dimensions,
                normalization: normalization(n),
            });
            validate_runtime(&config)?;
        }
        ModelCommand::ClearEmbedding => config.models.embedding = None,
    }
    config.save()?;
    println!("saved model configuration: {}", path.display());
    Ok(())
}

fn load(path: &Path) -> Result<ContinuityConfig> {
    if path.exists() {
        Ok(ContinuityConfig::open(path)?)
    } else {
        Ok(ContinuityConfig::new(path))
    }
}

fn validate_runtime(config: &ContinuityConfig) -> Result<()> {
    ModelSwitchboard::new(config.models.clone(), config.credentials.clone())?;
    Ok(())
}

fn show_general(value: Option<&GeneralModelEndpoint>) {
    match value {
        Some(endpoint) => println!(
            "general: provider={:?} model={} url={} credential={}",
            endpoint.provider,
            endpoint.model,
            endpoint.url.as_deref().unwrap_or("<provider-owned>"),
            endpoint.credential_id.as_str()
        ),
        None => println!("general: <unset>"),
    }
}

fn show_embedding(value: Option<&EmbeddingModelEndpoint>) {
    match value {
        Some(endpoint) => println!(
            "embedding: provider={:?} model={} url={} credential={} dimensions={} normalization={:?}",
            endpoint.provider,
            endpoint.model,
            endpoint.url.as_deref().unwrap_or("<unset>"),
            endpoint.credential_id.as_str(),
            endpoint.dimensions,
            endpoint.normalization
        ),
        None => println!("embedding: <unset>"),
    }
}
