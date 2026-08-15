# ADR 0012: Deterministic episodes and Insomnia memory authority

## Status

Accepted — 2026-08-15.

## Context

Continuity v2 treats conversations as indefinitely appendable Archive streams. A conversation does not acquire a terminal state merely because a provider session ended, an import reached its current end, or an earlier episode was finalized.

The previous implementation already proved a useful deterministic episode model: one response cycle begins at a user turn and contains subsequent assistant/tool/system turns up to the next user turn; complete response cycles are packed beneath an exact serialized input budget and never split. The previous runtime also allowed the live model to satisfy memory generation independently of Insomnia. That second behavior no longer matches the v2 authority model.

## Decision

Episodes are Archive-owned immutable ancestry ranges, not semantic topic units.

- Episode identity is derived from conversation ID plus immutable start/end node IDs on one ancestry path.
- Response cycles remain indivisible.
- The default exact episode input ceiling remains 32 KiB.
- Size overflow finalizes the preceding complete response-cycle range.
- A live uncovered tail finalizes after 15 minutes of inactivity by default; the timeout is configurable.
- A finite import finalizes the source tail available at end-of-import without closing the conversation.
- Previously finalized episode ranges are immutable and must form a contiguous prefix on the selected ancestry path. A branch that diverges before that prefix fails closed.
- No model call or semantic topic detector defines episode identity.

Insomnia is the sole authoritative generator of working memory.

Every finalized episode is Insomnia work. Scheduling priority is:

1. `create_memory` / immediate live work;
2. normal live work;
3. imported/backfill work.

Within a class, source chronology determines order.

The live model receives only a narrow `create_memory` capability. It cannot supply a memory payload or write the Memories owner. `create_memory` finalizes the current uncovered episode tail and queues the resulting tail episode at immediate-live priority. Earlier episodes produced by a size boundary retain normal live priority.

Explicit imperative retention language such as “remember this” remains part of the Insomnia extraction contract. The tool affects when Insomnia runs; the archived user instruction affects what Insomnia must retain when its referent is identifiable. Correct retention must not depend on the live model having called the tool.

Memories are a separate mutable semantic owner with stable memory IDs, immutable revisions, expected-revision conflict checks, mutation-ID replay protection, exact Archive/Episode provenance, a dense `memory_version`, and CVA-global ordering only at publication. Memory authority is independent of embedding availability.

Insomnia queue/lease/retry state is operational state, not a semantic Memories timeline. Queue registration is idempotent by episode. Active claims use lease tokens; stale tokens cannot finalize reclaimed work. Retry/terminal outcomes and attempt history remain inspectable. Interrupted processing becomes reclaimable on reopen.

## Consequences

- Finalizing an episode never closes a conversation.
- The old `processed/live_chat` shortcut is retired; seeing a conversation live does not mean memory extraction happened.
- The old synchronous `CaptureMemory` contract is retired. Immediate memory requests schedule Insomnia rather than letting the live model write or synchronously own persistence.
- Archive owns episode source truth; Memories owns working-memory truth; Insomnia operational state owns processing coordination.
- This adds development format markers for Memories and Insomnia state. Existing pre-ADR development CVAs are intentionally incompatible; no migration scaffolding is added during the rebuild.
- Memory vectors remain a separate owner. The later Memory-Vector decision binds immutable semantic `MemoryBodyId` content to compatibility profiles rather than Memory revisions; metadata-only revisions do not invalidate embeddings.

## Rejected alternatives

### Semantic episode detector

Rejected. It adds another model call or semantic subsystem merely to decide processing boundaries and makes episode identity nondeterministic.

### Conversation completion as an episode boundary

Rejected. Conversations are indefinitely appendable and provider/session completion is not source-history termination.

### Live-model working-memory writes

Rejected. It creates two competing authorities for working memory and bypasses Insomnia extraction/provenance policy.

### `create_memory` as a memory-write tool

Rejected. The live model may request immediate processing but cannot decide the authoritative persisted memory representation.
