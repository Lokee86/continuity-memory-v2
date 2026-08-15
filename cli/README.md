# Continuity CLI

Repo-local bring-up CLI for Continuity Memory v2.

This is a separate Cargo package, not a binary target of the `continuity-memory` library package and not installed by default. It depends only on the library's public Rust API through `continuity-memory = { path = ".." }`.

Run it from the repository without installing it:

```text
cargo run --manifest-path cli/Cargo.toml -- --help
```

The package can be removed or moved without changing the core library. If detached from this repository, replace the path dependency in `cli/Cargo.toml` with the desired Git or package source.

## Current command groups

- `cva`: create, inspect, and verify CVAs.
- `import graph-jsonl`: ingest the current graph JSONL development corpus format.
- `archive`: list conversations/branches, show branch turns, and inspect fragments.
- `config`: inspect/verify config, manage encrypted credentials, and select General/Embedding routes.
- `vectors`: inspect profiles and generation status.
- `dev`: exercise compatibility profiles, vector generation, and retrieval through the deterministic simulated embedding endpoint.

Live provider transport is not implemented yet, so normal model configuration/auth can be entered now but real model requests are not sent by this CLI yet.

## Credentials

API keys are prompted without terminal echo by default:

```text
cargo run --manifest-path cli/Cargo.toml -- \
  --config ./continuity.cfg \
  config credential add-api-key ready
```

For automation/tests, `--stdin` reads the first line from standard input instead. Passing secrets directly as command-line arguments is intentionally unsupported.
