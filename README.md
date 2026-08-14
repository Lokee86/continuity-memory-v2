# Continuity Memory v2

Clean rebuild of Continuity storage/runtime architecture.

This repository starts from one deliberately simple model:

> A `.cva` is a transactional file container holding several purpose-built, cross-indexed databases.

The databases share physical storage, integrity, snapshot publication, and reclamation machinery. They do **not** share a generalized semantic database abstraction.

## Initial database set

- Archive
- Memories
- Graph
- Archive Vectors
- Memory Vectors

Additional databases may be added only as explicit owners of a distinct mutable domain.

## Core rule

The common CVA substrate knows how to store and atomically publish database states. It does not know what those database states mean.

See [docs/architecture.md](docs/architecture.md) and [docs/invariants.md](docs/invariants.md).
