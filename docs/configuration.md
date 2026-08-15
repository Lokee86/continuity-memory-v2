# Local Configuration

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns the local Continuity configuration format, mutation semantics, and boundary from CVA semantic state.

## Overview

Continuity uses one small purpose-built `continuity.cfg` file for machine-local current configuration. It uses replaceable logical objects, whole-file atomic replacement, and no internal history.

## Ownership

`continuity.cfg` is a purpose-built local application-configuration file. It is not part of a `.cva`, does not participate in CVA semantic clocks, and has no historical or append-only semantics.

Configuration is current-state only:

```text
load current objects
    ↓
replace typed values in memory
    ↓
serialize one complete current file image
    ↓
write + sync temporary file
    ↓
atomically replace continuity.cfg
```

If a user wants configuration history, that belongs in an external version-control system such as Git.

## File header

All multi-byte values are little-endian.

```text
8 bytes   magic = "CVCFG\0\r\n"
u16       major = 1
u16       minor = 0
u32       object count
```

## Object framing

Each logical configuration object is independently framed:

```text
u16       UTF-8 key byte length
u16       object schema version
u32       flags
u32       payload byte length
u32       reserved = 0
N bytes   UTF-8 logical key
M bytes   object payload
```

Logical keys are stable names rather than content hashes. Replacing a setting replaces the object associated with that key in the next complete file image; superseded objects are not retained.

Unknown logical objects are preserved byte-for-byte when a newer/foreign object is present and known objects are changed. This lets the object vocabulary expand without requiring config history or an append-only store.

Flags are currently `0`. The field is reserved so later credential or other sensitive objects can mark encrypted payloads without encrypting the entire configuration file.

## Implemented objects

### `archive.fragments`

Schema `1`, flags `0`:

```text
u64       turns
u64       overlap
```

Current defaults are `turns = 8`, `overlap = 2`. `turns` must be non-zero and `overlap < turns`.

### `retrieval.default`

Schema `1`, flags `0`:

```text
u64       candidate_limit
u64       result_limit
f64       lexical_weight
f64       semantic_weight
```

Current defaults are `30`, `10`, `0.45`, and `0.55`. Limits must be non-zero, result limit cannot exceed candidate limit, and candidate limit cannot exceed the semantic-search maximum. Weights must be finite and positive; search normalizes them before hybrid fusion.

`Cva::search_with_config` consumes `RetrievalConfig`. `Cva::search` remains the default-policy convenience method.

### `models.general`

Schema `1`, flags `0`:

```text
u8        provider
3 bytes   reserved = 0
string    model
string    URL; empty for provider-owned routing
```

The general endpoint currently accepts `openai-codex` or `openai-ready`. `openai-codex` uses provider-owned routing and therefore stores no URL. `openai-ready` requires an explicit `http://` or `https://` endpoint URL.

### `models.embedding`

Schema `1`, flags `0`:

```text
u8        provider
u8        normalization: 0=None, 1=L2
2 bytes   reserved = 0
u32       dimensions
string    model
string    URL
```

The embedding endpoint currently accepts `openai-ready` only, requires non-zero dimensions, and requires an explicit endpoint URL. The current provider tags are `1=openai-codex` and `2=openai-ready`.

`ModelSwitchboardConfig` owns the two optional endpoint selections. `ModelSwitchboard` validates the configured capability boundary and exposes the selected general and embedding endpoints to the runtime. Provider authentication is explicit: `openai-codex` is reserved for ChatGPT device-code auth and `openai-ready` for API-key auth. Credential persistence and transport are separate implementation slices.

## Replacement and durability

`ContinuityConfig::save` validates all known objects before touching the existing file. It writes the new image to a temporary file in the same directory, flushes it, then performs a replace operation. Windows uses `MoveFileExW` with replace/write-through flags; Unix uses same-filesystem rename and synchronizes the parent directory.

Repeated replacement does not accumulate superseded configuration objects or require compaction.

## Security boundary

A locally generated 256-bit `MasterKey` is implemented. `ContinuityConfig::load_or_create_master_key()` currently persists it beside the config as `continuity.master-key.json`:

```text
continuity.cfg
continuity.master-key.json
    version: 1
    master_key_hex: 64 hex characters
```

Generation uses operating-system entropy (`BCryptGenRandom` on Windows and `/dev/urandom` on Unix). The key is created once and reused; normal load-or-create behavior never overwrites an existing key, and `Debug` output redacts the key material.

The JSON store is explicitly temporary development plumbing, not the final security boundary. Because the master key is plaintext on disk, it provides no meaningful at-rest protection against an actor who can read both files. The intended production replacement remains the operating-system credential store while encrypted credential payloads remain in `continuity.cfg`.

The whole config file does not need encryption. Object-level encryption will permit ordinary settings to remain inexpensive and inspectable while secrets receive authenticated encryption.

## Current limitations

- The default operating-system config location is not selected yet; callers currently supply a path.
- General and embedding switchboard objects are implemented, but direct HTTP transport is not wired yet.
- `openai-codex` device-code execution and `openai-ready` API-key credential objects are not implemented yet.
- Credential-payload encryption is not implemented yet.
- The master key currently lives in temporary plaintext JSON; Windows Credential Manager integration is not implemented yet.
- No import/export text format exists yet.

## Related docs

- [Architecture](architecture.md)
- [Rust API](api.md)
- [Current limitations](current-limitations.md)
- [ADR 0008](decisions/0008-purpose-built-local-configuration.md)
- [ADR 0009](decisions/0009-expandable-model-switchboard.md)

## Notes

The config object vocabulary is intentionally concrete. Shared framing does not make the file a generalized semantic store.
