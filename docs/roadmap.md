# Roadmap

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns the cross-cutting implementation sequence for the clean Continuity rebuild.

## Overview

Concrete storage owners come first. Shared mechanics are generalized only after at least two concrete owners prove the same requirement.

## Current status

Completed bootstrap slices:

1. CVA create/open/header and opaque chunk storage.
2. Container-global monotonic `u64` ordering.
3. Archive content-addressed bodies and branch-aware node graph.
4. Durable range-only fragments with 8-turn / 2-overlap policy.
5. Archive-local contiguous `u64` mutation ordering.
6. Dual `(global_version, archive_version)` metadata per semantic Archive mutation.
7. Append-only branch/session-head revisions with historical lookup.
8. Old-conversation revival through conversation-local branching, not Archive rollback.
9. Reopen validation and prepared-corpus round trip.
10. Compact Archive-owned derived indexes without composite string keys; current measurements are tracked in `development.md`.
11. Single-pass Container/Archive reopen reconstruction.
12. Lodestone-derived generic packed-vector rows plus an immutable content-addressed packed-vector store inside the CVA.
13. Immutable Archive-Vector sets that bind packed rows to ordered Archive `FragmentId`s with exact cross-store validation in the same physical reopen scan.
14. Immutable endpoint-independent compatibility profiles with Query/Document reference probes, tolerant cosine verification, and deterministic simulated endpoints.
15. Vector-generation semantic publication with dense local `vector_version`, CVA-global ordering, per-profile current generations, Archive coverage validation, and inert incomplete publications.
16. Exact semantic retrieval through compatibility verification, Query-mode embedding, current-generation resolution, exact cosine scan, and row-to-FragmentId mapping.
17. Original default lexical/hybrid retrieval with 30-candidate fusion, `0.45/0.55` channel weighting, overlap diversification, and 10 final results.
18. Purpose-built replaceable local configuration container with fragment/retrieval objects, deterministic framing, unknown-object preservation, and atomic whole-file replacement.
19. Expandable model-switchboard routing with `General`/`Insomnia`/`Embedding` capabilities, `openai-codex`/`openai-ready` providers, explicit auth kinds, persisted `models.general` / optional `models.insomnia` / `models.embedding` objects, and Insomnia-to-General fallback.
20. Self-generated 256-bit master key with a temporary JSON-backed key-store seam for later encrypted credential objects.
21. AES-256-GCM credential objects with stable credential IDs, model-route references, wrong-key/tamper rejection, and switchboard auth-header attachment.
22. Detachable repo-local CLI package exposing CVA/config/auth/archive/vector inspection plus simulated vector/retrieval bring-up through public library APIs only.
23. Direct `openai-ready` embedding HTTP transport with configured dimensions, Query/Document input types, deterministic response ordering, L2 normalization, measured 16-input batching, and bounded 16-request concurrency.
24. Archive-owned deterministic Episodes with response-cycle packing, 32 KiB default input ceiling, appendable-conversation semantics, 15-minute configurable inactivity finalization, finite-import tail finalization, and branch-prefix validation.
25. A separate mutable Memories owner with immutable revisions, mutation-ID idempotency, exact Archive/Episode provenance, dense `memory_version`, and CVA-global publication ordering.
26. A separate Insomnia operational owner with immediate-live/live/import scheduling classes, oldest-source ordering within each class, durable queue/attempt history, restart reclamation, leases, retries, terminal state, and a narrow `create_memory` tail-finalization + immediate-queue seam.
27. An `insomnia` Rust module with exact Episode input reads, strict JSON-schema General-model extraction, candidate/user-authority validation, explicit-retention semantics, deterministic candidate identity, and idempotent Memory publication.
28. Immutable Memory Vectors over shared packed matrices, keyed by `(CompatibilityProfileId, MemoryBodyId)`, with missing-only embedding, reopen validation, and enforced immutable Memory semantic bodies across metadata revisions.
29. One bounded read-only Insomnia Archive-evidence round with exact-turn, maximum-64-node ancestry-range, and lexical-search reads; four-request and 64-turn/128-KiB global bounds; second-round rejection; current-Episode-only user authority; and evidence-bound external assistant provenance.
30. A configurable one-shot Insomnia backlog worker pool with 1–64 workers (default 48 after live Luna/low calibration), atomic distinct-Episode claims, model inference outside the serialized CVA mutation boundary, bounded evidence reads between model rounds, retry/terminal handling, whole-backlog drain semantics, canonical-import registration, and automatic core Memory-Vector completion after authoritative extraction using missing-only `(CompatibilityProfileId, MemoryBodyId)` bindings.
31. Native ChatGPT/Codex device-code credential acquisition using OpenAI's device-auth protocol, including one-time code presentation, authorization polling, OAuth code exchange, ChatGPT account-ID extraction, encrypted credential persistence, and a repo-local `config credential login-codex` command with no manual token copy/paste.
32. Provider-native `openai-codex` General/Insomnia Responses transport with ChatGPT OAuth/account headers, explicit persisted reasoning effort, strict JSON-schema output, SSE completion parsing, provider dispatch alongside `openai-ready`, and a live `gpt-5.6-luna` / low-reasoning Insomnia transport smoke.
33. Live whole-file Insomnia concurrency calibration over the prepared 12-conversation / 66-Episode corpus using `gpt-5.6-luna` at low reasoning: 1/2/4/8/16/32/40/44/48/52/56/64-worker runs plus a 48-worker repeat, zero retries or terminal failures at every point, and a measured 48-worker processing sweet spot under extractor contract `v2-2`. That contract was later found to use a shortened semantic prompt; `v2-3` restored the tuned legacy extraction contract. Five additional 48-worker `v2-3` runs completed cleanly and exposed semantic selection/provenance weaknesses despite improved consolidation. Contract `v2-4` made candidate authority explicit; `v2-5` added narrow deterministic provenance/question/receipt enforcement; `v2-6` rejects residual numbered prompt/phase/step progress framing; `v2-7` separates adopted assistant authority from referent grounding and persists both provenance roles independently. Gold v1 remains historical; gold v2 corrects four grounding cases while retaining the same 40 source anchors. A controlled three-run low versus three-run medium `v2-6` comparison found identical strict semantic-target agreement at 60.0%; medium increased all-source repeatability (`0.536` to `0.656` mean pairwise Jaccard) but took about 69% longer, produced about 20% more Memories, and did not materially improve correctness. Luna-low remains the selected reasoning level. 48 remains the provisional worker default pending the focused concurrency confirmation after semantic selection/support work settles.
34. A candidate-level second-pass semantic reviewer was implemented experimentally and rejected after live measurement rather than retained. Three v2-7 control runs versus three review runs showed improved omit-case cleanliness (`79.2%` to `87.5%`) but worse retain-source recall (`65.6%` to `57.3%`) and worse average mechanical gold selection (`68.3%` to `63.3%`). The reviewer removed the tracked Godot-first false decision but could not recover omitted sources and still retained phase-renumbering. Direct inspection confirmed all four grounding-required gold cases already have sufficient context inside their authoritative Episode, so broader ancestry/vector retrieval is not the current blocker. Experimental reviewer code was removed.

