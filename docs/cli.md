# Repo-local CLI

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns the development CLI package boundary, invocation model, and currently exposed operator commands.

## Overview

`cli/` is a separate Cargo package named `reliquary-cli`. It is intentionally not a binary target of the core `reliquary-memory` package and is not installed by repository builds or tests.

Run it directly from the repository:

```text
cargo run --manifest-path cli/Cargo.toml -- --help
```

The package depends only on the public `reliquary-memory` Rust API through a path dependency. Deleting or moving `cli/` does not change the core library package. A detached copy only needs its dependency source changed from `path = ".."` to the desired Git/package source.

## Command surface

```text
reliquary
├── rel
│   ├── create [--scope project|organization|connection]
│   ├── info
│   └── verify
├── phy
│   ├── create
│   ├── info
│   └── verify
├── migrate <source> <output>
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
├── insomnia
│   └── run
└── dev
    ├── establish-profile
    ├── build-vectors
    └── search
```

`vectors probe` and `vectors build` use the configured live `openai-ready` embedding route. `build` establishes/reuses the compatibility profile, embeds the Archive, and publishes one generation. `dev` commands deliberately remain on `SimulatedEmbeddingEndpoint` for deterministic development work.

`insomnia run <rel>` is the whole-file bring-up surface. Unless `--existing-queue-only` is supplied, it materializes/queues uncovered canonical import paths first. Without `--phy`, it remains Project-only and invokes the ordinary core drain. `--phy <phy>` opens an explicit user-global Phylactery, enables the dedicated `user | project` ownership classifier, routes User-owned Memories into that PHY, and records owner-qualified external Memory references in the REL completion receipt. Project-owned Memories remain in the REL. The command uses `--workers` concurrent processors (default 48, maximum 64), the dedicated Insomnia model route or General fallback, and the configured embedding route. The core drain automatically establishes/reuses compatibility profiles and embeds only Memory bodies still missing in each participating owner. `--embedding-batch-size` and `--embedding-concurrency` control that automatic derived-vector phase independently from Insomnia worker concurrency.

## Configuration and credentials

The global `--config` option defaults to `reliquary.cfg` in the current directory. This is a CLI working-directory default, not a selected operating-system application-config location.

`config credential add-api-key <id>` prompts for the API key without terminal echo. `--stdin` is available for automation and tests. Secrets are not accepted as command-line arguments so they are not placed directly into shell history/process arguments.

`config credential login-codex <id>` performs the ChatGPT/Codex device-code flow directly. It requests a one-time code, prints the verification URL and code, waits for authorization, exchanges the resulting authorization code for OAuth ID/access/refresh tokens, extracts the ChatGPT account ID, and saves the credential under the requested ID. Manual token copy/paste is not part of the CLI. OAuth token refresh is not implemented yet.

`config model set-general`, `set-insomnia`, and `set-dream` persist General-model route selection and accept `--reasoning` for `openai-codex` routes. For example, `config model set-general --provider openai-codex --model gpt-5.6-luna --credential codex --reasoning low` selects Luna at low reasoning; if `models.insomnia` is unset, Insomnia inherits that General route. `set-embedding` persists the separate embedding route. `config verify` performs route/credential compatibility validation without sending network requests.

## REL, PHY, and import behavior

`rel create` creates a typed Project REL by default; `--scope organization`, `--scope project`, and `--scope connection` select the authoritative internal Reliquary scope kind. `rel info` reports the stored scope and whether the file is a legacy CVA physical form. `rel verify` performs a normal `Reliquary::open`, so the same format/reopen/reference validation used by the library is exercised. The old `cva` command name remains a compatibility alias for `rel`.

`phy create <path>` creates a typed user-global Phylactery with no Reliquary scope. `phy info` reports `kind: phylactery` plus Memory, Graph, packed-vector, Memory-vector, and compatibility-profile state. `phy verify` performs a normal `Phylactery::open`, including exact file-kind validation, source-independent Memory validation, Graph endpoint/global-version validation, and vector/profile reference validation. PHY has no `--scope` option.

`migrate <source> <output>` auto-detects a legacy 16-byte Project CVA or earlier 24-byte typed REL/PHY and semantically repacks it into a new 40-byte identified file. The source is never replaced in place. When an old REL contains `WorkspaceMetadata.id`, migration derives the new UUID deterministically from that ID so independently diverged copies retain the same logical owner identity; otherwise a new UUID is generated. Legacy WorkspaceMetadata itself is not copied into the output.

`import graph-jsonl` accepts the current generic development graph JSONL records used by the corpus smoke harness. The format is not ChatGPT-specific; any source can use it after normalization into node/branch records. It creates the target Project REL when absent or appends idempotent/compatible records to an existing Reliquary (including a legacy CVA), then materializes branch fragments with the requested window/overlap policy. Node records may include an optional `attachments` array. Each attachment has `path`, optional `filename`, and optional `mime_type`; relative paths resolve from the JSONL file's directory. The importer reads the bytes and submits the node plus all attachments through `Cva::ingest_turn`, so embedding the files and recording their source-turn provenance is one core ingestion operation rather than importer-side coordination.

The CLI does not expose a single-turn live-ingestion command, file-management commands, or a persistent capture service. The graph importer is currently the only CLI surface that drives turn ingestion.

Archive inspection is read-only. `archive conversations` uses the public current-branch inventory; `archive show` resolves one current branch; fragment commands list or expand durable fragments.

## Verification

The CLI package has its own lockfile and must be checked separately from the library package:

```text
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo check --manifest-path cli/Cargo.toml
cargo test --manifest-path cli/Cargo.toml
```

Repository verification also performs command-level smoke tests for REL create/info/verify and PHY create/info/verify, graph import/archive inspection, simulated profile/vector/search execution, and config show/verify without installing the binary.

## Related docs

- [Architecture](architecture.md)
- [Rust API](api.md)
- [Local configuration](configuration.md)
- [Development](development.md)
- [ADR 0011](decisions/0011-detachable-repo-local-cli.md)

## Notes

The CLI owns no HTTP protocol semantics; live embedding and General-model execution are supplied by the core provider endpoints and `ConfiguredGeneralEndpoint` dispatch. The CLI's Insomnia worker is a finite bring-up/drain command. Product/runtime and management work is tracked in [Roadmap](roadmap.md).