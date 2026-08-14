# Architectural Invariants

Parent index: [Documentation index](INDEX.md)

## Purpose

This document defines the governing architectural constraints for the Continuity Memory v2 rebuild.

## Overview

These constraints are intentionally stronger than convenience abstractions. Implementation status remains owned by current architecture, roadmap, and limitations documents.

## Invariants

1. **One file, several databases.** A `.cva` contains explicit purpose-built databases, not one generalized semantic database.
2. **One mutable domain, one owner.** Each independently mutable semantic domain has one concrete owner.
3. **Stable IDs cross database boundaries.** Cross-indexing uses IDs, not shared mutable semantic objects.
4. **The physical substrate has no semantic dependency knowledge.**
5. **No generalized semantic dependency engine.**
6. **Derived state remains derived.** Indexes/checkpoints/caches cannot become authority by persistence alone.
7. **Runtime work state is not semantic authority.**
8. **Prefer concrete duplication over speculative semantic generalization.**
9. **Previous Continuity code has no automatic authority.**
10. **Architecture outranks experimental-format compatibility.**
11. **Archive text has one authority.** Content bytes live in content-addressed objects referenced by nodes.
12. **Conversation ancestry is local.** Node `parent_id` cannot cross conversation IDs.
13. **Concurrent conversations do not acquire semantic ancestry from physical ordering.**
14. **Branch/session heads are revisions, not mutable in-place records.** Existing heads advance only to descendants.
15. **Reviving an old conversation creates a new local branch identity; it does not rewind an existing branch or the Archive.**
16. **Fragment identity is branch-neutral.**
17. **Live fragment materialization is append-only.**
18. **Global version is ordering only.** It is not a CVA root or semantic dependency identity.
19. **Archive version is a local watermark only.** It orders Archive mutations but is not Archive ancestry.
20. **Global and Archive clocks are independent.** Archive versions are contiguous; global versions between Archive records may have gaps.
21. **Normal Archive writes stay Archive-local.** They cannot require reading/republishing unrelated database heads.
22. **Whole-Archive historical state is a cut, not one giant record.** `A=N` identifies records/revisions visible through that watermark.
23. **Checkpointing and historical semantics are distinct.** Checkpoints accelerate reconstruction; clocks/revisions define historical ordering.
24. **Persistent ordering uses exact integers.** No floating-point version identity.

## Safety boundaries

Changing version ownership, conversation ancestry, mutable-record revision semantics, or persistent format requires architecture review and focused behavioral tests.

## Related docs

- [Architecture](architecture.md)
- [Behavioral contracts](behavioral-contracts.md)
- [Versioning and rollback plan](version-history-plan.md)
- [Architectural decisions](decisions/INDEX.md)

## Notes

Cross-database restore activation, actual concurrent writers, checkpointing, and retention/vacuum remain future work.