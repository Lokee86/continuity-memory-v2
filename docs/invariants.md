# Architectural Invariants

Parent index: [Documentation index](INDEX.md)

## Purpose

This document defines the governing architectural constraints for the Reliquary Memory v2 rebuild.

## Overview

These constraints are intentionally stronger than convenience abstractions. Implementation status remains owned by current architecture, roadmap, and limitations documents.

## Invariants

1. **One file, several databases.** A Reliquary `.rel` contains explicit purpose-built databases, not one generalized semantic database; legacy `.cva` is only the prior Project Reliquary physical form.
2. **One mutable domain, one owner.** Each independently mutable semantic domain has one concrete owner.
3. **Stable IDs cross database boundaries.** Cross-indexing uses IDs, not shared mutable semantic objects.
4. **The physical substrate has no semantic dependency knowledge.**
5. **No generalized semantic dependency engine.**
6. **Derived state remains derived.** Indexes/checkpoints/caches cannot become authority by persistence alone.
7. **Runtime work state is not semantic authority.**
8. **Prefer concrete duplication over speculative semantic generalization.**
9. **Previous Reliquary code has no automatic authority.**
10. **Architecture outranks experimental-format compatibility.**
11. **Archive text has one authority.** Content bytes live in content-addressed objects referenced by nodes.
12. **Conversation ancestry is local.** Node `parent_id` cannot cross conversation IDs.
13. **Concurrent conversations do not acquire semantic ancestry from physical ordering.**
14. **Branch/session heads are revisions, not mutable in-place records.** Existing heads advance only to descendants.
15. **Reviving an old conversation creates a new local branch identity; it does not rewind an existing branch or the Archive.**
16. **Fragment identity is branch-neutral.**
17. **Live fragment materialization is append-only.**
18. **Global version is ordering only.** It is not a CVA root or semantic dependency identity.
19. **Archive version is a local watermark only.** It orders Archive mutations but is not Archive ancestry.
20. **Global and Archive clocks are independent.** Archive versions are contiguous; global versions between Archive records may have gaps.
21. **Normal Archive writes stay Archive-local.** They cannot require reading/republishing unrelated database heads.
22. **Whole-Archive historical state is a cut, not one giant record.** `A=N` identifies records/revisions visible through that watermark.
23. **Checkpointing and historical semantics are distinct.** Checkpoints accelerate reconstruction; clocks/revisions define historical ordering.
24. **Persistent ordering uses exact integers.** No floating-point version identity.
25. **Unversioned semantic payloads are inert.** Reopen may observe incomplete node/branch/fragment payloads physically, but they cannot enter Archive state without valid `ArchiveRecordVersion` metadata.
26. **CVA composition has one physical owner.** `Cva` owns the single Container handle; concrete databases do not open competing handles or independently rescan the file during CVA reopen.
27. **Shared scan does not imply shared semantics.** Archive, Memories, Graph, packed-vector, Archive-Vector, profile, and generation rebuild logic consume the same physical payload stream but classify and validate their own records explicitly.
28. **Packed-vector matrices are immutable backing objects.** Identity includes schema plus exact matrix bytes; equal objects deduplicate and raw matrix creation consumes no semantic version ticket.
29. **Archive Vectors own row-to-Archive identity only.** An Archive-Vector set binds one packed matrix's row ordinals to an ordered list of existing unique `FragmentId`s.
30. **Archive-Vector identity is profile-independent.** Profiles, models, metrics, normalization, coverage watermarks, and active-generation state cannot be embedded in the row-binding object.
31. **Vector backing objects are not publication.** Creating a packed matrix, Archive-Vector set, or Compatibility Profile consumes no semantic version ticket.
32. **Compatibility Profiles own endpoint-independent vector-space contracts.** They contain dimensions, normalization, compatibility/probe policy, and stored probe references; provider/model/route/revision provenance is outside this owner.
33. **Vector Generations own vector semantic publication.** A generation binds one profile to one Archive-Vector set and source Archive watermark; the latest generation per profile is current.
34. **Vector version is a local watermark only.** It is dense within VectorGenerationStore and is not ancestry or a cross-database dependency identity.
35. **Archive and vector clocks are independent.** Their mutations interleave only through CVA-global ordering.
36. **A CVA-global version has at most one semantic claimant.** Reopen rejects a global ticket claimed by more than one mutable semantic owner, including Archive, Memories, Graph, and Vector Generations.
37. **Generation coverage must be truthful.** A generation source watermark cannot predate any mapped fragment, exceed current Archive state, or regress for that profile.
38. **Incomplete generation publication is inert.** A generation payload without valid generation-version metadata cannot become current semantic state.
39. **Compatibility is behavioral and tolerant.** Endpoint compatibility is decided by the profile contract plus corresponding probe-vector cosine thresholds; provider/model labels and exact probe-byte equality cannot decide compatibility.
40. **Retrieval never mixes vector spaces.** Exact semantic search resolves one selected Compatibility Profile's current Vector Generation, searches only that generation, and maps only its row bindings back to Archive fragments.
41. **Published generation representation must be interpretable.** Until alternate scalar/quantization semantics are explicitly defined, Vector Generations may reference only `f32` packed matrices even though raw PackedVectorStore objects support other scalar types.
42. **Retrieval is derived and read-only.** Lexical scoring, semantic search, hybrid fusion, deduplication, and diversification persist no authority and consume no semantic version ticket.
43. **Local configuration is current state, not semantic history.** `reliquary.cfg` is separate from `.rel`, uses replaceable logical objects, and consumes no semantic version ticket.
44. **Configuration replacement does not accumulate history.** Saving writes one complete validated current image and atomically replaces the prior file; superseded config objects are not retained.
45. **Model routing is not vector compatibility.** Switchboard provider/model/URL selections are machine-local integration policy; Compatibility Profiles remain the sole durable vector-space compatibility contract.
46. **Provider secrets are separate encrypted config objects.** Model routes reference credentials by stable ID; API keys and OAuth tokens are not stored in model-route payloads or Reliquary state.
47. **Credential ciphertext is authenticated to its logical key.** Moving or modifying encrypted credential bytes must fail decryption rather than silently rebind a secret.
48. **Executable model routing requires matching auth.** A runtime switchboard cannot resolve a route whose credential is missing or whose auth kind does not match the provider.
49. **Current branch inventory is derived read state.** Enumerating current branches exposes the latest visible branch revision and adds no new Archive authority or persistent record.
50. **The CLI owns no semantics.** The detachable `cli/` package may parse/prompt/render/dispatch only public library operations; configured runtime composition, route fallback, persistence workflows, retrieval policy, and default policy values remain in the core library.
51. **Source attachments are intrinsic source-event data.** Importers/runtimes submit a turn and its attachments together; they do not persist a file and then separately reconstruct its source-turn provenance.
52. **A source turn and its attachments publish together.** The node, attached file manifests, and source-to-file relationship become visible through one Archive semantic mutation; unversioned staged content or turn payloads are inert.
53. **Later file relationships remain explicit.** A file-to-Memory relationship crosses owners by stable `FileId`/`MemoryId` and does not transfer file authority into Memories.
54. **Interaction transports do not define semantic storage.** Warlock-native interaction, ACP, imports, APIs, and future provider adapters must normalize above concrete semantic owners; protocol-specific message models cannot become Archive authority by convenience.
55. **Product management is not a generalized semantic store.** A Reliquary management surface may compose explicit owner operations, but it cannot bypass owner validation or introduce a generic mutable object/root/dependency model.
56. **External protocols are optional product integrations.** Warlock-native use of customer-owned Reliquary state cannot require ACP, MCP, an IDE, or another external agent host.
57. **Incomplete live messages may be durable transcript state without becoming source history.** User-visible assistant text may be checkpointed into the interaction-stream journal, but text/attachments become Archive authority only when one completed message crosses the durable turn-ingestion boundary.
58. **Live session continuation is explicit.** Reopening a session that already has durable Archive history requires an explicit durable resume message; the session runtime cannot silently create an unrelated second root for that history.
59. **Source acknowledgement does not depend on background scheduling.** A completed live turn is durably acknowledged before Episode/Insomnia scheduling, and a scheduling failure cannot revoke or obscure that receipt.
60. **Every current durable owner is born identified.** New REL/PHY creation persists one UUID in the typed container header before returning.
61. **Owner IDs bear provenance.** Canonical durable IDs derive from authoritative scope/type plus UUID: `proj-`, `org-`, `con-`, or `phy-`; filenames and optional adapters are not identity.
62. **Displayed streamed text must already be durable.** A host must not expose an assistant delta as committed presentation state until the corresponding interaction-stream checkpoint has synchronized successfully.
63. **Recovered streaming state is interrupted state.** A persisted `Streaming` checkpoint is considered actively streaming only while the matching in-memory message exists; after runtime loss it is recovered as `Interrupted`.
64. **Completion supersedes the stream journal by stable message ID.** Once a checkpointed message is published as a completed Archive node, transcript resolution uses the Archive turn and must not duplicate the journal copy.
65. **Interaction-stream checkpoints are clock-neutral.** They consume physical append-only chunks but no Archive, Memory, Vector Generation, or CVA-global semantic version.
66. **Graph owns Memory-to-Memory relationship authority.** Graph relationship state is not Memory metadata and does not belong to Archive, Dream runtime state, or the physical Container.
67. **Graph endpoints use stable MemoryId only inside one owner.** Dense graph `NodeId` values are internal topology indexes; a bare `MemoryId` is not durable cross-owner identity, and the current Graph format does not persist cross-owner endpoints.
68. **Graph direction is semantic.** Processing, scheduling, candidate-selection, or comparison order cannot determine persisted relationship direction.
69. **Graph retraction is historical mutation, not deletion.** Removing a visible relationship appends an inactive mutation for the same oriented identity and advances `graph_version`.
70. **Graph topology is derived from versioned relationship state.** Adjacency and traversal structures may be rebuilt or replaced without changing semantic authority.
71. **Generic graph mechanics do not own Reliquary semantics.** `arcana-graph` may supply topology, storage mechanics, and traversal algorithms; Reliquary owns Memory endpoints, relationship vocabulary, CVA publication/versioning, and Dream semantics.
72. **A Graph batch is one semantic transaction.** A non-empty single-edge or multi-edge Graph publication consumes one CVA-global version and one Graph-local version; callers cannot observe an intermediate subset of an atomic relationship change set.
73. **Divergent Graph reconciliation follows owner dependencies.** Memories are replayed before Graph endpoints, relationship authority is republished through Graph with fresh clocks, and topology/duplicate indexes rebuild from merged authority instead of becoming synchronization metadata.

