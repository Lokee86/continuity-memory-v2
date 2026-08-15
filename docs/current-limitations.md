# Current Limitations

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns known incomplete, transitional, or practically limiting behavior in the current rebuild.

## Overview

Archive now has layered global/Archive ordering and conversation-local branch ancestry; the CVA also contains immutable packed-vector matrices and Archive-Vector row bindings. Embedding-profile/generation semantics and retrieval remain incomplete.

## Storage limits

- Archive, immutable packed-vector backing objects, and Archive-Vector row-to-fragment bindings are implemented; embedding profiles, vector generations, Memories, and Graph are not.
- Chunks are uncompressed and lack container-level checksum/authentication/encryption; packed-vector objects do verify their own SHA-256 content identity.
- No object packing, compaction, vacuum, reachability, or reclamation exists.
- No persistent snapshot/checkpoint acceleration exists.
- No concurrent-writer/locking model exists beyond one `Container` file handle.
- Format migration is not implemented.
- Whole-CVA rollback across multiple semantic databases is not implemented. Packed matrices and Archive-Vector sets are backing data and do not yet create a second semantic timeline.
- A general full historical `ArchiveView` API is not yet exposed, although each Archive cut is durably identified by its Archive version.

## Version/history limits

- Every semantic Archive node, branch revision, or fragment gets one Archive version and one CVA-global version.
- Each such mutation adds 72 bytes of ordering metadata/framing.
- Content-object creation does not independently advance Archive semantic versioning.
- Branch/session history is represented by repeated immutable branch-head revisions plus node ancestry; there is no separate generalized session database yet.
- Whole-Archive historical cuts are linear watermarks. Continuing one old conversation does not branch the Archive; it branches that conversation/session locally.
- Retention/vacuum policy for old branch/session revisions is undefined.

## Indexing and memory limits

Composite `HashMap<String, ...>` lookup keys have been removed. Nodes, current branch heads, and fragments now use dense record vectors plus compact open-addressed hash-to-index slots; exact keys are checked against the record itself. `ContentId -> ChunkRef` remains a direct fixed-width hash table because measurement showed that representation is smaller than an indirect record-plus-index layout for this key/value pair.

Remaining Archive record strings are still individually allocated; conversation/role/string interning has not been attempted.

Reopen uses one streaming physical pass: Container validates chunk framing/global tickets while `Cva` feeds the same payloads to Archive, packed-vector, and Archive-Vector rebuild. Current measurements are recorded in `development.md`. Packed-vector matrix bytes and Archive-Vector mappings are not retained in steady-state reopen indexes, but the scanner still materializes each physical chunk transiently; a very large single object can therefore create a large peak allocation.

## Retrieval limits

Fragments, generic packed matrices, and row-to-Archive-fragment bindings exist. Embedding profiles, semantic vector generations, lexical search, exact similarity search, hybrid ranking, and retrieval control are not implemented here yet.

## Product/runtime limits

No shared long-lived runtime, Insomnia, Dream, Ego, production importer, or durable operational worker state is implemented in this repository yet.

## Enforcement limits

Repository-local Pitlord policy has not yet been added.

## Related docs

- [Roadmap](roadmap.md)
- [Architecture](architecture.md)
- [Storage format](storage-format.md)
- [Versioning and rollback plan](version-history-plan.md)

## Notes

These limits should change in the same implementation change that removes them.