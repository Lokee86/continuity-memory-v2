# Reliquary Storage Format
Parent index: [Documentation index](INDEX.md)
## Purpose
This document is the exact reference owner for persistent records currently implemented by Reliquary Memory v2.
## Overview
The shared append-only container now supports two typed semantic file kinds. A Reliquary `.rel` contains the full existing source/workspace owner composition: Archive source/history records, embedded files, durable interaction-stream checkpoints, Memories, Graph relationship state, Insomnia operational/completion records, vector backing/bindings, compatibility profiles, and vector generations. A Phylactery `.phy` contains the narrower user-global owner set: Memories, Graph, Packed Vectors, Memory Vectors, and Compatibility Profiles. Each top-level type opens the same physical stream but dispatches and validates only its permitted owners.

## Reliquary and Phylactery file identity — implemented

New Reliquary files use a typed 40-byte header. The header carries authoritative `file_kind = Reliquary`, an internal scope kind for Organization, Project, or Connection, and a 16-byte durable owner UUID. Human-facing filenames should use `.org.rel`, `.prj.rel`, and `.con.rel`; filenames are hints only and do not determine semantic identity.

The existing 16-byte `.cva` header remains readable as **legacy Project Reliquary** data. Legacy detection is explicit and no automatic rewrite occurs on open, so a later migration can preserve deterministic IDs, record payloads, and semantic history while still distinguishing old physical files from typed REL files.

Phylactery `.phy` uses the same 40-byte typed header with authoritative `file_kind = Phylactery`, no Reliquary scope, and its own durable owner UUID. A `.phy` is not a legacy CVA and cannot be opened through the Reliquary/CVA lifecycle. Conversely, Phylactery rejects typed REL and legacy CVA files. See [ADR 0020](decisions/0020-reliquary-and-phylactery-file-kinds.md) and [ADR 0021](decisions/0021-typed-reliquary-scopes-and-connections.md).
## Exact contract
All integers and multi-byte scalar values are little-endian.
### Container header
| Offset | Size | Field | Value |
| --- | ---: | --- | --- |
| `0` | 8 | magic | `CVA\0\r\n\x1a\n` |
| `8` | 2 | major | `1` |
| `10` | 2 | minor | `0` |
| `12` | 4 | header length | `16` for legacy CVA; `24` for earlier typed files; `40` for current typed files |
| `16` | 1 | file kind | `1=Reliquary`; `2=Phylactery` |
| `17` | 1 | semantic scope discriminator | Reliquary: `1=Organization`, `2=Project`, `3=Connection`; Phylactery: `0` |
| `18` | 6 | reserved | zero |
| `24` | 16 | durable owner UUID | UUID bytes; current typed files only |
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
Global versions begin at `1` and are semantically consecutive. Ordinary semantic owners persist one `CVAVERS1` ticket per version. `CVAINSC2` is the exception: it owns a contiguous embedded global-version range for the Memory records inside that completion transaction, so those versions do not require standalone ticket chunks. Global versions order semantic mutations across concrete databases; immutable backing objects do not independently consume versions.

### Durable owner identity

Current typed REL/PHY files store a 16-byte UUID directly in the container header. The canonical external owner ID is derived from the authoritative type/scope plus that UUID: `proj-<uuid>`, `org-<uuid>`, `con-<uuid>`, or `phy-<uuid>`. Owner identity consumes no semantic/global version ticket. Copies, moves, renames, and reconciliation repacks preserve the UUID. Earlier 16-byte legacy CVA and 24-byte typed files have no header UUID and require explicit migration before owner-ID-based reconciliation.

Legacy `CVAWKFM1` / `CVAWKSP1` workspace-metadata chunks may remain physically present in old RELs, but current runtime semantics ignore them and do not write them.

### Interaction-stream checkpoints

Interaction-stream checkpoint:
```text
8 bytes   "CVAISTR1"
string    message ID
string    session/conversation ID
u8        parent present: 0=false, 1=true
optional  string parent message ID
u8        role: 0=User, 1=Agent
i64       timestamp_ns
u8        status: 0=Streaming, 1=Interrupted
string    cumulative UTF-8 content
```

Strings in this record use `u32 byte_length + UTF-8 bytes`. Interaction-stream records are append-only checkpoint revisions keyed by message ID; reopening keeps the latest valid revision for each message. Metadata is immutable across revisions and content must be append-only. `Interrupted` is terminal within one physical history.

These records are durable transcript state, not Archive semantic records. They consume no `CVAVERS1`, Archive version, Memory version, or Vector Generation version. A live runtime may expose the latest `Streaming` checkpoint as streaming only while the matching in-memory message is still active; after restart that same on-disk record is interpreted as interrupted. If the same message ID later exists as a completed Archive node, transcript resolution suppresses the checkpoint copy and uses the completed Archive turn.

