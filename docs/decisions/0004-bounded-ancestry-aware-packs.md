# ADR-0004: Bounded ancestry-aware packs with pack-level compression

Status: Accepted
Date: 2026-08-14
Owners: CVA physical packing/compression; Archive conversation locality
Supersedes: none
Superseded by: none

## Context

The current development CVA stores every physical record as one raw length-prefixed chunk. Archive content objects contain individual turn bodies, not whole conversations. Archive fragments are retrieval ranges over turns and are unrelated to physical chunk boundaries.

Per-record compression is possible, but the records most likely to benefit today are individual content objects. At the current `2.2 MB` corpus size, cold-open time is effectively indistinguishable from warm-open time, so adding per-record compression now would mainly reduce storage while sacrificing cross-record redundancy that larger compression units can exploit.

Future packing must also preserve Archive branch semantics without copying shared conversation ancestry.

## Decision

Do not make a whole conversation or a whole branch the physical compression unit.

Future Archive packing will use bounded immutable packs containing physically local logical records. Pack size is a tunable/measured parameter rather than a semantic constant. Packs may be compressed as physical units; compression codec and exact target size remain implementation-time measurements.

Conversation ancestry constrains pack growth:

```text
A1 -> A2 -> A3 -> A4 -> A5
          \
           B4 -> B5
```

A valid physical shape is:

```text
Pack 1: A1 A2 A3    shared ancestry, stored once
Pack 2: A4 A5       one continuation
Pack 3: B4 B5       divergent continuation
```

A branch never owns or duplicates its ancestral packs. Shared immutable nodes/content remain stored once and may be referenced by every descendant branch that uses them.

Once histories diverge, pack growth should follow one ancestry path rather than mixing divergent descendants into the same logical continuation pack. Size limits can seal a pack earlier; divergence can also create a new continuation boundary. Existing shared packs are never repacked merely because a branch appears.

Packing and compression remain distinct mechanics:

```text
logical records
    -> bounded pack
    -> optional compression of the pack
    -> physical CVA storage
```

Larger packs generally improve compression opportunity but increase random-read/decompression granularity, transient memory, rewrite/vacuum cost, and corruption blast radius. The final size therefore must be benchmarked against compression ratio, reopen cost, and random retrieval cost.

Packing should also preserve database/record locality. Archive records should not be indiscriminately mixed with future vector, Memory, or Graph records merely because writes were temporally adjacent. Exact pack composition remains a concrete-store decision.

## Consequences

- No standalone per-content-object compression work is required before Archive Vectors/retrieval.
- Packing and pack-level compression should be designed together after basic retrieval exposes real access patterns.
- Shared conversation prefixes remain physically deduplicated across branches.
- Branch semantics constrain pack boundaries without turning branches into physical ownership units.
- Whole-conversation compression is avoided because conversations grow and would otherwise require large rewrites or awkward append semantics.
- Whole-branch compression is avoided because branches share ancestry and branch-owned storage would encourage duplication.
- Fragment semantics remain independent of physical packing; a fragment may resolve across pack boundaries.
- Future pack compression may leave incompressible data raw when that is cheaper; vectors in particular should not be assumed compressible.

## Alternatives considered

- **Compress every current chunk independently:** deferred because individual turn bodies are relatively small and current cold-open measurements do not show an I/O bottleneck.
- **One compressed blob per conversation:** rejected because append growth and random access would require increasingly large rewrites/decompression.
- **One compressed blob per branch:** rejected because branches share ancestry and should not duplicate shared history.
- **Very large unrestricted packs:** rejected as a default because marginal compression gains eventually trade against random access, memory, vacuum/rewrite cost, and failure granularity.

## Verification

When implemented, benchmark several pack-size targets using at least:

- compressed file size / compression ratio;
- warm and repeatedly cache-evicted cold reopen;
- random retrieval of localized conversation ranges;
- transient decompression memory;
- branch divergence with proof that shared ancestry is stored once.

No fixed pack size or compression codec is accepted by this ADR; those are measurement-driven implementation choices.

## Risks and debt

- Current `ChunkRef` addresses standalone physical chunks and will need a representation for logical records inside packs.
- Reclamation/vacuum and pack rewrite rules remain undefined.
- Exact Archive record grouping inside a pack remains open.
- Corruption detection/checksums and encryption are separate format concerns.

## References

- [Architecture](../architecture.md)
- [Storage format](../storage-format.md)
- [Roadmap](../roadmap.md)
- [Current limitations](../current-limitations.md)
