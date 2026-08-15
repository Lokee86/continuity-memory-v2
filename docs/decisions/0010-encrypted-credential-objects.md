# ADR 0010: Encrypted credential objects

Status: **Accepted**

Date: 2026-08-14

## Context

Model routes need usable authentication without putting API keys or OAuth tokens in plaintext configuration. The switchboard already distinguishes `openai-ready` API-key auth from `openai-codex` ChatGPT device-code auth, and a machine-local 256-bit master key already exists.

## Decision

Credentials are replaceable current-state objects in `continuity.cfg` under stable logical keys:

```text
credential.<id>
```

Each credential object uses schema `1`, encrypted-object flag `1`, and AES-256-GCM. A fresh 96-bit nonce is generated for every credential encryption. Authenticated additional data binds the ciphertext to the exact logical object key so ciphertext cannot be moved between credential IDs without failing authentication.

Credential plaintext schemas are concrete:

- API key: one secret string;
- ChatGPT OAuth: ID token, access token, refresh token, and optional ChatGPT account ID.

Model-route objects store a `CredentialId`. The runtime switchboard validates that the referenced credential exists and that its auth kind matches the selected provider before producing request authentication.

`openai-ready` request auth is `Authorization: Bearer <api-key>`. `openai-codex` request auth is `Authorization: Bearer <access-token>` plus `ChatGPT-Account-ID` when an account ID is available.

Secrets are decrypted only into memory, secret debug output is redacted, and secret strings plus plaintext codec buffers are zeroized when discarded.

## Master-key boundary

Credential ciphertext lives in `continuity.cfg`; the master key does not. The current master-key backend remains the explicitly temporary plaintext `continuity.master-key.json`. Replacing that backend with the operating-system credential store does not change credential-object format or model-route references.

## Consequences

- Ordinary model/retrieval configuration remains inspectable and unencrypted.
- Multiple routes may share one credential or use separate credentials.
- Removing a credential removes its encrypted config object on the next save; no credential history accumulates internally.
- Saving credentials re-encrypts them with fresh nonces, so encrypted bytes are intentionally nondeterministic even when plaintext is unchanged.
- Missing or wrong-kind credentials prevent construction of an executable `ModelSwitchboard` but do not make the underlying config file unreadable.
- ChatGPT device-code acquisition/refresh remains a separate transport/authentication slice; this decision defines persistence and request attachment.

## Rejected alternatives

### Plaintext credentials in `continuity.cfg`

Rejected because provider secrets should not be exposed merely to keep config implementation simple.

### Whole-file encryption

Rejected because most configuration is non-sensitive and should remain inexpensive and inspectable.

### Provider-specific secret files

Rejected because credential ownership belongs to one config subsystem and model routes should reference credentials uniformly by stable ID.

## References

- [Local configuration](../configuration.md)
- [Architecture](../architecture.md)
- [ADR 0008](0008-purpose-built-local-configuration.md)
- [ADR 0009](0009-expandable-model-switchboard.md)
