# ADR-0003: Layered version clocks and conversation-local ancestry

Status: Accepted
Date: 2026-08-14
Owners: CVA container ordering; Archive local ordering and conversation/session history
Supersedes: [ADR 0002](0002-branching-publication-history.md)
Superseded by: none

## Context

Archive-wide parent-linked publications removed cross-database coupling but still treated every Archive mutation as one successor chain. Concurrent conversations therefore shared a semantic head despite having independent histories. Reviving an old conversation also naturally wants to branch that conversation, not rewind the entire Archive.

We still need exact whole-CVA ordering and exact whole-Archive historical cuts for inspection, checkpointing, and future recovery.

## Decision

Use three distinct levels:

```text
CVA global_version: u64
Archive archive_version: u64
conversation/session ancestry
```

Each semantic Archive mutation records both integers and its semantic `ChunkRef`. The Archive integer is contiguous and identifies a whole-Archive watermark; it has no parent pointer.

Conversation ancestry remains in immutable node `parent_id` links. Logical branch/session heads are append-only revisions of `(conversation_id, branch_id)`. Reopening an old conversation point creates/advances another local branch while Archive and global clocks continue forward.

No Archive-wide semantic publication head exists.

## Consequences

### Ownership and dependencies

- Container owns global ordering only.
- Archive owns Archive-local ordering and visibility semantics.
- Conversation/session ancestry belongs to Archive nodes and branch revisions.
- Future databases may adopt local watermarks only if their concrete storage needs justify them.
- No store may treat another store's local watermark as semantic dependency identity.

### State, lifecycle, and operations

- Unrelated conversations can advance independently at the semantic level.
- The single physical CVA still needs a narrow append/version ordering boundary when concurrent writers arrive.
- Archive historical cuts remain deterministic by replay through `A=N`.
- Branch/session revisions preserve old heads without copying conversations.
- Whole-CVA restore-and-continue is deferred until multiple concrete databases make the required timeline semantics clear.

### Compatibility and migration

- Archive format marker changes from `CVAAFMT1` to `CVAAFMT2`.
- `CVAAPUB1` and `CVAASEL1` are retired development records.
- Current metadata is `CVAAREC1 { global_version, archive_version, ChunkRef }`.
- Pre-change development CVAs are rejected rather than silently reinterpreted.

## Alternatives considered

- **One Archive-wide publication parent chain:** rejected because unrelated sessions acquire false ancestry and a shared semantic serialization point.
- **Only a global clock:** rejected because Archive-owned checkpoints/historical cuts benefit from a dense local watermark independent of other database activity.
- **Per-conversation clocks only:** rejected because the Archive also needs an exact whole-database mutation watermark.
- **Mutable branch heads in place:** rejected because retained history and crash/recovery reasoning are simpler with append-only revisions.

## Verification

Protected by tests for independent conversation ancestry, divergent global/Archive clocks, Archive-version reopen, append-only branch revisions with historical lookup, and old-conversation revival through a new local branch.

## Risks and debt

- The finite Insomnia drain now uses read-only Archive sharing, independent Memories/Insomnia store locks, indexed operational scheduling, and a narrow Container I/O/version lock. A general long-lived arbitrary concurrent-writer runtime is still not implemented.
- Generic whole-Archive historical view materialization is not exposed yet.
- Whole-CVA restore-and-continue timeline semantics remain open.
- Retention/vacuum of old branch revisions is undefined.

## References

- [Architecture](../architecture.md)
- [Storage format](../storage-format.md)
- [Versioning plan](../version-history-plan.md)
- [Behavioral contracts](../behavioral-contracts.md)