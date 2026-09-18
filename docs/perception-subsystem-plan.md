# Perception subsystem plan

Parent index: [Documentation index](INDEX.md)

Decision owners: [ADR 0033](decisions/0033-perception-entities-observations-and-ambiguity.md) and [ADR 0034](decisions/0034-cross-owner-relationship-graph-and-active-phy-privacy.md)

Shared temporal dependency: [ADR 0035 — Chronos](decisions/0035-chronos-shared-temporal-semantics.md)

## Status

Accepted architecture; implementation not yet started.

This document owns the implementation shape for Perception. ADR 0033 owns the architectural decision and rationale. Shipped behavior must move to current-state docs when implemented.

## Overview

Perception is the planned post-Dream semantic layer for durable Entities, sparse owner-qualified Relationships, and synthesized Observations. It uses bounded deterministic routing and inference rather than full-corpus reasoning.

## Purpose

Perception materializes semantic structure implicit in the organized Memory Web that is neither a directly remembered proposition nor an ordinary Dream relationship.

Initial semantic objects:

- **Entities** — durable canonical referents whose metadata and relationships may change without changing identity.
- **Relationships** — sparse typed semantic containers over owner-qualified Entity references. Relationships may carry bounded relationship-local derived state and may reference Entities across mounted REL/PHY Memory Webs without becoming independent Memory Webs.
- **Observations** — contingent higher-order propositions synthesized from existing semantic evidence.

Perception is not another full-corpus reasoning layer. Deterministic routing, Dream structure, mutation state, and wall time bound its inference.

## Pipeline

```text
Archive/source
    -> Insomnia semantic extraction + final Memory wording
    -> Insomnia Entity-mention enrichment
       -> Chronos Memory-level temporal assessment/inference when indicated
    -> durable Memory publication
    -> Dream Memory-Web organization (consumes Chronos)
    -> Perception 1: Entity synthesis/association/disambiguation
    -> Perception Relationship lane: sparse relationship synthesis/maintenance
    -> Perception 2: existing Observation contribution
    -> Perception 3: new Observation extrapolation
       -> Chronos higher-order temporal analysis when indicated
    -> Perception 4: Observation receptor generation
```

Observation reconsideration and ambiguity clarification are lifecycle/runtime mechanisms around these four routine passes.

## Insomnia enrichment prerequisite

After final authoritative Memory wording, Insomnia runs a separate metadata pass that extracts exact Entity mentions from title/content, persisted as field + UTF-8 byte span + verbatim text. A mention must be an **identity-bearing reusable referent**: its surface form must carry enough identity to be recognized again in another Memory without reconstructing the sentence that produced it. Bare/context-only references such as `the server`, `the file`, `the agent`, or `the repository` are excluded. Descriptive action/architecture phrases ending in `flow`, `lane`, `mechanism`, `request`, `response`, `seam`, `state`, or `status` are also screened out unless identity is carried by a code/API artifact rather than the prose phrase itself. Stable owner-local descriptive identities such as `write server`, `devtools window`, `the Vancouver office`, or `my brother` remain eligible when the modifier/relation consistently identifies the referent.

The implemented record is `MemoryRoutingMetadata`, bound to exact `MemoryId + MemoryBodyId`. Entity mentions are capped at 64 items and each emitted text value at 512 UTF-8 bytes. Write/reopen validation requires every mention span to match the immutable Memory body exactly. The metadata is clock-neutral and remains valid across metadata/lifecycle revisions because the Memory body cannot mutate in place.

Lexical locality is deliberately **not** model-extracted metadata. REL and PHY derive a disposable owner-local Memory lexical index directly from full current Memory title/content using the same deterministic tokenizer, inverted index, and coverage+density score as Archive lexical search. It rebuilds lazily from `memory_version`, persists no semantic state, and is the lexical routing/context-construction primitive Perception should use.

It does **not** resolve/create Entities, decide identity, assign Entity IDs/types, synthesize Observations, or create semantic authority beyond the Memory itself. Perception pass 1 remains the sole owner of Entity synthesis/association/disambiguation.

## Chronos temporal dependency

