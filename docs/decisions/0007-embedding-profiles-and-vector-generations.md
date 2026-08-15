# ADR-0007: Embedding profiles and vector-generation publication

Status: Accepted
Date: 2026-08-14
Owners: embedding-space identity; vector-generation semantic publication
Supersedes: none
Superseded by: none

## Context

Packed vectors own numeric bytes and Archive Vectors own row-to-`FragmentId` identity. Neither layer answers which embedding space produced those rows or which vector population retrieval should currently use.

Those are separate responsibilities. Profile identity must remain independent of packed scalar representation and Archive bindings, while generation publication must associate a profile with one Archive-Vector set and an Archive coverage point.

## Decision

### Embedding profiles

An `EmbeddingProfile` is an immutable, content-addressed vector-space identity containing:

- provider name;
- model name;
- model revision string;
- dimensions;
- declared normalization (`None` or `L2`);
- probe-suite version;
- behavior fingerprint.

The profile deliberately does **not** contain packed scalar type, Archive mappings, generation state, or a semantic version.

Profile identity is SHA-256 over a versioned domain separator plus all fields above. The behavior fingerprint is SHA-256 over exact `f32` results from probe-suite v1 in both query and document modes. This prevents endpoints with identical advertised metadata but different observed embedding behavior from silently sharing one profile.

Profile creation is clock-neutral. Profiles are immutable backing identity objects, not active retrieval state.

### Endpoint abstraction

`EmbeddingEndpoint` exposes a descriptor plus `Query` and `Document` embedding modes. The current repository includes only `SimulatedEmbeddingEndpoint`, which deterministically generates vectors from a seed for tests and development.

Probe-suite v1 uses these fixed inputs:

```text
query:    continuity probe query: cedar orbit
query:    continuity probe query: violet engine
document: continuity probe document: quiet harbor
document: continuity probe document: copper lantern
```

If `L2` normalization is declared, probe and generation vectors must be unit length within tolerance.

### Vector generations

A `VectorGeneration` is the first vector-layer semantic publication. It contains:

```text
profile_id
archive_vector_id
source_archive_version
global_version
vector_version
```

Its content identity is SHA-256 over profile ID, Archive-Vector ID, and source Archive version. `vector_version` is a dense generation-store-local `u64` watermark; `global_version` comes from the CVA-global ordering clock. Neither integer is ancestry.

Generation publication writes the immutable generation payload first, then allocates a global ticket, then writes version metadata pointing backward to that payload. An unversioned generation payload is inert on reopen.

The latest published generation for each profile is the derived active generation for that profile. Older generations remain retained and queryable by vector-version cut. A profile's source Archive version cannot regress.

Generation validation requires:

- the profile exists;
- the Archive-Vector set exists;
- profile dimensions equal packed-matrix dimensions;
- source Archive version is not ahead of current Archive state;
- source Archive version is at least the creation version of every mapped fragment.

The Archive-Vector persistent object remains profile-independent. Its rebuild index derives the newest mapped-fragment Archive version solely to validate generation coverage.

### Development builder

`build_archive_vector_generation` verifies the endpoint against the stored profile, embeds all currently durable Archive fragments in deterministic `FragmentId` order using document mode, writes an `f32` packed matrix, writes the Archive-Vector binding, and finally publishes the generation.

Backing objects written before a failed publication may remain orphaned. They consume no semantic versions and can be reclaimed by future reachability/vacuum work.

## Consequences

- Multiple profiles can coexist with independent active generations over the same Archive.
- Global ordering now spans two mutable semantic owners: Archive and Vector Generations; reopen rejects duplicate cross-store claims on one global version.
- Archive and vector-local watermarks remain independent.
- Retrieval can later choose one profile, resolve its active generation, search that generation's matrix, then map result rows through Archive Vectors.
- The generation source watermark makes staleness observable when Archive advances after publication.

## Limits

- Live provider adapters are not implemented.
- Exact behavior fingerprints assume deterministic probe output; tolerance/compatibility policy for nondeterministic remote providers is future work.
- The development builder currently stores generated rows as `f32`; alternate packed representations can be associated structurally, but quantization semantics are not yet modeled.
- Low-level generation publication validates structural compatibility and coverage but cannot prove that externally supplied vector bytes truly came from the claimed endpoint/profile.
- Explicit retirement/deactivation is not implemented; newest generation per profile is current.

## Verification

Tests cover profile deduplication, behavior-fingerprint separation, endpoint mismatch, generation round trip, independent profiles, generation supersession, idempotent rebuild, cross-store global ordering, dimension mismatch, source-cut validation, and inert unversioned generation payloads. A prepared-corpus smoke builds two simulated profiles and generations over all 281 fragments and verifies them after reopen.

## References

- [Architecture](../architecture.md)
- [Storage format](../storage-format.md)
- [ADR 0005](0005-cva-composition-and-packed-vector-objects.md)
- [ADR 0006](0006-archive-vector-row-bindings.md)
