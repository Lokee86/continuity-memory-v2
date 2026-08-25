# Dream implementation plan

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns the current implementation plan for Dream over the implemented Graph owner. It records the redesign derived from review of CTX Dream, the previous Continuity Dream implementation, the current Graph foundation, and the August 2026 planning discussion.

Dream's bounded candidate-retrieval, pair-classification, independent-verification, and accepted Graph-publication seams are now implemented; duplicate-chain and lifecycle behavior remain planned. This document freezes the intended semantic shape and tracks the remaining staged implementation.

## Overview

Dream is being rebuilt as a pair-oriented semantic layer over Memories, Memory Vectors, source provenance, and the existing Graph owner. Candidate retrieval, transient pair classification, transient independent verification, and atomic accepted-relation publication are implemented. Remaining work proceeds through duplicate/supersession handling, deterministic temporal interpretation, lifecycle policy, and only then long-lived scheduling/runtime integration.

## Responsibility

Insomnia answers:

> What durable Memories should exist from source material?

Dream answers:

1. Which Memory pairs might be related?
2. What semantic relationship exists between them?
3. What direction does that relationship have, if any?
4. What lifecycle consequences follow from the resulting graph?

Graph owns accepted Memory-to-Memory relationship authority. Dream discovers and evaluates those relationships; it must not become a second generalized relationship store.

## Core redesign: pair-oriented evaluation

Dream must evaluate a Memory pair independently of which endpoint caused the work.

For an unordered candidate pair `{A, B}`, the evaluator may conclude:

- no relationship;
- topical;
- factual `A -> B` or `B -> A`;
- causal `A -> B` or `B -> A`;
- recurrent;
- duplicate;
- supersedes `A -> B` or `B -> A`;
- later, references if its exact semantics justify inclusion.

Processing direction must never determine semantic direction. If B is the newly processed Memory but A supplies a fact used by B, Dream must be able to publish `A --factual--> B` immediately.

This removes the primary reason the previous implementation needed source-owned edge replacement and broad reverse-reconsideration machinery.

## Relationship vocabulary

The implemented classifier vocabulary is:

- topical;
- factual;
- causal;
- recurrent;
- duplicate-of;
- supersedes;
- none.

`references` remains outside the classifier contract pending exact semantic policy.

Structural parent relationships remain deterministic structural facts rather than ordinary Dream inference.

Milestone B returns exactly one primary semantic proposal (or `none`) per pair evaluation. This is an evaluator contract, not yet a persistent-Graph cardinality decision: structural facts and any later explicitly orthogonal relationship class may still require coexistence policy before publication is implemented.

## Duplicate handling

Duplicate history remains a predecessor chain, not a clique and not a connected-component-only representation.

Conceptually:

```text
newest -> previous equivalent -> previous equivalent -> oldest
```

Example:

```text
D -> C -> B -> A
```

If an equivalent observation X belongs temporally between C and B:

```text
before:
C -> B

after:
C -> X
X -> B
```

Once the insertion position is known, the chain mutation is constant-size: insert the new predecessor link and retarget at most the immediate newer link. There is no shifting of the rest of the chain.

### Indexed-chain implementation

The semantic structure does not have to be a literal unindexed linked list. Dream/Graph may maintain a hybrid lookup structure that locates the temporal insertion point without walking the chain.

The intended separation is:

```text
lookup/index machinery
    temporal key -> MemoryId / neighboring observation

semantic duplicate history
    predecessor duplicate links
```

The temporal key must use authoritative source chronology, with a deterministic tie-breaker where timestamps collide.

An ordered map/B-tree gives logarithmic predecessor/successor lookup and constant-size chain rewiring. A more specialized date/time index may later provide expected constant-time lookup if measurements justify it. The exact index is an implementation choice; duplicate semantics must not depend on it.

The previous implementation became complicated because it combined durable concurrency, source-owned replacement, out-of-order discovery, and chain repair. Those operational concerns must not be mistaken for complexity inherent in the duplicate model itself.

## Confidence and verification

Do not treat model self-reported numeric confidence as semantic authority.

The implemented staged design is:

```text
Pass 1: classify the pair
    -> proposed relation + direction + evidence

Pass 2: independently verify the proposed conclusion
    -> relation supported: yes / no / uncertain
    -> direction supported: yes / no / uncertain
    -> evidence supported: yes / no / uncertain

Continuity derives:
    any no        -> reject
    else uncertain -> uncertain
    else           -> accept
```

The verifier receives the same canonical Memory pair plus the proposed relation/direction and classifier evidence. It receives no classifier confidence value and is explicitly instructed not to defer to the classifier or its model identity. Reverse processing order produces the same verifier payload just as it does for classification.

