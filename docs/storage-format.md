# CVA Storage Format

Parent index: [Documentation index](INDEX.md)

## Purpose

This document is the exact reference owner for the persistent CVA, Archive, packed-vector, and Archive-Vector records currently implemented.

## Overview

CVA format `1.0` is a development append format. A fixed 16-byte header is followed by opaque length-prefixed chunks. `Cva::open` performs one physical scan and rebuilds Archive, packed-vector backing objects, and Archive-Vector row bindings.

## Exact contract

### CVA header

All integers are little-endian.

| Offset | Size | Field | Value |
| --- | ---: | --- | --- |
| `0` | 8 | magic | `CVA\0\r\n\x1a\n` |
| `8` | 2 | major | `1` |
| `10` | 2 | minor | `0` |
| `12` | 4 | header length | `16` |

### Physical chunk

```text
u64 payload_length
N bytes payload
```

A `ChunkRef` stores the length-prefix offset and payload length.

### Global version ticket

```text
8 bytes   magic = "CVAVERS1"
u64       global version
```

Tickets begin at `1` and are physically consecutive. They order semantic mutations across the CVA. Immutable backing objects such as Archive content, packed matrices, and Archive-Vector bindings do not independently consume tickets.

### Archive format marker

```text
8 bytes   magic = "CVAAFMT2"
u32       schema version = 1
```

Exactly one is required.

### Archive content

```text
8 bytes   magic = "CVACONT1"
32 bytes  SHA-256 content ID
u32       content byte length
N bytes   UTF-8 content
```

### Archive node

```text
8 bytes   magic = "CVANODE1"
i64       timestamp_ns
32 bytes  content ID
string    node ID
string    conversation ID
string    parent node ID; empty = none
string    role
```

### Archive branch/session head

```text
8 bytes   magic = "CVABRCH1"
u8        canonical flag
string    branch ID
string    conversation ID
string    leaf node ID
```

Repeated branch identities are immutable head revisions. Existing heads advance only to descendants.

### Archive fragment

```text
8 bytes   magic = "CVAFRAG1"
32 bytes  fragment ID
string    conversation ID
string    start node ID
string    end node ID
```

Fragment ID is SHA-256 of conversation/start/end IDs with `0x00` separators. Branch identity is excluded.

### Archive record-version metadata

Every semantic node, branch revision, or fragment has:

```text
8 bytes   magic = "CVAAREC1"
u64       global version
u64       Archive version
u64       record chunk offset
u64       record payload length
```

Archive versions start at `1` and are contiguous. Global versions must increase between Archive mutations but may have gaps. The metadata contains no publication parent or Archive-head pointer. With current chunk framing, each semantic Archive mutation adds 72 bytes of ordering metadata: 24 bytes for the global ticket chunk and 48 bytes for `CVAAREC1`.

### Packed-vector format marker

```text
8 bytes   magic = "CVAPVFM1"
u32       schema version = 1
```

Exactly one is required.

### Packed-vector object

```text
8 bytes   magic = "CVAPVEC1"
32 bytes  packed-vector ID
u32       dimensions
u8        scalar tag
3 bytes   reserved = 0
u64       row count
u64       matrix byte length
N bytes   contiguous matrix rows
```

`row_bytes = dimensions * scalar_width`; `matrix_byte_length = row_count * row_bytes`. Multi-byte scalar values are little-endian. Scalar tags: `1=i8`, `2=u8`, `3=i16`, `4=u16`, `5=i32`, `6=u32`, `7=i64`, `8=u64`, `9=f16`, `10=bf16`, `11=f32`, `12=f64`.

Packed-vector ID is SHA-256 over `"CVA-PACKED-VECTOR-V1\0"`, dimensions, scalar tag, and exact matrix bytes.

### Archive-Vector format marker

```text
8 bytes   magic = "CVAAVFM1"
u32       schema version = 1
```

Exactly one is required.

### Archive-Vector set

```text
8 bytes        magic = "CVAAVEC1"
32 bytes       Archive-Vector ID
32 bytes       packed-vector ID
u64            row count
row_count × 32 ordered FragmentIds
```

Row ordinal maps directly to the FragmentId at the same ordinal. The referenced packed matrix must exist and have exactly the same row count. Every mapped fragment must exist in Archive and may occur only once within the set.

Archive-Vector ID is SHA-256 over `"CVA-ARCHIVE-VECTORS-V1\0"`, the packed-vector ID, and the ordered FragmentId sequence. Order is therefore part of identity.

Archive-Vector sets contain no profile/model/metric/normalization metadata and no source Archive watermark. Those belong to the future profile/generation layer.

### Strings

```text
u32 byte_length
N bytes UTF-8
```

## Historical semantics

Archive state through `A=N` is defined by replaying Archive semantic metadata through that watermark. Conversation ancestry comes from node parents, not Archive-version adjacency.

Packed matrices and Archive-Vector sets are immutable backing objects. Their existence does not publish an active retrieval generation and does not advance semantic clocks.

## Diagnostics and failure behavior

Container open rejects invalid/truncated framing and malformed/non-consecutive global tickets.

`Cva::open` requires exactly one Archive, packed-vector, and Archive-Vector format marker. Archive rebuild validates content/history/references. Packed-vector rebuild validates schema, shape, reserved bytes, and content identity. Archive-Vector rebuild validates object identity plus matrix existence, exact row counts, real fragments, and unique fragment mappings.

Unversioned Archive node/branch/fragment payloads remain physically tolerated but semantically inert.

## Defaults or precedence

Default fragments use eight turns, two-turn overlap, and six-turn stride.

## Related docs

- [Architecture](architecture.md)
- [Rust API](api.md)
- [Current limitations](current-limitations.md)
- [ADR 0006](decisions/0006-archive-vector-row-bindings.md)

## Notes

These remain development formats. Migration, compaction, retention, authentication, encryption, Archive packing/compression, embedding profiles, vector generations, and similarity search are not yet implemented.
