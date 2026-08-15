# ADR 0011: Detachable repo-local CLI

- Status: **Accepted**
- Date: 2026-08-15

## Context

Continuity needs an operator-facing bring-up interface for CVA lifecycle, configuration/auth, archive inspection, and current vector/retrieval machinery. The core crate is a library and should not acquire product-CLI ownership merely to support development access.

The CLI also should not be installed system-wide during current bring-up work.

## Decision

The CLI lives in `cli/` as a separate Cargo package named `continuity-cli` with a `continuity` binary.

It is not a binary target or workspace member of the root `continuity-memory` package. Repository-local use is through `cargo run --manifest-path cli/Cargo.toml -- ...`.

The package may depend only on the public `continuity-memory` API. It may not reach into private library modules or duplicate semantic authority. If the CLI needs a generally useful read operation, that operation belongs in the public library API first.

The package owns argument parsing, terminal prompting, human-readable output, and operator command composition only. CVA semantics, configuration persistence/encryption, provider-auth semantics, and retrieval remain owned by the library.

The CLI is detachable: deleting it leaves the library package/build unchanged; moving it requires only replacing its local path dependency with another library source.

Secrets are not accepted as direct command-line values. Interactive secrets use hidden prompts; automation may use standard input.

## Consequences

Core `cargo build/test` does not build or install the CLI. The CLI has its own lockfile and verification commands.

`vectors probe` and `vectors build` now use live provider execution from the library. The CLI does not yet expose a normal live search command; `dev search` deliberately remains on the simulated embedding endpoint for deterministic bring-up without duplicating provider transport or retrieval semantics.

The public library now exposes current branch inventory so a detached operator can enumerate conversations/branches without private-index access.

## Alternatives rejected

- Adding `src/main.rs` or a root `[[bin]]`: couples bring-up CLI ownership to the core package.
- Installing a global development binary: unnecessary and contrary to the current bring-up preference.
- Having the CLI parse CVA/config formats directly: duplicates semantic/storage ownership and prevents clean detachment.

## Related docs

- [Repo-local CLI](../cli.md)
- [Architecture](../architecture.md)
- [Rust API](../api.md)
- [Development](../development.md)
