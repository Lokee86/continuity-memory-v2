# Current Limitations

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns known incomplete, transitional, or practically limiting behavior in the current rebuild.

## Overview

Archive now has layered global/Archive ordering and conversation-local branch ancestry, but the repository remains bootstrap storage code.

## Storage limits

- Only Archive is implemented.
- Chunks are uncompressed and lack checksum/authentication/encryption.
- No object packing, compaction, vacuum, reachability, or reclamation exists.
- No persistent snapshot/checkpoint acceleration exists.
- No concurrent-writer/locking model exists beyond one `Container` file handle.
- Format migration is not implemented.
- Whole-CVA rollback across multiple databases is not implemented.
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

On the prepared 12-conversation corpus, allocator-tracked retained open heap fell from `1,014,811` bytes to `752,905` bytes, while peak additional heap fell from `1,146,558` bytes to `884,584` bytes. Remaining record strings are still individually allocated; conversation/role/string interning has not been attempted.

Reopen now uses one streaming physical pass: Container validates chunk framing and global version tickets while Archive consumes the same payloads to reconstruct versioned state. On the prepared corpus, median warm-cache open is about `25.3 ms`, with roughly `24.6 ms` in the combined scan/reconstruction path and `3.0 ms` in reference validation in a traced sample. The remaining startup cost is therefore the unavoidable current full-file scan/decode, not duplicate replay. Persistent checkpointing is deferred until larger-Archive measurements justify the added machinery.

## Retrieval limits

Fragments exist, but lexical search, embeddings, vector databases, hybrid ranking, and retrieval control are not implemented here yet.

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