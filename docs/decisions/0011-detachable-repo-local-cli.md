# ADR 0011: Detachable repo-local CLI

- Status: **Accepted**
- Date: 2026-08-15

## Context

Reliquary needs an operator-facing bring-up interface for CVA lifecycle, configuration/auth, archive inspection, and current vector/retrieval machinery. The core crate is a library and should not acquire product-CLI ownership merely to support development access.

The CLI also should not be installed system-wide during current bring-up work.

## Decision

The CLI lives in `cli/` as a separate Cargo package named `reliquary-cli` with a `reliquary` binary.

It is not a binary target or workspace member of the root `reliquary-memory` package. Repository-local use is through `cargo run --manifest-path cli/Cargo.toml -- ...`.

The package may depend only on the public `reliquary-memory` API. It may not reach into private library modules or duplicate semantic authority. If the CLI needs a generally useful read operation, that operation belongs in the public library API first.

The package owns argument parsing, terminal prompting, human-readable output, and operator command composition only. CVA semantics, configuration persistence/encryption, provider-auth semantics, and retrieval remain owned by the library.

The CLI is detachable: deleting it leaves the library package/build unchanged; moving it requires only replacing its local path dependency with another library source.

Secrets are not accepted as direct command-line values. Interactive secrets use hidden prompts; automation may use standard input.

## Consequences

Core `cargo build/test` does not build or install the CLI. The CLI has its own lockfile and verification commands.

Configured runtime composition is library-owned. `ConfiguredRuntime` resolves configured model/embedding routes and executes finite Insomnia and vector workflows; the CLI supplies operator options and renders the resulting reports. Ownership classification fallback (`insomnia_metadata` then `insomnia`) is likewise a `ModelSwitchboard` rule rather than CLI policy.

`vectors probe` and `vectors build` use live provider execution through that library runtime surface. `dev search` remains on the simulated embedding endpoint for deterministic bring-up, while candidate sizing for a requested result limit is library-owned through `Cva::search_with_result_limit`.

A CLI boundary regression test rejects direct command-module dependencies on runtime-composition and retrieval-policy types. The public library also exposes current branch inventory so a detached operator can enumerate conversations/branches without private-index access.

## Alternatives rejected

- Adding `src/main.rs` or a root `[[bin]]`: couples bring-up CLI ownership to the core package.
- Installing a global development binary: unnecessary and contrary to the current bring-up preference.
- Having the CLI parse CVA/config formats directly: duplicates semantic/storage ownership and prevents clean detachment.

## Related docs

- [Repo-local CLI](../cli.md)
- [Architecture](../architecture.md)
- [Rust API](../api.md)
- [Development](../development.md)
