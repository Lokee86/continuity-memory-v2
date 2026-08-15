use anyhow::{Result, anyhow};
use continuity_memory::{
    CompatibilityProfileId, CredentialId, FragmentId, ModelProvider, VectorNormalization,
};
use std::io::{self, Read};
use std::path::Path;

pub fn credential_id(value: String) -> Result<CredentialId> {
    CredentialId::new(value).map_err(|error| anyhow!(error))
}

pub fn provider(value: crate::config_args::ProviderArg) -> ModelProvider {
    match value {
        crate::config_args::ProviderArg::OpenAiCodex => ModelProvider::OpenAiCodex,
        crate::config_args::ProviderArg::OpenAiReady => ModelProvider::OpenAiReady,
    }
}

pub fn normalization(value: crate::config_args::NormalizationArg) -> VectorNormalization {
    match value {
        crate::config_args::NormalizationArg::None => VectorNormalization::None,
        crate::config_args::NormalizationArg::L2 => VectorNormalization::L2,
    }
}

pub fn parse_normalization(value: &str) -> Result<VectorNormalization> {
    match value.to_ascii_lowercase().as_str() {
        "none" => Ok(VectorNormalization::None),
        "l2" => Ok(VectorNormalization::L2),
        _ => Err(anyhow!("normalization must be 'none' or 'l2'")),
    }
}

pub fn read_secret(prompt: &str, stdin: bool) -> Result<String> {
    if stdin {
        let mut value = String::new();
        io::stdin().read_to_string(&mut value)?;
        let value = value
            .lines()
            .next()
            .unwrap_or_default()
            .trim_end()
            .to_owned();
        if value.is_empty() {
            return Err(anyhow!("secret input was empty"));
        }
        Ok(value)
    } else {
        let value = rpassword::prompt_password(prompt)?;
        if value.is_empty() {
            return Err(anyhow!("secret input was empty"));
        }
        Ok(value)
    }
}

pub fn parse_fragment_id(value: &str) -> Result<FragmentId> {
    Ok(FragmentId(parse_hex_32(value)?))
}

pub fn parse_profile_id(value: &str) -> Result<CompatibilityProfileId> {
    Ok(CompatibilityProfileId(parse_hex_32(value)?))
}

pub fn hex32(bytes: &[u8; 32]) -> String {
    let mut output = String::with_capacity(64);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

pub fn file_len(path: &Path) -> Result<u64> {
    Ok(std::fs::metadata(path)?.len())
}

fn parse_hex_32(value: &str) -> Result<[u8; 32]> {
    if value.len() != 64 {
        return Err(anyhow!("expected a 64-character hexadecimal id"));
    }
    let mut bytes = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(pair)?;
        bytes[index] =
            u8::from_str_radix(text, 16).map_err(|_| anyhow!("invalid hexadecimal id"))?;
    }
    Ok(bytes)
}