The default `DreamVerificationPolicy` pays the second model call only for `duplicate_of` and `supersedes`, because those conclusions will drive duplicate/supersession lifecycle behavior. `broad_semantic()` additionally verifies factual, causal, and recurrent relationships so corpus/live measurement can determine whether their error reduction justifies permanent second-pass cost. Topical and `none` remain single-pass/non-verifiable by default.

No numeric confidence participates in the verifier contract or derived verdict. Whether factual, causal, and recurrent eventually use verification by default remains measurement-driven.

## Temporal determinism

Dream temporal reasoning must distinguish source chronology from Memory bookkeeping chronology.

`Memory.created_at` records when a Memory record was produced. It is not evidence for when the source statement, event, decision, or observation occurred.

Example:

```text
source turn timestamp: 2024-05-17   <- semantic chronology
Memory created_at:      2026-08-24   <- import/extraction bookkeeping
```

The deterministic temporal baseline therefore comes from two sources:

1. **Source time** — authoritative timestamps of the source turns and/or Episode range referenced by Memory provenance.
2. **Content time** — explicit temporal expressions deterministically extracted from Memory title/content.

These must remain distinguishable.

### Timestamp-anchored interpretation

Relative expressions should be interpreted against source-turn chronology where possible.

For example, a turn timestamped May 17 saying `next Tuesday` can be deterministically resolved relative to that turn timestamp. It must not be resolved relative to Memory creation/import time.

Initial temporal flow:

```text
Memory provenance
    -> source turn timestamp(s) / Episode range

Memory title/content
    -> deterministic temporal parser

source chronology + explicit expressions
    -> deterministic temporal interpretation
    -> anchors/ranges/recurrence evidence
```

Model enrichment is optional and only for genuinely unresolved temporal semantics. Deterministic verification of any model-enriched temporal result remains required.

Do not restore the old CTX expanded temporal-dimension/gravity representation. Do not mix record creation time into semantic event chronology.

## Memory identity and re-evaluation

Memory bodies are immutable and vector bindings already use `MemoryBodyId`. Dream should therefore avoid model calls merely because lifecycle or other metadata produced another Memory revision with unchanged semantic content.

Semantic evaluation should bind primarily to immutable body identity plus only those metadata/provenance fields that materially affect Dream semantics.

A later compact semantic fingerprint may include:

- Memory body identity;
- relevant category/type;
- relevant provenance/source chronology;
- Dream policy version.

Lifecycle-only revisions must not create semantic hot loops.

## Candidate discovery

Milestone A is implemented as a read-only bounded retrieval seam.

Current lanes:

1. exact semantic comparison of the source Memory's stored vector against current Memory-body vectors under one Compatibility Profile;
2. a bounded prior/older semantic quota based on authoritative source timestamps so old duplicates or corrections are not hidden by newer high-similarity results;
3. deterministic lexical/metadata overlap over Memory title/content/category/type/parent.

The source and candidates are returned with immutable body identity, authoritative source timestamp when available, and all active Graph relations touching each Memory. Archived Memories are excluded. Candidates without vectors may still enter through the lexical/metadata lane. Retrieval performs no new embedding or model call and is deterministic for unchanged CVA state/configuration.

Existing Graph context is attached for later pair evaluation rather than used as a separate ranking lane in the initial implementation. A dedicated temporal candidate lane remains deferred until deterministic temporal indexing exists. Recurrence should be treated as temporal evidence/candidate discovery rather than a completely separate LLM pipeline.

Do not restore CTX LLM-generated search keywords or add LLM candidate triage until candidate-recall measurements demonstrate a need.

## Pair classification — implemented

Milestone B canonicalizes every distinct pair by stable `MemoryId` before inference. The model therefore always receives the same A/B ordering for the same two Memories regardless of which Memory triggered processing. This removes processing direction from the classifier contract rather than asking the model to compensate for it.

The strict v1 output proposes one relation from `none`, `topical`, `factual`, `causal`, `recurrent`, `duplicate_of`, or `supersedes`, plus a direction. `topical`, `recurrent`, and `duplicate_of` are semantically undirected; `factual`, `causal`, and `supersedes` require A→B or B→A. Source timestamps and current Graph relationships are included as context, but the prompt explicitly forbids treating chronology alone as supersession/causality or treating existing Graph state as proof.

Every non-none proposal must include exactly one verbatim evidence quote from each Memory. Continuity validates relation/direction compatibility, unique A/B evidence coverage, and literal quote membership after the model call. Invalid structured conclusions are rejected before any downstream stage can observe them as accepted semantics.

