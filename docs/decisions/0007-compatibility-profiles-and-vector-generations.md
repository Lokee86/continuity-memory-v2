# ADR-0007: Compatibility profiles and vector-generation publication

Status: Accepted
Date: 2026-08-14
Owners: vector-space compatibility and vector-generation publication
Supersedes: none
Superseded by: none

## Context

Archive Vectors bind packed rows to Archive fragments but intentionally do not say which embedding space produced those rows. Retrieval still needs a durable way to decide whether a configured embedding endpoint can safely search an existing vector population.

Endpoint identity is not a reliable compatibility boundary. Provider names, routing providers, reported model names, and model revisions may be absent, change independently of vector behavior, or differ while producing compatible vectors. Exact hashes of probe output are also too brittle: numerically equivalent executions can differ slightly in returned floating-point values.

## Decision

Add immutable `CompatibilityProfile` objects and mutable `VectorGeneration` publication.

A compatibility profile contains only the durable compatibility contract needed by the current vector layer:

```text
CompatibilityProfile
├── dimensions
├── normalization
├── probe_suite_version
├── compatibility_policy_version
└── probe references
    ├── 2 query-mode vectors
    └── 2 document-mode vectors
```

Provider, model name, endpoint-reported model, and revision are not profile fields and do not participate in compatibility identity. Such values may be useful provenance elsewhere later, but they are not evidence that two endpoints share a vector space.

The fixed probe suite is embedded through the endpoint in both Query and Document modes. Compatibility is verified by corresponding-vector cosine similarity, not exact output equality. Policy v1 requires every probe to reach cosine `>= 0.99999`. Dimensions, normalization, probe-suite version, and compatibility-policy version must also match.

`CompatibilityProfileId` content-addresses the exact stored profile artifact, including its reference vectors. The ID is not itself the compatibility test. When establishing a profile, Continuity probes the endpoint once, compares the candidate tolerantly against existing profiles, reuses a compatible existing profile when one exists, and stores a new profile only when no compatible profile exists.

Compatibility profiles are immutable backing objects and consume no semantic version ticket.

A published generation is:

```text
VectorGeneration
├── compatibility_profile_id
├── archive_vector_id
├── source_archive_version
├── global_version
└── vector_version
```

Generation publication is the first mutable vector-layer semantic operation. `vector_version` is dense within `VectorGenerationStore`; `global_version` orders generation publication against other CVA semantic mutations. The latest generation for each compatibility profile is current, while older generations remain addressable through historical vector-version cuts.

Generation publication validates that the compatibility profile and Archive-Vector set exist, the packed-matrix dimensions match the profile, the current matrix representation is `f32`, and the claimed source Archive version is not older than any mapped fragment. Packed storage remains representation-generic, but a semantic generation cannot publish a representation whose search interpretation has not been defined. Building a generation from an endpoint first verifies that endpoint against the compatibility profile.

The compatibility-profile and generation formats use new markers (`CVACPFM1` and `CVAVGFM2`). CVAs produced by the discarded endpoint-identity profile format are development artifacts and are rejected rather than migrated.

## Consequences

### Ownership

- `CompatibilityProfileStore` owns vector-space compatibility contracts and reference probes.
- Endpoint/provider/model provenance is outside the compatibility owner.
- `VectorGenerationStore` owns profile-to-Archive-vector activation, Archive coverage, and vector semantic ordering.
- `ArchiveVectorStore` remains profile-independent row-to-fragment identity.
- `PackedVectorStore` remains numeric representation only.

### Routed and numerically drifting endpoints

Small floating-point differences do not create a new profile when the endpoint remains above the compatibility threshold. A materially different endpoint does not pass the profile and cannot build/search that generation under the profile.

### Reopen

`Cva::open` continues to perform one physical chunk scan. Compatibility profiles are rebuilt before vector generations are cross-validated. The CVA-global ticket sequence is still owned by Container; composition-level validation rejects a global ticket claimed by both Archive and Vector Generations.

## Alternatives considered

- **Provider/model/revision as profile identity:** rejected because endpoint labels are provenance, not proof of vector-space compatibility.
- **Exact SHA-256 of probe output as compatibility identity:** rejected because tiny numerical drift produces false incompatibility.
- **No probes; trust dimensions/model names:** rejected because equal dimensions and labels do not prove compatible vector spaces.
- **Put compatibility data inside Archive Vectors:** rejected because row-to-Archive identity is independent of how vectors were produced.
- **Publish profiles on the semantic clock:** rejected because profiles are immutable compatibility evidence; activation happens through Vector Generations.

## Verification

Focused tests prove tolerant reuse under small deterministic drift, separation of materially different endpoints, dimension/scalar rejection, profile round-trip and clock neutrality, generation activation/history, endpoint compatibility enforcement, source-Archive coverage validation, inert unversioned generation payloads, global-ticket collision rejection, and exact current-generation semantic retrieval.

The corpus smoke builds two simulated compatibility profiles and two independent vector generations over the same Archive population, verifies both after reopen, and executes exact semantic retrieval through one selected profile.

## References

- [Architecture](../architecture.md)
- [Storage format](../storage-format.md)
- [Architectural invariants](../invariants.md)
- [ADR 0005](0005-cva-composition-and-packed-vector-objects.md)
- [ADR 0006](0006-archive-vector-row-bindings.md)