74. **Duplicate history is a chronological predecessor chain.** Active Dream `duplicate-of` edges point `newer -> previous equivalent`; ordering uses authoritative source chronology, never Memory creation bookkeeping, and any lookup/index structure remains derived from Graph state.
75. **Dream lifecycle is a Graph-derived Memory projection.** Accepted Graph state is published before lifecycle metadata; lifecycle revisions do not redefine relationship authority or mutate Memory bodies, failed bounded inference does not advance `extracted` sources, and archived Memories are never automatically reactivated by reconciliation.
76. **Canonical promotion is evidence-driven and deterministic.** Direct/correction authority promotes only configured current-state classes, duplicate corroboration requires distinct user authority anchors, canonical supersession transfers only through a unique active superseder, and age/model confidence/graph degree/reprocessing are never promotion evidence.
77. **Dream temporal semantics are body/source-time derived.** Content-time anchors and recurrence patterns derive from immutable Memory text plus authoritative source chronology; `Memory.created_at_ns` is never semantic time, source-time proximity alone is not a relationship signal, and derived temporal analysis owns no semantic clock.
78. **Reliquary identity is stored, not inferred.** New REL files carry authoritative file kind and Organization/Project/Connection scope kind in the physical header. A legacy 16-byte CVA header is explicitly Project Reliquary, and opening it must not silently rewrite the physical identity.
79. **Phylactery identity is distinct and stored.** New PHY files carry authoritative Phylactery file kind with no Reliquary scope; REL, PHY, and legacy CVA lifecycles must reject one another when their semantic identities do not match.
80. **Phylactery does not depend on Project Archive retention.** A current PHY Memory is valid without source turns, Episodes, Files, or a live originating REL. Direct PHY publication rejects REL-local provenance fields until an explicit cross-file lineage/export contract exists.
81. **Shared codecs do not collapse file semantics.** Phylactery currently persists Memories, Graph, Packed Vectors, Memory Vectors, and Compatibility Profiles only; REL-only owners cannot appear in PHY merely because the shared container can physically carry their bytes.
82. **Current Graph endpoints are same-file Memory identities.** REL and PHY may each persist Memory Graph state, but the current `MemoryId`-only endpoint format does not establish cross-file Graph relationships.
83. **Cross-file Memory references are owner-qualified.** A bare `MemoryId` is sufficient only inside one durable owner; persisted references to a Memory in another REL/PHY use `MemoryRef { owner_id, memory_id }`.
84. **Insomnia destination ownership cannot redefine semantic identity.** User/Project routing is decided only after semantic groups are fixed; ownership cannot change propositions, authority, provenance, metadata/lifecycle, grouping, or candidate key material.
85. **User-global export is conservative and source-independent.** Without an ownership classifier, Insomnia defaults to Project. A User-owned result may enter PHY only through an explicit routing target, and REL-local provenance is removed before PHY publication.
86. **Cross-file Insomnia publication prefers durable truth over atomic illusion.** With no cross-file transaction manager, routed User Memories sync to PHY before the REL completion receipt; deterministic mutation IDs make a crash in between idempotently recoverable.
87. **Source-independent does not mean time-free.** Insomnia resolves semantic source chronology while authoritative source evidence is available and persists it as optional `Memory.source_time_ns`; removing REL-local provenance for PHY publication must not replace that timestamp with Memory creation/update bookkeeping.
88. **Dream Graph and lifecycle effects are owner-local.** A Dream pass over one REL or PHY may read and mutate only Memories/Graph/lifecycle state owned by that file; another owner cannot become a candidate or mutation target implicitly.
89. **Unknown chronology stays unknown.** PHY Dream may use persisted `source_time_ns`; REL may additionally recover legacy source time from validated Archive provenance. Dream must never substitute `created_at_ns`/`updated_at_ns` for semantic chronology, and chronology-required duplicate ordering fails closed when source time is unavailable.
90. **PHY canonicalization does not invent missing provenance.** Source-independent direct-authority and supersession rules may promote PHY Memories, but REL corroboration logic that requires independent source authority anchors does not acquire a weaker PHY surrogate.
91. **Communities are derived Graph structure.** Persisted community snapshots cannot add, remove, redirect, or redefine Graph relationships and consume no semantic/global version ticket.
92. **Community detection is owner-local and graph-versioned.** One current snapshot partitions only Graph nodes from one REL or PHY and records the exact `graph_version` from which it was derived; cross-owner nodes cannot participate.
93. **Community identity is exact-membership identity, not continuity.** Community identity derives from durable owner UUID plus sorted member `MemoryId`s; identity continuity across changed membership, splits, or merges requires an explicit lineage contract.
94. **Community scan-and-merge is deterministic derived computation.** Algorithm v2 uses deterministic graph-local shard order, fixed shard size/fan-in/Leiden configuration, and reduction output that cannot depend on worker scheduling. Every structural edge must participate exactly once in the reduction representation.
95. **Human community names are presentation metadata.** A Warlock display rename cannot change persisted membership, Graph authority, or traversal semantics.
96. **Hosted model adapters do not own Reliquary stage routing.** A host may supply exact provider endpoints for Reliquary capabilities, but effective Insomnia/Dream fallback, metadata/ownership selection, and background-worker dispatch remain library-owned and must match standalone configured execution semantics.
97. **Live transcript search cannot broaden history.** The hot conversation search may inspect only the explicitly selected root-to-leaf ancestry (or the runtime's current open-session leaf), may derive Fragment windows transiently, and must not persist those windows, inspect sibling leaves, scan unrelated conversations, or advance semantic clocks.

## Safety boundaries

Changing version ownership, conversation ancestry, mutable-record revision semantics, or persistent format requires architecture review and focused behavioral tests.

## Related docs

- [Architecture](architecture.md)
- [Behavioral contracts](behavioral-contracts.md)
- [Versioning and rollback plan](version-history-plan.md)
- [Architectural decisions](decisions/INDEX.md)
- [ADR 0016](decisions/0016-native-product-surface-and-shared-interaction-runtime.md)

## Notes

Cross-database restore activation, actual concurrent writers, checkpointing, and retention/vacuum remain future work.