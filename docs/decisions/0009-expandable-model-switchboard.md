# ADR 0009: Expandable model switchboard

Status: **Accepted**

Date: 2026-08-14

## Context

Reliquary needs one runtime boundary for model selection without making CVA compatibility profiles depend on provider/model labels. The first concrete needs are one general-purpose model endpoint and one embedding endpoint.

Provider integration also needs distinct authentication mechanics. `openai-codex` is intended to use ChatGPT device-code authentication, while `openai-ready` is the generic OpenAI-compatible/API-key path.

## Decision

Reliquary will use a capability-oriented model switchboard. The initial capabilities are `General` and `Embedding`; later capabilities are added explicitly rather than encoded into one provider-specific abstraction.

The initial providers are:

- `openai-codex`: general capability only, ChatGPT device-code authentication, provider-owned endpoint routing;
- `openai-ready`: general and embedding capabilities, API-key authentication, explicit endpoint URLs.

Persistent selections live in separate replaceable config objects:

```text
models.general
models.embedding
```

The embedding selection additionally records dimensions and normalization because the current embedding capability contract requires those values. These settings describe runtime routing only. They do not prove vector compatibility and are not stored in Compatibility Profiles.

`ModelSwitchboard` validates capability/provider combinations and exposes selected endpoint configuration. Direct HTTP transport, device-code execution, credential encryption, and credential-store integration remain separate implementation slices.

## Consequences

- Adding another model capability does not require converting the switchboard into a generalized provider framework.
- Adding another provider requires explicit capability/auth support.
- `openai-codex` cannot accidentally be selected as an embedding provider while that capability is unsupported.
- OpenAI-compatible endpoints can point at arbitrary HTTP(S) services without changing CVA semantics.
- Provider/model/URL configuration remains machine-local and replaceable.
- Compatibility Profiles remain behavioral vector-space contracts independent of provider configuration.

## Rejected alternatives

### Provider identity as vector compatibility

Rejected. Provider/model labels are routing metadata and cannot establish compatibility with existing vector generations.

### One undifferentiated model endpoint

Rejected because general inference and embeddings have different capability contracts and may use different providers.

### Codex CLI subprocess integration

Rejected as the runtime/provider boundary. Codex authentication and requests belong inside Reliquary rather than depending on an installed CLI process.

## References

- [Architecture](../architecture.md)
- [Local configuration](../configuration.md)
- [ADR 0007](0007-compatibility-profiles-and-vector-generations.md)