Perception does not own a separate temporal parser or temporal-inference stack. It consumes the shared Chronos subsystem defined by ADR 0035 after Perception has selected the bounded evidence relevant to an Observation synthesis or reconsideration.

Chronos may compare supporting Memory intervals/patterns, derive Observation valid-time semantics deterministically, expose temporal conflict/ambiguity, or invoke bounded temporal inference when deterministic interpretation is insufficient. Perception remains responsible for deciding what Observation proposition exists.

Deterministic Chronos products remain derived and need not be persisted. Any non-deterministically inferred temporal conclusion that becomes part of durable Perception state must be tied to the semantic inputs/version that justified it.

## Pass 1 — Entity synthesis, association, disambiguation

Perception consumes Insomnia-extracted Entity mentions against the post-Dream Memory Web.

Candidate flow:

1. Embed the mention in Memory context.
2. Search the owner-local Entity index.
3. No plausible candidate -> create an Entity only when the mention still carries identifiable reusable identity; generic/context-only mentions must not become durable Entities.
4. Clear match -> associate the Memory/mention.
5. Uncertain match -> compare against semantic centroids and Memory-Web context already attached to the candidate.
6. Use bounded inference only to confirm association, reject it, or preserve ambiguity.

Entities are durable referents. Sarah remains the same Entity through employer, role, location, relevance, and conversational changes.

Entity metadata/relationships are therefore mutable independently of Entity existence. The design should support multiple semantic centroids or equivalent multi-context representation where one referent spans materially different contexts.

## Relationship lane — sparse cross-owner relational state

After Entity resolution, Perception may materialize or update Relationships defined by ADR 0034.

A Relationship is not created for every Entity or every possible Entity pair. Candidate discovery is bounded to affected resolved Entities and evidence. Explicit durable relational facts may materialize a Relationship immediately; otherwise repeated relational evidence may need to cross a configured materiality/frequency threshold.

Relationship participants are owner-qualified Entity references and may span currently mounted REL/PHY Memory Webs. The Relationship itself remains owned by one REL or PHY. Visibility follows that owner, not the visibility of its participant Entities.

The runtime privacy rule is strict: only the active PHY contributes private Relationship state; authorized active RELs contribute their shared Relationship state. Non-active PHY Relationship layers are never traversed as bridges through shared Entities.

Relationship-local state may include relational Observations, directional perspective state, participant roles, classification, evidence references, and compact synthesized state. It must not acquire raw Memories, an independent Dream graph, Communities, or its own recursive Perception scheduler.

This lane does not alter Dream's same-owner Memory-to-Memory relationship authority.

## Pass 2 — Existing Observation contribution

This pass determines whether a new Memory materially bears on an existing Observation.

The semantic judgment is strictly pairwise:

```text
Memory M <-> Observation O
```

Possible results include support, challenge/contradiction, weakening, qualification, ambiguity, or irrelevance. This pass never synthesizes a new Observation.

### Candidate routing

Exhaustive `new Memory x all Observations` inference is forbidden because cost grows linearly with Observation population.

Existing Observations expose separately generated **routing receptors**: prospective-evidence descriptions of future facts that could materially bear on the Observation.

Each receptor is embedded independently and indexed back to its Observation. A new Memory queries that index for a bounded candidate set.

Exact deterministic routes may supplement receptor search, especially:

- shared Entity IDs;
- direct dependency/support links after an existing support object mutates.

Candidate Observation IDs are deduplicated before pairwise inference.

Routing receptors are routing metadata only. They are never evidentiary authority.

## Pass 3 — New Observation extrapolation

This pass asks whether the changed Memory Web now supports a higher-order proposition that no individual Memory explicitly represents.

It runs independently of pass 2: one Memory may support an existing Observation and also help establish another.

Observation extrapolation is bounded multi-Memory inference over Dream's structural organization.

### Multi-resolution Community hierarchy

The current flat Community layer is insufficient as an inference boundary for large RELs/PHYs.

Communities are **derived semantic structure**. They are inferred from the organization of the semantic Graph and therefore carry meaningful information about how Memories cohere, but they do not create, override, or replace Memory, Entity, Observation, or Relationship authority.