The classifier is transient and read-only. It records the model identity in the returned result but persists nothing. Independent verification remains transient; accepted non-duplicate conclusions can now be reconciled into Graph atomically. Lifecycle mutation remains a later stage.

## Lifecycle

Current planned lifecycle remains:

```text
extracted
    -> knowledge
    -> canonical

any active state
    -> archived
```

`archived` is retained inactive history, especially for superseded Memories and non-representative duplicates. It is not a replacement for historical storage and does not delete evidence.

Initial expectation:

- successful bounded first Dream processing may move `extracted -> knowledge`, even if no relationship is found;
- `knowledge -> canonical` must be driven by semantic authority/current-representation policy, not an arbitrary scalar quality threshold;
- duplicate and supersession graph conclusions may drive archival only after required verification;
- canonical remains supersedable and duplicatable.

Exact canonical-promotion policy requires representative fixtures/corpus evidence before implementation.

## Supersession

Preferred semantic direction:

```text
NEW --supersedes--> OLD
```

Graph is authoritative for the relationship. `Memory.superseded_by`, if retained, is a projection derived from current graph state rather than an independent source of truth.

Removing or replacing a supersedes relationship must reconcile the current graph before changing lifecycle state; it must not blindly reactivate an older Memory.

Memory creation time is not proof that one semantic statement supersedes another. Supersession must come from correction/replacement meaning and applicable source/content chronology.

## Reconsideration

Do not port the old broad reverse-reconsideration/task system by default.

Pair-oriented evaluation removes the source-direction bug that created much of that machinery. Re-evaluation should initially occur only when justified, such as:

- a pair's semantic inputs materially change;
- an explicit Dream policy/model migration requests rescan;
- new candidate evidence causes a previously unevaluated pair to be discovered.

Add broader reconsideration only if measurements expose a real semantic case that cannot be handled by pair-oriented evaluation and bounded rescans.

## Candidate semantic context and Graph publication

Dream does not require a separate durable pair-evaluation record merely to classify candidate relationships.

A Dream candidate should carry the semantic context already owned by existing Continuity records:

- the candidate Memory/body;
- current lifecycle/category/type metadata relevant to Dream;
- provenance and authoritative source-turn/Episode timestamps;
- the candidate's current Graph relationships/neighbor context.

The Memory being processed should be presented with the same context. Dream can then evaluate the pair from those two contextualized Memories.

Classification and verification outputs may remain transient until an accepted relationship is published. Graph remains the authoritative durable owner of accepted Memory-to-Memory relationships. Model identity, policy version, raw confidence, verifier traces, or other evaluation diagnostics should not be added to Graph merely because they could be recorded; add such persistence only when a concrete reproducibility, migration, audit, or invalidation requirement demonstrates the need.

Milestone D settles the required Graph publication behavior for primary Dream relations. `Cva::set_memory_relations` publishes one or several edge changes as a single Graph semantic transaction; `publish_dream_pair` uses that boundary to preserve semantic direction, retract replaced primary pair relations, and activate the new conclusion without exposing contradictory intermediate state. Topical/recurrent use reciprocal edges; factual/causal/supersedes preserve classifier direction; `none` clears primary Dream state. Duplicate persistence is deliberately deferred to the chronological predecessor-chain milestone.

## Implementation sequence

### Phase 1 — candidate context and Graph publication seam — implemented

Candidate context, evaluator-facing pair direction semantics, and publication mechanics are implemented: Memory/body identity, relevant metadata/provenance/source chronology, attached active Graph data, canonical MemoryId A/B ordering, atomic Graph relation batches, safe primary-pair replacement/retraction, and classifier-direction preservation. No separate durable pair-evaluation store was introduced.

### Phase 2 — Memory candidate retrieval — implemented

Milestone A currently provides:

- exact stored Memory-vector semantic comparison with no embedding call;
- source-timestamp-based prior semantic coverage;
- deterministic lexical/metadata fallback, including unvectorized candidates;
- deterministic bounded lane fusion and stable tie-breaking;
- archived exclusion and attached Graph context;
- focused synthetic tests for semantic ranking, prior coverage, deterministic repeatability, lexical fallback, archival filtering, and Graph-context attachment.

The next validation step is candidate-recall inspection on representative real Insomnia-generated Memories. Do not add an LLM triage stage unless that measurement demonstrates a need.

### Phase 3 — pair classifier — implemented

Milestone B currently provides:

- optional dedicated `models.dream` switchboard route with General fallback;
- canonical MemoryId A/B ordering before every model call;
- strict structured relation/direction/evidence output;
- bounded evaluation over Milestone A candidate sets;
- deterministic rejection of invalid direction combinations and non-verbatim evidence;
- no Graph or lifecycle side effects.

### Phase 4 — verification pass — implemented

Milestone C currently provides:

- a separate verifier call over the same canonical pair, proposal, and verbatim classifier evidence;
- three categorical checks (`relation_supported`, `direction_supported`, `evidence_supported`) rather than numeric model confidence;
- deterministic derivation of `accept`, `reject`, or `uncertain` from those checks;
- default verification for `duplicate_of` and `supersedes` only;
- opt-in broad verification for factual, causal, and recurrent measurement;
- pair/classification identity validation and reverse-processing-order invariance;
- no Graph or lifecycle side effects.

Representative live/corpus measurement must decide whether factual, causal, and recurrent should move into the default verification policy.

### Phase 5 — accepted Graph publication — implemented

Milestone D currently provides:

- one atomic Graph transaction for multi-edge pair reconciliation;
- reciprocal Graph edges for topical/recurrent semantic symmetry;
- exact classifier direction for factual/causal/supersedes;
- `none` reconciliation that removes only Dream-owned primary semantic relations;
- preservation of structural-parent/references Graph state;
- verification gating before publication;
- idempotent no-op publication when the pair is already in the desired state;
- verified duplicate publication deferred until predecessor-chain semantics are implemented.

The next end-to-end milestone adds duplicate/supersession lifecycle behavior and the `extracted -> knowledge` completion transition around the now-working inference/publication pipeline.

### Phase 6 — duplicate and supersession lifecycle

- implement indexed duplicate predecessor-chain insertion;
- derive duplicate observation/history information from the chain;
- archive non-representative duplicate Memories according to policy;
- reconcile supersession;
- begin measured canonical-promotion policy.

### Phase 7 — deterministic temporal layer

- obtain source turn/Episode timestamps through provenance;
- deterministically parse content-time expressions;
- resolve relative expressions against source timestamps;
- retain anchors/ranges and recurrence patterns;
- add temporal candidate lookup/indexing;
- require deterministic verification for any later model enrichment.

### Phase 8 — bounded reconsideration if required

Only add cases demonstrated by tests/production evidence. Do not blindly reproduce the previous reverse-trigger architecture.

### Phase 9 — long-lived runtime integration

After semantic behavior is correct:

- continuous scheduling;
- retry/backoff;
- concurrency;
- resumable explicit rescans/migrations;
- status and control surfaces.

### Phase 10 — optional sophistication

Only when measurements justify it:

- temporal model enrichment;
- LLM candidate triage;
- broader verifier voting;
- mature-memory periodic re-evaluation;
- more elaborate canonical policy;
- graph clustering/community signals.

## Deliberately rejected carry-over

Do not reproduce these CTX/previous-Dream mechanisms by default:

- source-owned replacement of all generated edges;
- source-to-candidate processing direction as semantic direction;
- broad reverse reconsideration merely to repair direction;
- LLM-generated search keywords as primary candidate discovery;
- model self-confidence as lifecycle authority;
- scalar semantic quality-score multiplication;
- snapshot lifecycle state;
- mandatory LLM temporal review;
- expanded temporal dimensions/gravity/date coercion;
- Memory creation time as event chronology;
- separate recurrence LLM pipeline per pair;
- periodic cooldown/backoff as the initial semantic architecture.

## Open decisions at current review point

The review has not yet frozen:

1. which relationship kinds are represented as symmetric versus directed in persistent Graph storage;
2. persistent Graph coexistence/cardinality policy beyond the classifier's one-primary-proposal contract;
3. exact independent-observation/corroboration policy across duplicate chains;
4. exact canonical-promotion rules;
5. whether second-pass verification becomes universal for all non-topical semantic relations after measurement;
6. exact temporal index representation and whether it is persisted or cheaply rebuilt;
7. precise semantics for `references`.

The next implementation milestone is duplicate/supersession handling: implement the chronological duplicate predecessor chain and lifecycle consequences without changing the already-settled pair-oriented classifier/verifier/publication contract.

## Related docs

- [Architecture](architecture.md)
- [Rust API](api.md)
- [Roadmap](roadmap.md)
- [Architectural invariants](invariants.md)
- [Behavioral contracts](behavioral-contracts.md)

## Notes

This plan may describe both completed Dream milestones and future stages, but shipped behavior must also be reflected in the current-state documentation owners. Graph remains the durable relationship authority throughout the staged implementation.
