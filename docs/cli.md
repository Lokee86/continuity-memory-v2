# Repo-local CLI

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns the development CLI package boundary, invocation model, and currently exposed operator commands.

## Overview

`cli/` is a separate Cargo package named `continuity-cli`. It is intentionally not a binary target of the core `continuity-memory` package and is not installed by repository builds or tests.

Run it directly from the repository:

```text
cargo run --manifest-path cli/Cargo.toml -- --help
```

The package depends only on the public `continuity-memory` Rust API through a path dependency. Deleting or moving `cli/` does not change the core library package. A detached copy only needs its dependency source changed from `path = ".."` to the desired Git/package source.

## Command surface

```text
continuity
├── cva
│   ├── create
│   ├── info
│   └── verify
├── import
│   └── graph-jsonl
├── archive
│   ├── conversations
│   ├── show
│   ├── fragments
│   └── fragment
├── config
│   ├── show
│   ├── verify
│   ├── credential ...
│   └── model ...
├── vectors
│   ├── status
│   ├── profiles
│   ├── probe
│   └── build
└── dev
    ├── establish-profile
    ├── build-vectors
    └── search
```

`vectors probe` and `vectors build` use the configured live `openai-ready` embedding route. `build` establishes/reuses the compatibility profile, embeds the Archive, and publishes one generation. `dev` commands deliberately remain on `SimulatedEmbeddingEndpoint` for deterministic development work.

## Configuration and credentials

The global `--config` option defaults to `continuity.cfg` in the current directory. This is a CLI working-directory default, not a selected operating-system application-config location.

`config credential add-api-key <id>` prompts for the API key without terminal echo. `--stdin` is available for automation and tests. Secrets are not accepted as command-line arguments so they are not placed directly into shell history/process arguments.

`config credential add-codex-tokens` is a development bridge for already-obtained ChatGPT OAuth material. The CLI does not yet perform Codex device-code login or token refresh.

`config model set-general` and `set-embedding` persist route selection and require the referenced credential to make the resulting switchboard executable. `config verify` performs the same route/credential compatibility validation without sending network requests.

## CVA and import behavior

`cva verify` performs a normal `Cva::open`, so the same format/reopen/reference validation used by the library is exercised.

`import graph-jsonl` accepts the current development graph JSONL records used by the corpus smoke harness. It creates the target CVA when absent or appends idempotent/compatible records to an existing CVA, then materializes branch fragments with the requested window/overlap policy.

Archive inspection is read-only. `archive conversations` uses the public current-branch inventory; `archive show` resolves one current branch; fragment commands list or expand durable fragments.

## Verification

The CLI package has its own lockfile and must be checked separately from the library package:

```text
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo check --manifest-path cli/Cargo.toml
cargo test --manifest-path cli/Cargo.toml
```

Repository verification also performs command-level smoke tests for CVA create/info/verify, graph import/archive inspection, simulated profile/vector/search execution, and config show/verify without installing the binary.

## Related docs

- [Architecture](architecture.md)
- [Rust API](api.md)
- [Local configuration](configuration.md)
- [Development](development.md)
- [ADR 0011](decisions/0011-detachable-repo-local-cli.md)

## Notes

The CLI owns no HTTP protocol semantics; live embedding is supplied by the core `OpenAiReadyEmbeddingEndpoint`. General-model transport and the shared long-lived runtime remain future boundaries.