## Expected ownership or ownership boundary

`Cva` owns physical composition and the single Container handle. Archive owns source-history semantics, immutable Episodes, and `archive_version`. Memories owns authoritative working-memory revisions and dense `memory_version`; semantic `MemoryBodyId` content is immutable across revisions. Insomnia operational state owns episode-processing coordination but no semantic clock. Packed vectors own immutable numeric matrices; Memory Vectors own immutable `(CompatibilityProfileId, MemoryBodyId)` row bindings; Archive Vectors own immutable row-to-fragment bindings; Compatibility Profiles own immutable vector-space compatibility contracts. Vector Generations own Archive profile/population activation and the independent dense `vector_version`. Archive, Memories, and Vector Generations interleave only through CVA-global ordering; Memory Vectors are clock-neutral derived bindings.

## Planned behavior

Near-term priorities:

1. Separate **source selection** from Memory synthesis inside Insomnia. Force an explicit retain/omit disposition for each user authority turn in the Episode, with authority kind and permitted grounding/assistant-authority role, before any consolidation into Memory candidates. The tracked grounding misses do not need more context: all four gold-v2 grounding antecedents are already Episode-local. The experiment must target source recall without reintroducing question/receipt/execution false positives.
2. Run fresh gold-v2 Luna-low comparisons after the source-selection split, including retain recall, omit cleanliness, provenance-role correctness, and strict semantic-target review. Only after selection behavior settles should the focused 32/48/64 worker confirmation run. Keep Luna-low as the current reasoning default; do not add a blanket post-extraction semantic reviewer or vector retrieval unless a later measured case demonstrates the need.
3. Add the grouped Memory/outcome publication boundary; mutation-ID replay already converges correctly after an interrupted split publication, but one physical correctness boundary is still required before production hardening.
4. Build the shared long-lived Continuity runtime, including automatic size/inactivity episode scheduling, persistent/background worker orchestration, cached endpoint capability verification, and host exposure of the narrow `create_memory` tool.
5. Extend provider transport beyond the implemented `openai-ready` embedding/General paths and implemented `openai-codex` device-code + General/Insomnia execution: add OAuth token refresh and later provider-native/local adapters; replace temporary JSON key persistence with an OS credential-store implementation before production.
6. Add Graph as its own semantic owner and reconnect Dream only after Memories are operational; Ego follows the shared runtime and memory retrieval path. **Dream rebuild constraint:** processing direction must not determine semantic edge direction. Candidate evaluation should treat the selected/source Memory only as the scheduling trigger, evaluate the Memory pair, and return the correct relationship orientation independently (for example, processing B against A must still be able to persist `A --factual--> B` when A supplies the fact used by B). Do not preserve the native Go coupling where Dream can emit only `SOURCE -> CANDIDATE`. This also requires redesigning the old source-owned replacement/retraction rule so an edge discovered from the opposite evaluation direction cannot later be retracted merely because it is not reproduced by the endpoint Memory's own pass.
7. Continue measurement-driven Archive packing/checkpoint/ANN work separately; do not block Insomnia bring-up on speculative storage acceleration.

