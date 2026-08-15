# ADR-0005: CVA composition and immutable packed-vector objects

Status: Accepted
Date: 2026-08-14
Owners: CVA physical composition; packed-vector backing storage
Supersedes: none
Superseded by: none

## Context

Archive was initially the only concrete database, so the public `Archive` object also owned the physical `Container`. Adding reusable packed-vector storage creates a second concrete owner inside the same `.cva`. Keeping the file handle inside Archive would make Archive physically own another database; opening each database through a separate Container would instead rescan the same file and create competing physical owners.

Packed vector bytes also do not yet have embedding-profile, generation, Archive-fragment, or Memory identity. Giving raw matrices semantic publication clocks before those owners exist would invent semantics prematurely.

## Decision

Introduce `Cva` as the narrow physical composition owner:

```text
Cva
├── Container
├── Archive
└── PackedVectorStore
```

`Cva` owns the one file handle and performs one physical reopen scan. The scan feeds the same payload to explicit Archive and packed-vector rebuild states. There is no generic database registry, database trait, semantic root, or dependency graph.

Packed vectors are immutable content-addressed backing objects. Their identity is SHA-256 over a versioned domain separator plus dimensions, scalar tag, and the exact packed matrix bytes. Equal matrices with the same schema are stored once.

The packed row schema comes from Lodestone's dependency-light `lodestone-packed` crate. Dimensions are any non-zero `u32`; scalar representations include signed/unsigned 8/16/32/64-bit integers plus f16, bf16, f32, and f64. Rows have fixed width and are stored contiguously with no per-row framing.

Raw packed-vector object creation does **not** allocate a CVA-global version ticket and does **not** advance the Archive clock. It is backing data, analogous to an Archive content object. ADR 0006 subsequently defines Archive Vectors as another immutable backing layer; ADR 0007 defines VectorGeneration publication as the semantic activation/order layer for a profiled vector population.

## Consequences

### Ownership

- Container owns physical framing, I/O, and the global ticket sequence.
- `Cva` owns physical composition and dispatch only.
- Archive owns conversation/history semantics and its local watermark.
- `PackedVectorStore` owns immutable matrix identity, durable records, validation, deduplication, and derived ID-to-chunk lookup.
- Embedding profiles, source-row mappings, metrics, quantization meaning, and active generations are not owned by the packed store.

### Reopen and recovery

- CVA reopen remains one physical chunk walk.
- Both current concrete stores must see exactly one valid format marker.
- Packed-vector objects are validated for schema/shape and recomputed content identity during reopen.
- Packed-vector matrix bytes are not retained in the derived reopen index; the index retains metadata plus `ChunkRef` and reads the matrix on demand.

### Compatibility

- Existing Archive-only development CVAs lack the packed-vector format marker and are rejected by `Cva::open`.
- No migration path is added. Development corpora are regenerated.
- Lodestone v1 objects/snapshots remain unchanged; `lodestone-packed` is a separate reusable representation layer.

## Alternatives considered

- **Keep Container inside Archive:** rejected because Archive would become the physical owner of unrelated vector storage.
- **Open one Container per database:** rejected because every database would rescan the same CVA and physical write ownership would be ambiguous.
- **Give packed matrices a semantic local clock now:** rejected because raw matrices have no active-profile/generation semantics yet.
- **Generalized database registry/trait:** rejected because two concrete owners do not justify merging semantic authority.
- **Import all of `lodestone-core`:** rejected because Continuity only needs packed-row representation and should not pull in Lodestone's mmap/object/search dependencies.

## Verification

Tests cover packed-vector round trip beside Archive data, content-addressed deduplication, non-advancement of semantic clocks, and schema support from 512×int8 through 4096×float64.

## Risks and debt

- The current Container scan materializes each physical chunk in memory, so a very large single vector object can create a large transient allocation during reopen. Segmentation or selective/streaming scan behavior should be measured before very large vector populations.
- Exact similarity search is intentionally not implemented in this slice. Row-to-domain identity was subsequently implemented by ADR 0006.

## References

- [Architecture](../architecture.md)
- [Storage format](../storage-format.md)
- [Architectural invariants](../invariants.md)
- [Roadmap](../roadmap.md)
- [ADR 0001](0001-purpose-built-database-ownership.md)
- [ADR 0006](0006-archive-vector-row-bindings.md)
- [ADR 0007](0007-embedding-profiles-and-vector-generations.md)
