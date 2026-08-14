# ADR-0002: Branching local publication history for point-in-time rollback

Status: Superseded
Date: 2026-08-14
Owners: historical decision only
Supersedes: none
Superseded by: [ADR 0003](0003-layered-version-clocks-and-local-ancestry.md)

## Context

This decision attempted to remove the earlier CVA-wide root by giving each semantic database a parent-linked publication history.

## Decision

The accepted design at this point gave every Archive semantic mutation one parent Archive publication and allowed rollback by selecting an older Archive publication as the parent of later writes.

## Consequences

### Ownership and dependencies

This improved on a CVA-wide cross-database root because Archive writes no longer republished other database state.

However, it still imposed one semantic Archive head across every conversation, meaning unrelated conversation mutations became successive parents merely because they shared the Archive database.

### State, lifecycle, and operations

The implementation briefly supported Archive-wide restore/select and branching publication ancestry. Tests proved it mechanically, but concurrent-session analysis exposed the false serialization boundary.

### Compatibility and migration

The corresponding `CVAAFMT1`, `CVAAPUB1`, and `CVAASEL1` development records are no longer current. Current Archive format uses `CVAAFMT2` and layered record-version metadata.

## Alternatives considered

- **Keep one Archive publication chain:** rejected after concurrent-session analysis because it creates a shared semantic head for unrelated conversations.
- **Use local conversation ancestry plus integer watermarks:** adopted by ADR 0003.

## Verification

Historical tests for Archive-wide restore were removed when the design was superseded. Current tests instead protect independent conversations, dual clocks, branch-head revisions, and old-session revival.

## Risks and debt

Whole-CVA restore-and-continue remains unresolved and must not be solved by restoring this Archive-wide semantic chain.

## References

- [ADR 0003](0003-layered-version-clocks-and-local-ancestry.md)
- [Architecture](../architecture.md)
- [Versioning plan](../version-history-plan.md)