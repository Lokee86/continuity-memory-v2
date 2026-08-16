# CVA Storage Format
Parent index: [Documentation index](INDEX.md)
## Purpose
This document is the exact reference owner for persistent records currently implemented by Continuity Memory v2.
## Overview
The development format is one append-only CVA file containing Archive, vector-backing, compatibility-profile, and vector-generation records. `Cva::open` performs one physical scan and dispatches each payload to the concrete owners.
## Exact contract
All integers and multi-byte scalar values are little-endian.
### CVA header
| Offset | Size | Field | Value |
| --- | ---: | --- | --- |
| `0` | 8 | magic | `CVA\0\r\n\x1a\n` |
| `8` | 2 | major | `1` |
| `10` | 2 | minor | `0` |
| `12` | 4 | header length | `16` |
Physical chunks follow:
```text
u64 payload_length
N bytes payload
```
`ChunkRef` stores the length-prefix offset and payload length.
### Global version ticket
```text
8 bytes   "CVAVERS1"
u64       global version
```
Tickets begin at `1` and are physically consecutive. They order semantic mutations across concrete databases. Immutable backing objects do not independently consume tickets.
### Archive records
Format marker:
```text
8 bytes   "CVAAFMT2"
u32       schema = 1
```
Content:
```text
8 bytes   "CVACONT1"
32 bytes  ContentId = SHA-256(content)
u32       byte length
N bytes   UTF-8 content
```
Node:
```text
8 bytes   "CVANODE1"
i64       timestamp_ns
32 bytes  ContentId
string    node ID
string    conversation ID
string    parent node ID; empty = none
string    role
```
Branch/session head:
```text
8 bytes   "CVABRCH1"
u8        canonical
string    branch ID
string    conversation ID
string    leaf node ID
```
Current branch enumeration is reconstructed from the latest visible revision for each `(conversation ID, branch ID)` pair; it adds no persistent record type.
Fragment:
```text
8 bytes   "CVAFRAG1"
32 bytes  FragmentId
string    conversation ID
string    start node ID
string    end node ID
```
Archive semantic metadata:
```text
8 bytes   "CVAAREC1"
u64       global version
u64       Archive version
u64       record chunk offset
u64       record payload length
```
Archive versions begin at `1` and are contiguous. A semantic Archive payload without valid metadata is inert. Fragment creation Archive versions are retained in the derived fragment index so later generation coverage can be validated.
### Packed vectors
Format marker:
```text
8 bytes   "CVAPVFM1"
u32       schema = 1
```
Matrix object:
```text
8 bytes   "CVAPVEC1"
32 bytes  PackedVectorId
u32       dimensions
u8        scalar tag
3 bytes   reserved = 0
u64       row count
u64       matrix byte length
N bytes   contiguous rows
```
`row_bytes = dimensions * scalar_width`; matrix length must equal `row_count * row_bytes`. Scalar tags: `1=i8`, `2=u8`, `3=i16`, `4=u16`, `5=i32`, `6=u32`, `7=i64`, `8=u64`, `9=f16`, `10=bf16`, `11=f32`, `12=f64`.
Packed-vector ID is SHA-256 over `"CVA-PACKED-VECTOR-V1\0"`, dimensions, scalar tag, and exact matrix bytes.
### Memory Vectors
Format marker:
```text
8 bytes   "CVAMVFM1"
u32       schema = 1
```
Row-binding object:
```text
8 bytes        "CVAMVEC1"
32 bytes       MemoryVectorId
32 bytes       CompatibilityProfileId
32 bytes       PackedVectorId
u64            row count
row_count × 32 ordered MemoryBodyIds
```
Row `N` maps to `memory_body_ids[N]`. Matrix row count must match exactly, the matrix must be `f32` with dimensions equal to the compatibility profile, and every `MemoryBodyId` must exist and be unique within the set. Across all Memory-Vector objects, a `(CompatibilityProfileId, MemoryBodyId)` pair may appear only once. `MemoryVectorId` is SHA-256 over `"CVA-MEMORY-VECTORS-V1\0"`, the profile ID, `PackedVectorId`, and ordered `MemoryBodyId`s.

