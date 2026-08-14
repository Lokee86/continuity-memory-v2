# CVA and Archive Storage Format

Parent index: [Documentation index](INDEX.md)

## Purpose

This document is the exact reference owner for the persistent file and Archive record format currently implemented by this repository.

## Overview

CVA format `1.0` is a development append format. A fixed 16-byte header is followed by opaque length-prefixed chunks. Container-global version tickets and Archive-local record-version metadata provide two ordering levels without an Archive-wide semantic parent chain.

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

A `ChunkRef` stores the byte offset of the length prefix and payload length.

### Global version ticket

```text
8 bytes   magic = "CVAVERS1"
u64       global version
```

Tickets begin at `1` and are physically consecutive. They order durable mutations across the CVA. A consumed ticket may have no corresponding Archive mutation if an operation fails after allocation.

### Archive format marker

```text
8 bytes   magic = "CVAAFMT2"
u32       marker payload schema version = 1
```

Exactly one marker is required by `Archive::open`. The `CVAAFMT2` magic intentionally rejects the earlier development prototype that used Archive-wide publication ancestry.

### Archive content record

```text
8 bytes   magic = "CVACONT1"
32 bytes  SHA-256 content ID
u32       content byte length
N bytes   UTF-8 content
```

Content objects are immutable backing data. They do not independently advance the semantic Archive clock; the node that references new content does.

### Archive node record

```text
8 bytes   magic = "CVANODE1"
i64       timestamp_ns
32 bytes  content ID
string    node ID
string    conversation ID
string    parent node ID; empty = none
string    role
```

Nodes are immutable. `parent node ID` defines conversation-local ancestry and must resolve within the same conversation.

### Archive branch/session-head record

```text
8 bytes   magic = "CVABRCH1"
u8        canonical flag
string    branch ID
string    conversation ID
string    leaf node ID
```

The same `(conversation ID, branch ID)` may appear repeatedly. Each occurrence is an immutable head revision. A later revision may advance only to a descendant of the previous leaf. Replaying through a historical Archive cut uses the newest visible revision; reviving an older point uses a new branch identity.

### Archive fragment record

```text
8 bytes   magic = "CVAFRAG1"
32 bytes  fragment ID
string    conversation ID
string    start node ID
string    end node ID
```

Fragment ID is SHA-256 of `conversation_id`, `start_node_id`, and `end_node_id`, each followed by `0x00`. Branch identity is excluded.

### Archive record-version metadata

Every semantic node, branch revision, or fragment record has one metadata chunk:

```text
8 bytes   magic = "CVAAREC1"
u64       global version
u64       Archive version
u64       semantic record chunk offset
u64       semantic record payload length
```

Archive versions start at `1` and must be contiguous. Global versions must increase between Archive mutations but need not be contiguous because other domains or failed operations may consume tickets.

The metadata contains no parent publication or Archive-head pointer.

### Strings

```text
u32 byte_length
N bytes UTF-8
```

## Historical semantics

The whole Archive state through watermark `A=N` is defined by replaying semantic record metadata with `archive_version <= N` in order:

- nodes are immutable insertions;
- fragments are immutable insertions;
- repeated branch/session identities replace only the derived current head for that identity.

Conversation branching is determined by node `parent_id`, not Archive-version adjacency.

## Defaults or precedence

Default fragments use eight turns, two-turn overlap, and six-turn stride. Live paths materialize complete windows; close may append a final tail.

## Diagnostics and failure behavior

Container open rejects invalid/truncated header or chunk framing and malformed/non-consecutive global version tickets.

Archive open rejects a missing/duplicate Archive marker, malformed recognized records, corrupt content IDs, non-contiguous Archive versions, non-increasing/out-of-range global versions in Archive metadata, invalid semantic `ChunkRef`s/types, node conflicts/cycles, missing references, and invalid fragments.

Unversioned node/branch/fragment payloads are physically tolerated but are not semantic Archive state; this makes interrupted pre-metadata writes inert on reopen.

## Examples

A node with unseen content typically appends:

```text
content payload
node payload
global version ticket
Archive record-version metadata
```

A branch-head update appends a new branch payload plus version ticket/metadata. Older head revisions remain retained.

Each semantic Archive mutation currently adds 72 bytes of ordering metadata/framing: 24 bytes for the global ticket and 48 bytes for `CVAAREC1`.

## Related docs

- [Architecture](architecture.md)
- [Rust API](api.md)
- [Versioning and rollback plan](version-history-plan.md)
- [Current limitations](current-limitations.md)

## Notes

The CVA and Archive formats remain development formats. Compaction, retention, migration, authentication, encryption, and compression are not yet implemented.