use crate::{
    Credential, CredentialError, CredentialId, CredentialsConfig, ModelProvider,
    ModelSwitchboardConfig, SecretString,
};
use std::collections::BTreeMap;

#[derive(Clone, Eq, PartialEq)]
pub struct ModelRequestAuth {
    authorization: SecretString,
    pub chatgpt_account_id: Option<String>,
}

impl std::fmt::Debug for ModelRequestAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelRequestAuth")
            .field("authorization", &"[REDACTED]")
            .field("chatgpt_account_id", &self.chatgpt_account_id)
            .finish()
    }
}

impl ModelRequestAuth {
    pub fn authorization_header(&self) -> &str {
        self.authorization.expose()
    }

    pub fn apply_to(&self, headers: &mut BTreeMap<String, String>) {
        headers.insert("Authorization".into(), self.authorization.expose().into());
        if let Some(account_id) = &self.chatgpt_account_id {
            headers.insert("ChatGPT-Account-ID".into(), account_id.clone());
        }
    }
}

pub(crate) fn resolve_auth(
    provider: ModelProvider,
    id: &CredentialId,
    credentials: &CredentialsConfig,
) -> ModelRequestAuth {
    let credential = credentials.get(id).expect("validated credential");
    match (provider, credential) {
        (ModelProvider::OpenAiReady, Credential::ApiKey { api_key }) => ModelRequestAuth {
            authorization: bearer(api_key.expose()),
            chatgpt_account_id: None,
        },
        (
            ModelProvider::OpenAiCodex,
            Credential::ChatGptOAuth {
                access_token,
                account_id,
                ..
            },
        ) => ModelRequestAuth {
            authorization: bearer(access_token.expose()),
            chatgpt_account_id: account_id.clone(),
        },
        _ => unreachable!("validated credential kind"),
    }
}

pub(crate) fn validate_credentials(
    config: &ModelSwitchboardConfig,
    credentials: &CredentialsConfig,
) -> Result<(), CredentialError> {
    for (provider, id) in config
        .general
        .iter()
        .map(|endpoint| (endpoint.provider, &endpoint.credential_id))
        .chain(
            config
                .insomnia
                .iter()
                .map(|endpoint| (endpoint.provider, &endpoint.credential_id)),
        )
        .chain(
            config
                .insomnia_metadata
                .iter()
                .map(|endpoint| (endpoint.provider, &endpoint.credential_id)),
        )
        .chain(
            config
                .chronos
                .iter()
                .map(|endpoint| (endpoint.provider, &endpoint.credential_id)),
        )
        .chain(
            config
                .dream
                .iter()
                .map(|endpoint| (endpoint.provider, &endpoint.credential_id)),
        )
        .chain(
            config
                .embedding
                .iter()
                .map(|endpoint| (endpoint.provider, &endpoint.credential_id)),
        )
    {
        let credential = credentials
            .get(id)
            .ok_or_else(|| CredentialError::MissingCredential(id.as_str().into()))?;
        if credential.auth_kind() != provider.auth_kind() {
            return Err(CredentialError::WrongAuthKind(id.as_str().into()));
        }
    }
    Ok(())
}

fn bearer(token: &str) -> SecretString {
    SecretString::new(format!("Bearer {token}")).expect("non-empty bearer token")
}
