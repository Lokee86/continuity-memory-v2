use crate::config_codec::{decode_config, encode_config};
use crate::{Credential, CredentialId, JsonMasterKeyStore, MasterKeyStore, ReliquaryConfig};
use std::fs;
use std::path::PathBuf;

fn test_path(name: &str) -> PathBuf {
    let unique = uuid::Uuid::new_v4();
    let dir = std::env::temp_dir().join(format!("continuity-credential-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn id(value: &str) -> CredentialId {
    CredentialId::new(value).unwrap()
}

#[test]
fn encrypted_credentials_round_trip_without_plaintext_in_config() {
    let path = test_path("reliquary.cfg");
    let mut config = ReliquaryConfig::new(&path);
    config
        .credentials
        .insert_api_key(id("ready"), "super-secret-api-key")
        .unwrap();
    config
        .credentials
        .insert_chatgpt_oauth(
            id("codex"),
            "secret-id-token",
            "secret-access-token",
            "secret-refresh-token",
            Some("account-123".into()),
        )
        .unwrap();
    config.save().unwrap();

    let bytes = fs::read(&path).unwrap();
    for secret in [
        b"super-secret-api-key".as_slice(),
        b"secret-id-token",
        b"secret-access-token",
        b"secret-refresh-token",
    ] {
        assert!(!bytes.windows(secret.len()).any(|window| window == secret));
    }

    let reopened = ReliquaryConfig::open(&path).unwrap();
    assert_eq!(reopened.credentials, config.credentials);
    assert!(matches!(
        reopened.credentials.get(&id("ready")),
        Some(Credential::ApiKey { .. })
    ));
}

#[test]
fn wrong_master_key_cannot_decrypt_credentials() {
    let path = test_path("reliquary.cfg");
    let mut config = ReliquaryConfig::new(&path);
    config
        .credentials
        .insert_api_key(id("ready"), "super-secret")
        .unwrap();
    config.save().unwrap();

    let replacement_path = path.with_file_name("replacement-key.json");
    JsonMasterKeyStore::new(&replacement_path)
        .load_or_create()
        .unwrap();
    fs::copy(
        replacement_path,
        path.with_file_name("reliquary.master-key.json"),
    )
    .unwrap();

    assert!(ReliquaryConfig::open(&path).is_err());
}

#[test]
fn authenticated_encryption_rejects_tampering() {
    let path = test_path("reliquary.cfg");
    let mut config = ReliquaryConfig::new(&path);
    config
        .credentials
        .insert_api_key(id("ready"), "super-secret")
        .unwrap();
    config.save().unwrap();

    let mut objects = decode_config(&fs::read(&path).unwrap()).unwrap();
    let object = objects.get_mut("credential.ready").unwrap();
    let last = object.payload.len() - 1;
    object.payload[last] ^= 0x01;
    fs::write(&path, encode_config(&objects).unwrap()).unwrap();

    assert!(ReliquaryConfig::open(&path).is_err());
}

#[test]
fn clearing_credentials_removes_encrypted_objects() {
    let path = test_path("reliquary.cfg");
    let mut config = ReliquaryConfig::new(&path);
    config
        .credentials
        .insert_api_key(id("ready"), "super-secret")
        .unwrap();
    config.save().unwrap();
    assert!(
        decode_config(&fs::read(&path).unwrap())
            .unwrap()
            .contains_key("credential.ready")
    );

    config.credentials = crate::CredentialsConfig::default();
    config.save().unwrap();
    assert!(
        !decode_config(&fs::read(&path).unwrap())
            .unwrap()
            .contains_key("credential.ready")
    );
}

#[test]
fn invalid_credential_ids_are_rejected() {
    for value in ["", "has spaces", "slash/name", "é"] {
        assert!(CredentialId::new(value).is_err());
    }
    assert!(CredentialId::new("openrouter.primary").is_ok());
}
