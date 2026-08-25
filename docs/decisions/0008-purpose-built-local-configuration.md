# ADR 0008: Purpose-built replaceable local configuration

Status: **Accepted**

Date: 2026-08-14

## Context

Reliquary needs machine-local configuration for fragment/retrieval policy, the planned model switchboard, provider metadata, and credentials. A generalized embedded database such as SQLite would solve persistence, but it would add a database dependency for a tiny bounded settings domain and complicate the product's no-generalized-database architecture story.

Configuration also has different semantics from CVA data. A changed setting simply replaces the current setting. Reliquary does not need configuration publication history, rollback generations, append-only revisions, or internal source control.

## Decision

Reliquary will use a purpose-built local configuration file with a magic header, format version, and independently framed logical objects.

Objects use stable logical keys, not content-addressed identities. Saving configuration writes one complete current file image to a temporary file and atomically replaces the previous file. Superseded objects are not retained.

The configuration file is separate from `.cva` semantic state and consumes no Archive, vector, or CVA-global version tickets.

The object frame reserves flags for later per-object encryption. Credentials will eventually be encrypted individually with a locally generated master key held by the operating-system credential store; whole-file encryption is not required.

## Consequences

- Configuration remains small, deterministic, and fast to load as one bounded file.
- Repeated changes do not create dead history or require compaction.
- New logical object types can be added without introducing a relational schema or generalized database.
- Unknown objects can be preserved across rewrites.
- Configuration history is intentionally delegated to external tools such as Git when users want it.
- Model-switchboard and credential records can be added as concrete object types later.
- The system must own atomic whole-file replacement and format validation itself.

## Rejected alternatives

### SQLite

Rejected for current configuration because the domain does not need relational queries, indexes, joins, transactions across large mutable sets, or database history. Its runtime/schema overhead would exceed the needs of the settings domain and weaken the no-generalized-database claim.

### TOML as authoritative storage

Rejected as the primary store because Reliquary will need encrypted credential payloads and GUI/runtime mutation. A text format can still be offered later as import/export without becoming a second source of truth.

### Append-only/CAM history

Rejected for configuration. Content addressing and publication history are useful where immutable semantic history matters; configuration is current state and is replaced directly.

## References

- [Local configuration](../configuration.md)
- [Architecture](../architecture.md)
- [Architectural invariants](../invariants.md)