Community processing should expose multiple useful resolutions:

1. compute the normal owner-local Leiden partition;
2. recursively run Leiden inside a Community while genuine subcommunities exist;
3. build a Community meta-graph from cross-Community semantic relationships and derive coarser super-communities where genuine higher-level structure exists;
4. stop subdivision or aggregation when another level does not add meaningful structure.

Depth is dynamic in both directions. Leiden must not be forced to invent a split or grouping solely to satisfy an inference or presentation budget.

The hierarchy must be derived from persisted Graph/Community state, not from the temporary shard/reduction tree used by scan-and-merge. Execution topology is not semantic hierarchy.

Persisted sub-/super-Community structure may be named, assigned lineage, traversed, surfaced in the Knowledge Interface, and used for coarse -> normal -> fine retrieval or Perception routing. It remains descriptive organization: a Community-level node is not itself a proposition and cannot become an independent relationship authority.

### Oversized irreducible leaves

If the deepest genuine Community is still too large, Perception builds an ephemeral bounded processing neighbourhood around the new/changed Memory instead of persisting fake child Communities.

Neighbourhood construction may use bounded combinations of:

- strongest graph relationships;
- graph distance/locality;
- shared Entities;
- lexical locality from the deterministic owner-local Memory lexical index;
- semantic similarity as a backstop.

The neighbourhood is a processing window only, with no Community identity or semantic authority.

The same bounded-neighbourhood mechanism can support initial Observation discovery and later extrapolation triggered by changed Memories. Existing-Observation contribution remains the separate receptor-routed pairwise pass.

### Synthesis contract

The model receives the bounded set and may emit zero or more candidate Observations plus exact supporting semantic objects.

A candidate must be a useful abstraction, pattern, implication, or other higher-order proposition rather than a paraphrase of one supplied Memory.

Persisted Observations retain exact support/derivation lineage. Independent support paths remain distinguishable rather than collapsing to a count.

## Pass 4 — Observation routing-receptor generation

Every new Observation, and any materially rewritten Observation, receives a separate receptor-generation pass.

The pass generates prospective future facts that could materially:

- support it;
- challenge/contradict it;
- qualify it;
- indicate that its current interpretation warrants reconsideration.

Each receptor is embedded independently and mapped back to the Observation. Receptors must not be collapsed into one centroid because relevant evidence can occupy very different semantic regions.

Generation favors recall over precision: a false positive costs one bounded pairwise comparison; a false negative may hide relevant evidence indefinitely.

## Observation lifecycle

Observations are retained historical semantic objects and should normally be archived rather than deleted.

### Mutation-driven reconsideration

Relevant mutations accumulate against an Observation. Crossing the configured threshold triggers semantic reconsideration.

Mutation only establishes that inference is warranted. Reconsideration may keep the Observation active, qualify/supersede it, expose contradiction/ambiguity, or mark it stale.

### Wall-time reconsideration

Observations also receive wall-time checkpoints for propositions that may silently become outdated.

At a checkpoint, deterministic machinery asks whether any relevant mutation occurred since the last semantic reconsideration:

- **none** -> mark stale and remove from priority consideration without inference;
- **one or more** -> run semantic reconsideration.

A mutation does not imply freshness. Old supporting evidence may still lead reconsideration to mark an Observation stale.

`stale` means non-priority/currentness-uncertain, not false. Wall time alone never invalidates or deletes an Observation; contradiction/supersession requires evidence.

## Entity and Observation ambiguity

Perception may persist unresolved Entity or Observation ambiguity instead of forcing an unsafe decision.

When a later user turn deterministically intersects one, the runtime injects an instruction-bearing Perception addendum that:

- identifies the ambiguity;
- prohibits assuming the unresolved identity/interpretation;
- requires the agent to seek minimum clarification when the current turn does not already resolve it.

The user's answer returns to Perception for resolution or continued ambiguity.

Initial scope stops here. Do not generalize this into a universal curiosity/open-question system until Entity/Observation ambiguity demonstrates that need.

## Ownership invariants