## Implementation sequence

For each new store:

```text
define authority + records
    ↓
define local mutation/revision semantics
    ↓
attach global ordering only as needed
    ↓
prove reopen/recovery
    ↓
measure simple implementation
    ↓
add acceleration
```

## Acceptance criteria

A storage slice is complete only when ownership, persistent format, failure/recovery, focused tests, documentation coverage, and derived-vs-authoritative state are explicit.

## Open decisions

- Archive checkpoint representation, cadence, and retention.
- Exact pack target size, Archive record grouping, and compression codec; ADR 0004 fixes the bounded ancestry-aware shape but leaves these measurement-driven.
- Actual concurrent file append/version reservation mechanics.
- Broader calibration of compatibility policy v2 against representative routed/local embedding endpoints beyond the measured OpenRouter/Qwen3 route.
- Explicit vector-generation retirement/deactivation and retention policy.
- Quantization metadata and alternate packed representations for published generations.
- Whole-CVA historical materialization and restore/timeline representation now that Archive, Memories, and Vector Generations are concrete mutable semantic domains.
- Atomic grouped publication boundary for one completed Insomnia extraction attempt across zero-or-more Memory revisions and the durable processing outcome; mutation-ID replay already guarantees convergence, but the final grouped correctness boundary remains to be implemented.
- Retention/vacuum semantics for abandoned conversation/session branches.
- Dream pair-evaluation ownership: define how independently oriented edges are identified, attributed, reconsidered, and retracted once semantic edge direction is decoupled from the Memory that triggered processing. The rebuilt design must not require the semantically correct edge origin to be selected as the processing source before that edge can exist.

## Related docs

- [Architecture](architecture.md)
- [Current limitations](current-limitations.md)
- [Versioning and rollback plan](version-history-plan.md)
- [ADR 0003](decisions/0003-layered-version-clocks-and-local-ancestry.md)
- [ADR 0009](decisions/0009-expandable-model-switchboard.md)
- [ADR 0010](decisions/0010-encrypted-credential-objects.md)
- [ADR 0011](decisions/0011-detachable-repo-local-cli.md)
- [ADR 0012](decisions/0012-deterministic-episodes-and-insomnia-memory-authority.md)
- [ADR 0013](decisions/0013-immutable-memory-vector-bindings.md)

## Notes

Sequence can change with measurements; ownership boundaries should not.
