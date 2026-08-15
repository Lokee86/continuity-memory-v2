# ADR-0006: Archive Vectors are immutable row bindings

Status: Accepted
Date: 2026-08-14
Owners: Archive-Vector row identity
Supersedes: none
Superseded by: none

## Context

Packed vectors now provide schema plus contiguous numeric rows inside the CVA, but those rows have no Archive meaning. A separate layer must answer which Archive record each row represents without also taking ownership of embedding-model identity.

Embedding profiles and vector generations are related but distinct concerns. A profile identifies an embedding space. Vector Generations, implemented subsequently in ADR 0007, associate one profile with one Archive-Vector set and coverage/activation metadata. Putting profile data directly into Archive Vectors would merge those responsibilities and make row-to-Archive identity depend on model configuration.

## Decision

Add `ArchiveVectorStore` as an explicit CVA store of immutable `ArchiveVectorSet` objects:

```text
ArchiveVectorSet
├── packed_vector_id
└── ordered FragmentId list
    row 0 -> fragment_ids[0]
    row 1 -> fragment_ids[1]
    ...
```

An Archive-Vector set contains no embedding profile, model, metric, normalization rule, quantization semantics, Archive watermark, or active-generation state.

The set ID is SHA-256 over a versioned domain separator, the referenced `PackedVectorId`, and the ordered `FragmentId` sequence. Mapping order is therefore part of identity.

Creation and reopen validate:

- the packed matrix exists;
- matrix row count exactly equals mapping length;
- every mapped `FragmentId` exists in Archive;
- a fragment appears at most once within one set;
- the stored Archive-Vector ID matches the complete mapping object.

Like packed matrices, Archive-Vector sets are immutable backing objects and do not allocate a global version ticket. VectorGeneration publication defines when a profile plus Archive-Vector set becomes meaningful active retrieval state and owns that semantic ordering.

## Consequences

### Ownership

- `PackedVectorStore` owns numeric representation and matrix identity.
- `ArchiveVectorStore` owns row-to-Archive-fragment identity.
- Archive remains authority for fragments and source text.
- `EmbeddingProfile` owns embedding-space identity.
- `VectorGenerationStore` owns the association between a profile and an Archive-Vector set, including source Archive coverage and publication ordering.

### Reopen

`Cva::open` still performs one physical chunk scan. Archive-Vector mappings are decoded during that scan and retained only transiently until Archive and packed-vector reconstruction are complete. Cross-store references are then validated. The steady-state index retains metadata plus `ChunkRef`, including a derived `max_fragment_archive_version` used by VectorGeneration coverage validation; full `FragmentId` mappings are not retained in heap.

### Compatibility

The new `CVAAVFM1` format marker is required. Earlier development CVAs without it are rejected and regenerated rather than migrated.

## Alternatives considered

- **Store `profile_id` in Archive Vectors:** rejected because profile identity belongs above row-to-Archive identity.
- **Store `source_archive_version` in Archive Vectors:** rejected because exact mapped fragment IDs already define the binding; population coverage belongs to the generation that publishes the set.
- **Make Archive own vector rows directly:** rejected because Archive remains source-history authority, while vectors are separate derived data.
- **Give every Archive-Vector set a semantic version:** deferred because immutable unactivated bindings are backing objects; generation publication is the first concrete semantic mutation that needs ordering.

## Verification

Focused tests cover row mapping round-trip, packed-matrix existence, exact row counts, real/unique fragments, clock neutrality, required format marker, and corruption rejection on reopen.

## References

- [Architecture](../architecture.md)
- [Storage format](../storage-format.md)
- [Architectural invariants](../invariants.md)
- [ADR 0005](0005-cva-composition-and-packed-vector-objects.md)
- [ADR 0007](0007-embedding-profiles-and-vector-generations.md)