- Insomnia owns source-grounded Memory extraction and Entity-mention metadata extraction; it consumes Chronos during eligible Memory processing. Lexical Memory routing is deterministic derived indexing, not Insomnia model output.
- Dream owns Memory-to-Memory semantic relationship authority and Community organization; it consumes Chronos for temporal candidate/context reasoning.
- Chronos owns shared temporal detection, normalization, parsing, resolution, comparison, and bounded temporal-inference mechanics; it owns no semantic objects or transaction clock.
- Perception owns Entity, Relationship, and Observation semantic objects plus Entity/Observation ambiguity state; it consumes Chronos for Observation temporal interpretation.
- Relationship participants are owner-qualified Entity references; Relationship ownership and visibility are independent of participant-Entity visibility.
- Only the active PHY may contribute private Relationship state to normal runtime composition; authorized REL Relationship state may be shared/portable across users.
- Community detection and hierarchy remain derived semantic organization, never independent semantic authority.
- Sub-/super-Community levels may be persisted, named, lineaged, traversed, and used for routing, but cannot create or override Memory/Entity/Observation/Relationship truth.
- Scan-and-merge reduction intermediates are execution machinery and must not be promoted into semantic hierarchy without an independent graph-derived hierarchy pass.
- Processing neighbourhoods are ephemeral and never fake Communities.
- Routing receptors are metadata, never evidence.
- Pairwise Observation contribution and multi-Memory extrapolation remain separate contracts.
- Observation support lineage resolves to real owner-qualified semantic objects.
- Entity identity is durable; Observation currentness is contingent.

## Implementation sequence

### A — Entity metadata + deterministic Memory lexical index — implemented 2026-09-17

- `MemoryRoutingMetadata` defines bounded exact Entity mentions over immutable Memory text; lexical routing comes from the disposable Memory lexical index over full Memory title/content.
- Insomnia contract `v3-4` owns the post-wording Entity enrichment pass; configured/runtime-host execution uses `models.insomnia_metadata` when present and otherwise effective main Insomnia. Extraction targets identity-bearing reusable referents rather than generic noun phrases; a narrow deterministic guard drops known bare/context-only generic surfaces plus multiword action/architecture phrases headed by `flow`, `lane`, `mechanism`, `request`, `response`, `seam`, `state`, or `status`. The model may select one occurrence when identical surface text has mixed semantics, but durable routing metadata remains exact title/content spans only.
- REL Project results embed Entity routing metadata atomically in `CVAINSC5`; PHY/User results persist the same clock-neutral attachment beside the routed Memory.
- New Entity routing attachments use `CVAMRTE2`; legacy `CVAMRTE1` remains readable and its obsolete model-generated lexical terms are discarded on decode.
- REL and PHY expose disposable Memory lexical search over complete current non-archived title/content, using the same deterministic lexical machinery as Archive search and no model call.
- Reopen validates `MemoryId + MemoryBodyId` binding and exact Entity source text. Migration/reconciliation replay the attachment, identical writes are idempotent, and conflicts fail closed.

The next implementation milestone is **B — Entity owner and pass 1**. ADR 0036 establishes the shared typed semantic-Graph direction: Arcana remains the graph kernel, the Graph catalogue now addresses typed semantic nodes, and Milestone B will add the first Entity-backed relation family without moving Entity payload/lifecycle authority into Graph.

### B — Entity owner and pass 1

- **Implemented:** durable Entity IDs, revisions/persistence, normalized one-to-many aliases, mutable semantic metadata, REL/PHY reopen, migration/reconciliation replay, and resolution-reference validation.
- **Next:** generalize Graph relation endpoints/persistence from the current Memory-only mutation format to validated typed semantic-node endpoints.
- Represent Memory↔Entity topology through the shared Graph rather than a parallel Entity adjacency store.
- Add Entity vector/centroid indexing.
- Implement create/associate/disambiguate behavior.
- Persist unresolved Entity ambiguity.

### B2 — Relationship owner and synthesis lane

