# Getting Started

Parent index: [Reliquary operator manual](INDEX.md)

## Purpose

Get a new REL/PHY created, verified, and ready for development or integration without first reading the architecture documents.

## Overview

Reliquary is a Rust library/engine used by Warlock. It is **not** currently a standalone end-user application. The repository also contains a detachable development/operator CLI.

A Reliquary file is owner-local semantic state. Current file forms are:

- `.prj.rel` — Project Reliquary.
- `.org.rel` — Organization Reliquary.
- `.con.rel` — Connection Reliquary.
- `.phy` — user-global Phylactery.
- legacy `.cva` — readable/migratable legacy Project Reliquary.

### 1. Verify the repository

For ordinary library work:

```text
cargo fmt --check
cargo check
cargo test
```

The CLI is a separate Cargo package and has separate checks; see [Repo-local CLI](../cli.md).

### 2. Inspect the CLI

Run the repo-local CLI without installing it:

```text
cargo run --manifest-path cli/Cargo.toml -- --help
```

The CLI defaults to `reliquary.cfg` in the current working directory unless `--config` is supplied.

### 3. Create a REL

```text
cargo run --manifest-path cli/Cargo.toml -- rel create <path>
cargo run --manifest-path cli/Cargo.toml -- rel info <path>
cargo run --manifest-path cli/Cargo.toml -- rel verify <path>
```

Use `--type LABEL` only when you want a descriptive organization/type label. It does not change REL behavior.

### 4. Create a PHY when user-global Memory is required

```text
cargo run --manifest-path cli/Cargo.toml -- phy create <path>
cargo run --manifest-path cli/Cargo.toml -- phy info <path>
cargo run --manifest-path cli/Cargo.toml -- phy verify <path>
```

A PHY is not a REL with a different extension. It has separate ownership and provenance rules.

### 5. Configure model/vector routes only when the workflow needs them

Use `config show` and `config verify` before running configured model work. Insomnia and live vector construction need configured routes; basic file creation, reopen, verification, and many deterministic operations do not.

### 6. Put data into the REL

Current CLI ingestion is development-oriented. `import graph-jsonl` is the available bulk graph/turn-import path. Live product ingestion should normally go through the library/runtime APIs rather than inventing CLI-side ownership logic.

### 7. Flush durable work

Library callers should explicitly call `sync()` at the durability boundaries owned by their application. The runtime/finite workflow surfaces perform their documented syncs.

## What not to expect yet

The CLI does not currently provide a full interactive chat surface, a single-turn live-ingestion command, the final Warlock Knowledge UI, or a standalone Perception drain command. In the long-lived runtime host, however, post-Dream Entity pass 1 is automatic for both the active REL and attached PHY: extracted mentions are scheduled through Admission/V4 and persisted as Entity/Graph/resolution state.

## Related docs

- [Repo-local CLI](../cli.md)
- [Development](../development.md)
- [Local configuration](../configuration.md)
- [Integration guide](integration-guide.md)

## Notes

Use the CLI for operator/development workflows. Product integrations should prefer the library and runtime-host boundaries described in the integration guide.