Divergent Reliquary reconciliation retains and merges interaction-stream records independently of Archive semantic history. Compatible prefix histories keep the longest visible content and retain `Interrupted` if either side recorded interruption; non-prefix text or immutable-metadata divergence is rejected rather than silently discarding user-visible output.

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
N bytes   arbitrary content bytes
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
Episode:
```text
8 bytes   "CVAEPIS1"
32 bytes  EpisodeId
u8        origin: 1=Live, 2=Import
u8        boundary: 1=Size, 2=Inactivity, 3=CreateMemory, 4=ImportEnd
i64       source_through_ns
i64       finalized_at_ns
string    conversation ID
string    start node ID
string    end node ID
```
`EpisodeId` is SHA-256 over the conversation ID, start node ID, and end node ID in order, with each value encoded as `u64 byte_length + UTF-8 bytes`. Episodes are Archive semantic records and are published through ordinary `CVAAREC1` metadata.

File manifest:
```text
8 bytes   "CVAFILE1"
32 bytes  FileId
32 bytes  ContentId
u64       original byte length
string    filename
string    MIME type; empty = none
```
File bytes reuse the content-addressed `CVACONT1` store. `FileId` is SHA-256 over the `CVAFILE1-ID\0` domain separator, `ContentId`, byte length, length-prefixed filename, and length-prefixed MIME type. Identical bytes therefore share one content object while distinct filenames or MIME metadata remain distinct file manifests. `CVAFILE1` remains the standalone semantic representation for files introduced outside a source turn.

Native source turn with attachments:
```text
8 bytes   "CVATURN1"
i64       timestamp_ns
32 bytes  node ContentId
string    node ID
string    conversation ID
string    parent node ID; empty = none
string    role
u32       attachment count
repeated attachments:
    32 bytes  FileId
    32 bytes  ContentId
    u64       original byte length
    string    filename
    string    MIME type; empty = none
```
The turn body and attachment bytes are stored first as content-addressed `CVACONT1` backing objects. One versioned `CVATURN1` then publishes the source node, its attachment file manifests, and the source-to-file relationship together. An unversioned `CVATURN1` is inert. Attached files therefore require no later source-association record or importer-side repair step.

File-to-Memory link:
```text
8 bytes   "CVAFMEM1"
32 bytes  FileId
32 bytes  MemoryId
```
The `(FileId, MemoryId)` pair is the link identity. The file must exist in Archive when the link is written; `Cva` validates the Memory target after both Archive and Memories have rebuilt. This record represents a later semantic relationship and is intentionally separate from native source attachment provenance.

Archive semantic metadata:
```text
8 bytes   "CVAAREC1"
u64       global version
u64       Archive version
u64       record chunk offset
u64       record payload length
```
Archive versions begin at `1` and are contiguous. A semantic Archive payload without valid metadata is inert. Nodes, native source turns, branches, fragments, episodes, standalone file manifests, and file-to-Memory links are semantic Archive payloads. One `CVATURN1` consumes one Archive/global version regardless of its attachment count. Fragment creation Archive versions are retained in the derived fragment index so later generation coverage can be validated.

### Phylactery owner composition

A current `.phy` initializes and requires only the persistent formats for Memories, Graph, Packed Vectors, Memory Vectors, and Compatibility Profiles. It does not initialize or accept Archive/Episode semantics, Files/attachments, Insomnia operational/completion state, Archive Vectors, Vector Generations, or interaction-stream checkpoints as Phylactery owners.

The same Memory record codec is reused, but current Phylactery validity is stricter about provenance: `source_episode_id`, `source_node_id`, `content_source_conversation_id`, `content_source_node_id`, `grounding_source_conversation_id`, and `grounding_source_node_id` must all be absent. This allows user-global Memories to remain independently valid when an originating Project REL is unavailable or intentionally not retained. Cross-file provenance requires a future explicit lineage/export representation rather than storing REL-local IDs as dangling references.

The existing disposable lexical index indexes REL Archive Fragments and filenames, so it is not part of `.phy`. A user-Memory lexical index, if required, is a separate future derived owner/design.