- Define Relationship IDs, owner/versioning rules, participant `EntityRef`s, roles/cardinality, classification, and evidence/support references.
- Implement sparse Relationship materialization from explicit or repeated relational evidence without an all-Entity-pairs path.
- Permit cross-owner Entity references while keeping Relationship visibility bound to its owner.
- Implement active-PHY privacy and authorized-REL runtime composition fixtures.
- Define bounded relationship-local Observations/perspectives without creating nested Memory Webs.

### C — Multi-resolution Communities and local neighbourhoods

- Extend Community representation to a derived multi-resolution hierarchy.
- Add recursive sub-Communities only while genuine internal structure remains.
- Build a Community meta-graph from cross-Community semantic relationships and derive coarser super-Communities where meaningful higher-level structure exists.
- Keep the hierarchy graph-derived; do not persist scan-and-merge reduction intermediates as semantic structure.
- Preserve or extend lineage, semantic naming, and stable inspection semantics at each persisted hierarchy level.
- Expose coarse -> normal -> fine routing for Perception and later retrieval/Ego integration.
- Detect oversized irreducible leaves.
- Implement ephemeral bounded local-neighbourhood construction for leaves that still exceed inference budgets.

### D — Observation owner and pass 3

- Define Observation persistence, support lineage, lifecycle, and chronology.
- Implement bounded multi-Memory extrapolation.
- Integrate Chronos over the bounded support set for deterministic valid-time synthesis and unresolved temporal fallback.
- Reconcile semantic duplicates without losing independent support paths.

### E — Receptors and pass 2

- Define receptor records/vector bindings.
- Implement receptor generation for new/materially rewritten Observations.
- Build bounded receptor-vector retrieval.
- Implement strict pairwise Memory-to-Observation contribution.

### F — Reconsideration and ambiguity runtime

- Add mutation accounting and thresholds.
- Add deterministic wall-time checks/stale transition.
- Add semantic reconsideration inference.
- Add Entity/Observation clarification triggers and runtime injection.

### G — Scale and quality validation

- Measure receptor recall and false positives on cross-domain evidence cases.
- Measure inference cost as Memory/Entity/Observation populations grow.
- Validate recursive Community/local-neighbourhood coverage on large RELs.
- Validate stale lifecycle without false deletion/invalidation.
- Tune thresholds only from measured workloads.

## Open implementation decisions

- Persistent schemas for Entity, Relationship, Observation, ambiguity, and receptor state.
- Owner-qualified `EntityRef` representation, dangling-reference behavior, Relationship roles/cardinality, and materialization thresholds.
- Dedicated Perception model route vs General/Dream fallback.
- Entity candidate thresholds and multi-centroid maintenance.
- Multi-resolution Community criteria: recursive split stopping rules, Community meta-graph construction/weighting, super-Community resolution/stopping rules, and processing-neighbourhood budget.
- Observation derivation/inference taxonomy beyond support lineage.
- Receptor vector-index implementation and candidate limits.
- Mutation-accounting boundaries and wall-time policy.
- Observation duplicate/canonical reconciliation mechanics.
- Retrieval/Ego treatment of Entities and Observations once implemented.

## Notes

This remains future architecture. The earlier derived-semantic-nodes exploration is retained as design history; ADRs 0033–0035 and this plan own the current accepted direction.

## Related docs

- [ADR 0033](decisions/0033-perception-entities-observations-and-ambiguity.md)
- [ADR 0034 — cross-owner Relationship graph and active-PHY privacy](decisions/0034-cross-owner-relationship-graph-and-active-phy-privacy.md)
- [ADR 0035 — Chronos shared temporal semantics](decisions/0035-chronos-shared-temporal-semantics.md)
- [Chronos subsystem plan](chronos-subsystem-plan.md)
- [ADR 0012 — Insomnia memory authority](decisions/0012-deterministic-episodes-and-insomnia-memory-authority.md)
- [ADR 0025 — derived Graph communities](decisions/0025-owner-local-derived-communities.md)
- [ADR 0030 — Dream maintenance](decisions/0030-provenance-anchored-dream-maintenance.md)
- [ADR 0032 — Community lineage](decisions/0032-community-lineage-and-name-continuity.md)
- [Dream design and validation record](dream-implementation-plan.md)
- [Roadmap](roadmap.md)
