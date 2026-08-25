# Reliquary CLI

Repo-local bring-up CLI for Reliquary Memory v2.

This is a separate Cargo package, not a binary target of the `reliquary-memory` library package and not installed by default. It depends only on the library's public Rust API through `reliquary-memory = { path = ".." }`.

Run it from the repository without installing it:

```text
cargo run --manifest-path cli/Cargo.toml -- --help
```

The package can be removed or moved without changing the core library. If detached from this repository, replace the path dependency in `cli/Cargo.toml` with the desired Git or package source.

## Current command groups

- `cva`: create, inspect, and verify CVAs.
- `import graph-jsonl`: ingest the current graph JSONL development corpus format.
- `archive`: list conversations/branches, show branch turns, and inspect fragments.
- `config`: inspect/verify config, manage encrypted credentials, and select General/Insomnia/Embedding routes. Insomnia falls back to General when unset.
- `vectors`: inspect profiles/generations, probe the configured live embedding endpoint, and build real Archive vector generations.
- `insomnia run`: register canonical import Episodes, drain the whole Insomnia backlog with configurable worker concurrency, and fill missing Memory Vectors.
- `dev`: exercise compatibility profiles, vector generation, and retrieval through the deterministic simulated embedding endpoint.

Direct `openai-ready` embedding and General-model transport are implemented. `vectors probe` tests the configured live embedding endpoint and `vectors build` establishes/reuses its compatibility profile and publishes a real Archive vector generation. Insomnia can use its own General-model route or fall back to `models.general`. `insomnia run <cva> --workers 16` is the current finite whole-file bring-up path; the core drain automatically fills missing Memory Vectors after authoritative extraction, and worker concurrency is independent from embedding batch/concurrency settings.

## Credentials

API keys are prompted without terminal echo by default:

```text
cargo run --manifest-path cli/Cargo.toml -- \
  --config ./reliquary.cfg \
  config credential add-api-key ready
```

For automation/tests, `--stdin` reads the first line from standard input instead. Passing secrets directly as command-line arguments is intentionally unsupported.

ChatGPT/Codex authentication uses device codes rather than pasted tokens:

```text
cargo run --manifest-path cli/Cargo.toml -- \
  --config ./reliquary.cfg \
  config credential login-codex codex
```

The command prints the OpenAI verification URL and one-time code, waits for authorization, then stores the returned OAuth credential encrypted in `reliquary.cfg`.