### Memories
Format marker:
```text
8 bytes   "CVAMEMF2"
```
Standalone content-addressed Memory body:
```text
8 bytes   "CVAMBDY1"
32 bytes  MemoryBodyId = SHA-256(exact body bytes)
u32       body byte length
N bytes   body bytes
```
The exact body bytes are:
```text
u64       title UTF-8 byte length
N bytes   title UTF-8
u64       content UTF-8 byte length
N bytes   content UTF-8
```
Memory record:
```text
8 bytes   "CVAMEMR3"
32 bytes  MemoryId
u64       revision
32 bytes  MemoryBodyId
u8        archived: 0=false, 1=true
optional  MemoryId superseded_by
optional  MemoryId parent_id
optional  EpisodeId source_episode_id
i64       created_at_ns
i64       updated_at_ns
string    category
string    memory_type
string    authority_kind
string    scope
string    lifecycle_state
optional  string source_node_id
optional  string content_source_conversation_id
optional  string content_source_node_id
optional  string grounding_source_conversation_id
optional  string grounding_source_node_id
string    mutation_id
```
Optional fixed IDs and optional strings use a one-byte `0`/`1` presence flag followed by the encoded value when present. `authority_kind` is one of `direct`, `correction`, `adoption`, `retention`, or `unknown`; current Insomnia writes the first four, while legacy/manual records may use `unknown`. `MemoryId` for an automatically assigned new Memory is SHA-256 over `"continuity-memory-id\0"`, the mutation-ID byte length as `u64`, and the mutation-ID UTF-8 bytes. A Memory's `MemoryBodyId` cannot change across revisions.

Legacy `CVAMEMR2` records remain decodable. They have the same layout except that `authority_kind` is absent; reopen assigns `authority_kind = "unknown"` rather than inferring provenance that was never persisted.

Standalone Memory publication metadata:
```text
8 bytes   "CVAMEMV1"
u64       global version
u64       Memory version
u64       record chunk offset
u64       record payload length
```
Memory versions begin at `1` and are dense. Normal direct Memory publication may store/deduplicate a standalone body, append `CVAMEMR3`, allocate one global version, and append `CVAMEMV1`; a standalone Memory record without valid version metadata is inert. Successful Insomnia processing uses the `CVAINSC2` transaction described below instead: newly required Memory bodies, `CVAMEMR3` records, and their contiguous global-version range become visible through the one outer completion chunk and do not emit separate body/record/version/global-ticket chunks before it.

### Graph
Format marker:
```text
8 bytes   "CVAGFMT1"
u32       schema = 1
```
Dense node mapping:
```text
8 bytes   "CVAGNODE"
32 bytes  MemoryId
u32       dense NodeId
```
Node mappings are structural index records and consume no semantic version. They are assigned monotonically from zero when a Memory first participates in Graph topology. Stable public identity remains `MemoryId`.

Single relationship mutation:
```text
8 bytes   "CVAGMUT1"
32 bytes  source MemoryId
32 bytes  target MemoryId
u16       relationship kind
u8        active: 0=retracted, 1=active
```

Atomic relationship batch:
```text
8 bytes   "CVAGBAT1"
u32       relationship change count
repeated changes:
    32 bytes  source MemoryId
    32 bytes  target MemoryId
    u16       relationship kind
    u8        active: 0=retracted, 1=active
```

Relationship kind codes are `1=topical`, `2=factual`, `3=causal`, `4=recurrent`, `5=references`, `6=duplicate-of`, `7=supersedes`, and `8=structural-parent`. One batch cannot contain the same oriented `(source, target, kind)` identity more than once. Already-visible no-op states are removed before a new transaction is written.

Relationship version metadata:
```text
8 bytes   "CVAGVER1"
u64       global version
u64       graph version
u64       mutation payload chunk offset
u64       mutation payload length
```
Graph versions begin at `1` and are dense. `CVAGVER1` may reference either one `CVAGMUT1` or one non-empty `CVAGBAT1`; the complete referenced payload is one atomic Graph transaction and consumes one CVA-global version plus one Graph-local version regardless of change count. A relationship payload without valid version metadata is inert. The current state of one oriented `(source, target, kind)` identity is its latest versioned `active` value; retraction appends `active=0` rather than deleting history. Both Memory endpoints must exist on reopen. Pre-Graph CVAs with no Graph records open as Graph version `0`; the format marker is appended lazily before their first Graph mutation.

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

Current successful Episode completion:
```text
8 bytes   "CVAINSC2"
32 bytes  EpisodeId
u32       attempt number
i64       started_at_ns
i64       completed_at_ns
u32       rejected candidate count
u64       first embedded global version
u32       embedded global-version count
string    extractor model
string    extractor contract/version
u32       resulting MemoryId count
N×32      resulting MemoryIds
u32       embedded Memory-body count
repeated embedded Memory bodies:
    32    MemoryBodyId
    u32   Memory-body byte length
    N     exact Memory body bytes: u64 title byte length + UTF-8 title + u64 content byte length + UTF-8 content
u32       newly published Memory-record count
repeated newly published records:
    u64   global version
    u64   memory version
    u32   encoded Memory-record length
    N     complete "CVAMEMR3" record payload
```
The embedded global-version count must equal the newly published Memory-record count. For a non-empty publication, record global versions are contiguous beginning at the stored first version. A zero-Memory completion consumes no global versions.

