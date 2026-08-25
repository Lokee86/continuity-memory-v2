# ADR-0001: Purpose-built database ownership inside one CVA

Status: Accepted
Date: 2026-08-14
Owners: CVA physical container and each concrete semantic database
Supersedes: none
Superseded by: none

## Context

Earlier Reliquary storage work generalized mutable domain state through broad roots, manifests, and dependency identities. Local changes could then invalidate unrelated structures, and storage ownership became difficult to reason about.

The rebuild needs one portable `.cva` while retaining explicit authority for Archive, Memories, Graph, and vector domains.

## Decision

A `.cva` is one physical container holding several purpose-built databases. The container owns physical mechanics only. Every mutable semantic domain owns its records, mutation rules, indexes, local derived state, external references, and publication semantics.

Cross-database references use stable logical IDs. The referring database owns the meaning and validation of the reference. The physical container is a non-owner of semantic dependencies and semantic equivalence.

Concrete database implementations precede any shared abstraction. Similar mechanics may be extracted only after multiple owners prove they are physically identical and the extraction does not merge semantic authority.

## Consequences

### Ownership and dependencies

- Archive, Memories, Graph, Archive Vectors, and Memory Vectors are separate semantic owners.
- The container may be depended on by all databases for physical I/O.
- The container cannot depend on domain semantics or maintain a generic semantic dependency graph.
- A shared runtime may coordinate product operations later, but does not merge database authority.

### State, lifecycle, and operations

- Each database defines its own durable records and rebuildable acceleration state.
- Cross-database atomicity, if ever genuinely required by a product operation, must be explicit rather than assumed as the default write model.
- Runtime queues/leases/retries remain operational state rather than hidden semantic authority.

### Compatibility and migration

- Experimental CVAs from discarded architectures are not automatically supported.
- Migration tooling, when added, must translate between explicit authorities rather than preserve obsolete generalized-root semantics.

## Alternatives considered

- **One generalized semantic database/root model:** rejected because it coupled unrelated mutation domains and made invalidation/publication semantics global.
- **Separate physical files for each database:** rejected as the default product model because the CVA must remain one portable archive.
- **Generalized store framework immediately:** rejected until concrete duplicated mechanics demonstrate a justified abstraction.

## Verification

- Current module boundaries separate `Container` from Archive domain codecs and validation.
- [Architectural invariants](../invariants.md) record the ownership rules.
- Repository-local Pitlord policy remains future enforcement work.

## Risks and debt

- Some low-level physical mechanics may initially be duplicated.
- Future contributors may be tempted to recreate a generalized framework as more databases appear; architecture review and Pitlord policy should guard this boundary.

## References

- [Architecture](../architecture.md)
- [Architectural invariants](../invariants.md)
- [Roadmap](../roadmap.md)