# CVA Storage Format

Parent index: [Documentation index](INDEX.md)

## Purpose
This document is the exact reference owner for persistent records currently implemented by Continuity Memory v2.

## Overview
The development format is one append-only CVA file containing explicit Archive, vector-backing, profile, and vector-generation records with one shared physical scan and concrete ownership.

## Container
CVA format `1.0` is a development append format. All integers are little-endian.

| Offset | Size | Header field | Value |
| --- | ---: | --- | --- |
| `0` | 8 | magic | `CVA\0\r\n\x1a\n` |
| `8` | 2 | major | `1` |
| `10` | 2 | minor | `0` |
| `12` | 4 | header length | `16` |

Physical chunks follow the header:
```text
u64 payload_length
N bytes payload
```
`ChunkRef` stores the length-prefix offset and payload length.

### Global version ticket
```text
8 bytes   magic = "CVAVERS1"
u64       global version
```
Tickets begin at `1` and are physically consecutive. They order semantic mutations across concrete databases. Immutable backing objects do not independently consume tickets.

## Archive records
### Format marker
```text
8 bytes   "CVAAFMT2"
u32       schema = 1
```
### Content
```text
8 bytes   "CVACONT1"
32 bytes  ContentId = SHA-256(content)
u32       byte length
N bytes   UTF-8 content
```
### Node
```text
8 bytes   "CVANODE1"
32 bytes  ContentId
i64       timestamp_ns
string    node ID
string    conversation ID
string    parent node ID; empty = none
string    role
```
### Branch/session head
```text
8 bytes   "CVABRCH1"
u8        canonical
string    branch ID
string    conversation ID
string    leaf node ID
```
Repeated branch identities are immutable head revisions. Existing heads advance only to descendants.

### Fragment
```text
8 bytes   "CVAFRAG1"
32 bytes  FragmentId
string    conversation ID
string    start node ID
string    end node ID
```
Fragment ID is SHA-256 of conversation/start/end IDs with `0x00` separators. Branch identity is excluded.

### Archive record-version metadata
```text
8 bytes   "CVAAREC1"
u64       global version
u64       Archive version
u64       record chunk offset
u64       record payload length
```
Every semantic node, branch revision, or fragment has one metadata record. Archive versions begin at `1` and are contiguous. The referenced payload must precede metadata. A semantic payload without valid metadata is inert. With current chunk framing, each semantic Archive mutation adds 72 bytes of ordering metadata: 24 bytes for its global-ticket chunk plus 48 bytes for `CVAAREC1`.

## Packed vectors
### Format marker
```text
8 bytes   "CVAPVFM1"
u32       schema = 1
```
### Matrix object
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
`row_bytes = dimensions * scalar_width`; matrix length must equal `row_count * row_bytes`. Multi-byte values are little-endian. Scalar tags: `1=i8`, `2=u8`, `3=i16`, `4=u16`, `5=i32`, `6=u32`, `7=i64`, `8=u64`, `9=f16`, `10=bf16`, `11=f32`, `12=f64`.

Packed-vector ID is SHA-256 over `"CVA-PACKED-VECTOR-V1\0"`, dimensions, scalar tag, and exact matrix bytes.

## Archive Vectors
### Format marker
```text
8 bytes   "CVAAVFM1"
u32       schema = 1
```
### Row-binding object
```text
8 bytes        "CVAAVEC1"
32 bytes       ArchiveVectorId
32 bytes       PackedVectorId
u64            row count
row_count × 32 ordered FragmentIds
```
Row `N` maps to `fragment_ids[N]`. Matrix row count must match exactly; every fragment must exist and be unique within the set. ArchiveVectorId is SHA-256 over `"CVA-ARCHIVE-VECTORS-V1\0"`, PackedVectorId, and ordered FragmentIds. Profile/source metadata is excluded.

## Embedding profiles
### Format marker
```text
8 bytes   "CVAEPFM1"
u32       schema = 1
```
### Profile object
```text
8 bytes   "CVAEPRO1"
32 bytes  EmbeddingProfileId
u32       dimensions
u8        normalization: 0=None, 1=L2
3 bytes   reserved = 0
u32       probe-suite version
32 bytes  behavior fingerprint
string    provider
string    model
string    revision
```
EmbeddingProfileId is SHA-256 over `"CVA-EMBEDDING-PROFILE-V1\0"`, provider/model/revision with `0x00` separators, dimensions, normalization tag, probe-suite version, and behavior fingerprint.

Probe-suite v1 fingerprints exact `f32` embeddings from two fixed query probes and two fixed document probes; exact inputs are in ADR 0007. Profiles are immutable and consume no semantic version.

## Vector generations
### Format marker
```text
8 bytes   "CVAVGFM1"
u32       schema = 1
```
### Generation payload
```text
8 bytes   "CVAVGEN1"
32 bytes  VectorGenerationId
32 bytes  EmbeddingProfileId
32 bytes  ArchiveVectorId
u64       source Archive version
```
VectorGenerationId is SHA-256 over `"CVA-VECTOR-GENERATION-V1\0"`, profile ID, ArchiveVectorId, and source Archive version.

### Generation version metadata
```text
8 bytes   "CVAVGRC1"
u64       global version
u64       vector version
u64       generation payload chunk offset
u64       generation payload length
```
Vector versions begin at `1` and are contiguous within VectorGenerationStore. Global versions increase but may have gaps. The payload must precede metadata; an unversioned generation payload is inert.

The latest published generation for each profile is current. Source Archive version may not regress for a profile, exceed current Archive state, or predate any fragment in the referenced Archive-Vector set.

## Strings
```text
u32 byte_length
N bytes UTF-8
```

## Reopen and historical semantics
`Cva::open` requires exactly one current format marker for Archive, Packed Vectors, Archive Vectors, Embedding Profiles, and Vector Generations. Earlier development files are rejected rather than migrated.

One physical chunk scan feeds all concrete rebuild states. Container validates framing/global tickets; each owner recognizes its own records. Cross-store references are validated after reconstruction in dependency order, and reopen rejects a global version claimed by more than one semantic mutation across Archive and Vector Generations.

Archive and Vector Generations have independent local watermarks; global ordering may interleave them. Integer adjacency is never semantic ancestry. Packed matrices, Archive-Vector bindings, and profiles are immutable backing objects; a published VectorGeneration is the semantic association that makes one profile/population current.

## Related docs
- [Architecture](architecture.md)
- [Rust API](api.md)
- [ADR 0006](decisions/0006-archive-vector-row-bindings.md)
- [ADR 0007](decisions/0007-embedding-profiles-and-vector-generations.md)

## Notes
These are development formats. Migration, packing/compression, authentication/encryption, quantization metadata, exact similarity search, and retention/vacuum remain future work.