Memory Vectors are immutable derived bindings and consume no semantic version ticket. Memory revisions may change metadata but cannot change their semantic `MemoryBodyId`; therefore no Memory-vector regeneration/update record exists. A new compatibility profile may bind the same Memory body to another immutable vector.
### Archive Vectors
Format marker:
```text
8 bytes   "CVAAVFM1"
u32       schema = 1
```
Row-binding object:
```text
8 bytes        "CVAAVEC1"
32 bytes       ArchiveVectorId
32 bytes       PackedVectorId
u64            row count
row_count × 32 ordered FragmentIds
```
Row `N` maps to `fragment_ids[N]`. Matrix row count must match exactly; every fragment must exist and be unique within the set. `ArchiveVectorId` is SHA-256 over `"CVA-ARCHIVE-VECTORS-V1\0"`, `PackedVectorId`, and ordered `FragmentId`s. Compatibility and source-Archive metadata are excluded.
### Compatibility profiles
Format marker:
```text
8 bytes   "CVACPFM1"
u32       schema = 1
```
Profile object:
```text
8 bytes   "CVACPRO1"
32 bytes  CompatibilityProfileId
u32       dimensions
u8        normalization: 0=None, 1=L2
3 bytes   reserved = 0
u32       probe-suite version
u32       compatibility-policy version
u32       reference count
repeated references:
    u8    mode: 1=Query, 2=Document
    3     reserved = 0
    u32   vector length
    N×f32 reference values
```
Current probe suite v1 stores two Query and two Document references. Compatibility policy v2 requires corresponding probe vectors from an endpoint to have cosine similarity `>= 0.9998`, with dimensions, normalization, suite version, and policy version equal.
`CompatibilityProfileId` content-addresses the exact stored contract using `"CVA-COMPATIBILITY-PROFILE-V1\0"`, dimensions, normalization, suite/policy versions, and ordered mode+`f32` reference values. Exact ID equality is not the compatibility test; tolerant probe comparison is.
Provider, model, route, and revision metadata are not stored in compatibility profiles. Profiles are immutable and consume no semantic version.
### Vector generations
Format marker:
```text
8 bytes   "CVAVGFM2"
u32       schema = 1
```
Generation payload:
```text
8 bytes   "CVAVGEN2"
32 bytes  VectorGenerationId
32 bytes  CompatibilityProfileId
32 bytes  ArchiveVectorId
u64       source Archive version
```
Generation version metadata:
```text
8 bytes   "CVAVGRC2"
u64       global version
u64       vector version
u64       generation payload chunk offset
u64       generation payload length
```
`VectorGenerationId` is SHA-256 over `"CVA-VECTOR-GENERATION-V2\0"`, compatibility-profile ID, Archive-Vector ID, and source Archive version. Vector versions begin at `1` and are contiguous inside `VectorGenerationStore`. An unversioned generation payload is inert.
The latest published generation for each compatibility profile is current. Source Archive version may not regress for that profile, exceed current Archive state, or predate any fragment in the referenced Archive-Vector set. Current generation publication requires the referenced packed matrix to use `f32`; packed storage remains generic, but alternate searchable scalar representations are not semantic generation formats until their interpretation is defined.
### Insomnia operational records
Format marker:
```text
8 bytes   "CVAINSF1"
```
Legacy/final work-state record:
```text
8 bytes   "CVAINSW1"
32 bytes  EpisodeId
u8        priority
u8        work state
u32       attempt count
i64       updated_at_ns
optional string lease owner
optional 32-byte lease token
optional i64 lease expiry
optional i64 retry-after
optional string last error
```
`CVAINSW1` remains decodable for existing development files and is still used for one final Terminal outcome during the current compatibility slice. New Pending, Processing, renewal, Failed/retry, and lease-expiry transitions are runtime-only and do not emit records. On reopen, persisted transient Work states are ignored; every Episode without a final success/terminal outcome is re-derived as Pending from Archive Episode metadata. Old Complete/Terminal records remain compatible final state.

Successful Episode completion:
```text
8 bytes   "CVAINSC1"
32 bytes  EpisodeId
u32       attempt number
i64       started_at_ns
i64       completed_at_ns
u32       rejected candidate count
string    extractor model
string    extractor contract/version
u32       resulting MemoryId count
N×32      resulting MemoryIds
u32       newly published Memory-record count
repeated newly published records:
    u64   global version
    u64   memory version
    u32   encoded Memory-record length
    N     complete "CVAMEMR2" record payload
```
`CVAINSC1` is the visibility boundary for an Insomnia success. Content-addressed Memory bodies and global-version tickets may be appended before it, but nested Memory records are not reconstructed as current Memories until this complete chunk is present. The same chunk reconstructs the compact successful Insomnia receipt. Zero new Memory records is valid.

The older `CVAINSA1` attempt record remains decodable so existing same-format development files can reopen, but current processing no longer emits it. Retryable failures are runtime-only and add no persistent record. A final Terminal outcome currently persists as one `CVAINSW1` record; a successful outcome persists as the one compact `CVAINSC1` receipt.

Optional values use a one-byte `0`/`1` presence flag followed by the encoded value when present.

### Strings
```text
u32 byte_length
N bytes UTF-8
```
## Historical semantics
Archive and Vector Generations have independent local watermarks. Global ordering may interleave them; integer adjacency is never semantic ancestry. Packed matrices, Memory-Vector bindings, Archive-Vector bindings, and compatibility profiles are immutable backing objects. Memory Vectors have no local clock because their identity is immutable semantic Memory content plus compatibility profile. A published Vector Generation is the semantic association that activates one profile/population.
## Diagnostics and failure behavior
`Cva::open` requires exactly one current format marker for Archive, Memories, Insomnia operational state, Packed Vectors, Memory Vectors, Archive Vectors, Compatibility Profiles, and Vector Generations. Earlier development formats are rejected rather than migrated.
Container validates framing/global tickets. A truncated **final** length-prefixed chunk is treated as an interrupted append: reopen truncates the file to that chunk's starting offset and resumes from the last complete chunk boundary. Truncation of the CVA header still fails closed. Concrete stores validate their own complete records. Cross-store references are validated after reconstruction in dependency order. Composition-level validation rejects a global version claimed by multiple semantic mutations.
## Defaults or precedence
Default fragments use eight turns with two-turn overlap. Compatibility probe suite v1 and compatibility policy v2 are fixed by the current implementation.
## Related docs
- [Architecture](architecture.md)
- [Rust API](api.md)
- [ADR 0006](decisions/0006-archive-vector-row-bindings.md)
- [ADR 0007](decisions/0007-compatibility-profiles-and-vector-generations.md)
- [ADR 0013](decisions/0013-immutable-memory-vector-bindings.md)
## Notes
These are development formats. Migration, packing/compression, authentication/encryption, quantization metadata, persistent lexical indexing, ANN acceleration, and retention/vacuum remain future work.