`CVAINSC2` is the physical and logical visibility boundary for an Insomnia success. New content-addressed Memory bodies, their Memory records, their global-version allocation, and the compact successful Episode receipt all live inside this one outer CVA chunk. No standalone Memory-body, Memory-record, Memory-version, or `CVAVERS1` chunk is emitted before it. Embedded bodies are indexed by `MemoryBodyId` against the outer completion `ChunkRef`; body resolution reads that completion chunk and selects the matching embedded body by ID. Existing bodies may be referenced without being re-embedded.

If a `CVAINSC2` append is interrupted, ordinary trailing-chunk recovery removes the incomplete outer chunk, leaving no orphan body, record, or global-version ticket. The Episode therefore reopens as Pending and can be retried safely. A valid completed transaction reconstructs all of its new Memories and the successful Insomnia receipt together.

`CVAINSC1` remains decodable for existing development files. In that legacy format, content-addressed Memory bodies and standalone global-version tickets may precede the completion chunk; the completion remains the logical visibility boundary for its nested Memory records. Current processing writes only `CVAINSC2`.

The older `CVAINSA1` attempt record remains decodable so existing same-format development files can reopen, but current processing no longer emits it. Retryable failures are runtime-only and add no persistent record. A final Terminal outcome currently persists as one `CVAINSW1` record; a successful outcome persists as one compact `CVAINSC2` transaction.

Optional values use a one-byte `0`/`1` presence flag followed by the encoded value when present.

### Strings
```text
u32 byte_length
N bytes UTF-8
```
## Historical semantics
Archive, Memories, Graph, and Vector Generations have independent local watermarks. Global ordering may interleave their semantic mutations; integer adjacency is never semantic ancestry. Packed matrices, Memory-Vector bindings, Archive-Vector bindings, and compatibility profiles are immutable backing objects. Memory Vectors have no local clock because their identity is immutable semantic Memory content plus compatibility profile. A published Vector Generation is the semantic association that activates one profile/population.
## Diagnostics and failure behavior
`Cva::open` / `Reliquary::open` requires Reliquary type/scope identity (or the legacy 16-byte Project form) and exactly one current format marker for Archive, Memories, Insomnia operational state, Packed Vectors, Memory Vectors, Archive Vectors, Compatibility Profiles, and Vector Generations. Current 40-byte typed files also carry the durable owner UUID; earlier 16-byte and 24-byte forms remain readable for explicit migration but have no owner ID. Graph is a narrow compatibility exception: a CVA created before Graph existed may omit `CVAGFMT1` when it contains no Graph records; that CVA opens with empty Graph state and receives the marker lazily before its first Graph mutation. Other earlier development-format incompatibilities are rejected rather than migrated.

`Phylactery::open` requires exact typed Phylactery identity (`file_kind=2`, scope byte `0`) and rebuilds only Memories, Graph, Packed Vectors, Memory Vectors, and Compatibility Profiles. It validates Memory/Graph global-version uniqueness, Graph endpoints, vector/profile references, and source-independent Memory provenance. REL, legacy CVA, and invalid file-kind/scope combinations fail closed.
Container validates framing/global tickets. A truncated **final** length-prefixed chunk is treated as an interrupted append: reopen truncates the file to that chunk's starting offset and resumes from the last complete chunk boundary. Truncation of the CVA header still fails closed. Concrete stores validate their own complete records. Cross-store references are validated after reconstruction in dependency order. Composition-level validation rejects a global version claimed by multiple semantic mutations.
## Defaults or precedence
Default fragments use eight turns with two-turn overlap. Default Episode input ceiling is 32 KiB. Compatibility probe suite v1 and compatibility policy v2 are fixed by the current implementation.
## Related docs
- [Architecture](architecture.md)
- [Rust API](api.md)
- [ADR 0006](decisions/0006-archive-vector-row-bindings.md)
- [ADR 0007](decisions/0007-compatibility-profiles-and-vector-generations.md)
- [ADR 0012](decisions/0012-deterministic-episodes-and-insomnia-memory-authority.md)
- [ADR 0013](decisions/0013-immutable-memory-vector-bindings.md)
## Notes
These are development formats. Filenames are indexed only in the disposable in-memory lexical index and add no persistent record. File-tree semantics, file-content extraction/indexing, migration, packing/compression, authentication/encryption, quantization metadata, persistent lexical indexing, ANN acceleration, and retention/vacuum remain future work.
