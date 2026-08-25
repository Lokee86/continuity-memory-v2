# Reliquary and Phylactery memory scope plan

Parent index: [Documentation index](INDEX.md)

## Status

Provisional architecture. **Reliquary** and **Phylactery** are accepted names under [ADR 0018](decisions/0018-reliquary-and-phylactery-naming.md); the storage/pipeline design recorded here remains unresolved and unimplemented. It does not amend the current CVA/Insomnia contracts until a later implementation decision is accepted.

## Problem

A single omni-memory pool creates an avoidable context-management problem. Stable user-level facts and preferences need to persist across work, while project state needs to remain durable without competing with unrelated projects for retrieval relevance or being displaced as the user works elsewhere.

The intended model therefore has **two independent persistence and retrieval layers**, not one memory hierarchy with project tags:

1. **Phylactery** is the user-global memory/profile: it persists across projects and is eligible for cross-project retrieval.
2. **Reliquary** is the project-local memory/workspace: each Reliquary persists independently and participates in retrieval only when that project is active or explicitly consulted.

Normal active context can compose both retrieval results:

`current interaction + relevant user memory + relevant active-project memory`

Inactive project memories do not participate in ordinary retrieval. Mothballing a project therefore preserves its memory state without allowing that state to pollute unrelated work or compete continuously for context budget.

## Storage direction

Project memory remains the provenance-rich container system already being built. Its long-term product file identity is **Reliquary `.rel`**. It can retain Archive turns, Episodes, authoritative source/grounding relationships, Memories, vectors, and later graph/Dream state.

User memory is **Phylactery `.phy`**, a distinct semantic file kind rather than a Reliquary with fields removed. It is not merely a project container with missing fields. Its expected core is the durable user Memory web plus the indexes/graph/vector structures required to retrieve and maintain it.

Reliquary and Phylactery should share the same low-level container/storage machinery wherever the mechanics are genuinely common. Their semantic file kind, validity rules, and permitted owner composition remain distinct. The physical file must eventually encode `Reliquary | Phylactery` authoritatively; the `.rel` or `.phy` extension alone is not sufficient type evidence.

The current implementation still uses `.cva`. Existing `.cva` files are treated as **legacy Reliquary data** for migration purposes. The `.rel`/`.phy` direction is accepted under ADR 0020, but the exact header migration, owner sets, and physical schema transition remain unimplemented.

A user-profile Memory does **not** require source turns or project provenance in order to be valid. Source/provenance should be retained when it can legally and operationally cross the project boundary, but the profile format must remain valid when no source is available.

This is important for NDA, confidentiality, client-data, and other policy boundaries: a project may be allowed to contribute a generalized user Memory while being forbidden from exporting the conversation/source material that established it.

A likely project export-policy shape is conceptually:

- `memory_export = allow | deny`
- `source_export = allow | deny`

The exact names and policy representation are unresolved. `source_export = deny` must not imply that an otherwise permitted user Memory cannot exist. `memory_export = deny` prevents the project from contributing user-profile state at all.

When source export is allowed, the profile may retain copied evidence/provenance sufficient for later inspection. Whether this is a copied source package, a durable lineage record, or another purpose-built representation remains open. The user profile must not require a live cross-file pointer to the originating project in order to remain usable.

## Common versus destination-specific metadata

The current experimental metadata pass assigns semantic fields that can plausibly remain common to both destinations:

- `category`
- `type`
- `lifecycle`

Project and user Memories nevertheless need different surrounding metadata contracts.

Project Memories may retain full source identity, assistant-authority identity, grounding identity, Episode/source relationships, and other project-local provenance.

Phylactery Memories need a lighter contract. Candidate fields include user-global scope/ownership, creation/update state, optional origin lineage, and source-availability/export state. Full source-turn provenance is optional rather than mandatory.

The final Phylactery schema should be defined from user-memory requirements rather than by copying the Reliquary Memory record and making provenance nullable everywhere.

## Governed scopes outside the memory pipeline

The broader Warlock context model also includes **Organization** and **Relationship** scopes. These are not additional Insomnia Memory destinations.

They are explicitly governed, authoritative instruction scopes:

- **Organization** records how an organization intentionally operates: policy, procedure, approval rules, institutional defaults, security/compliance rules, terminology, and other explicitly managed instructions.
- **Relationship** records how parties intentionally operate with one another: negotiated exceptions, client/vendor-specific rules, standing communication or approval paths, contractual operating constraints, and other explicitly managed relationship instructions.

Insomnia and Dream must **not automatically create, infer, promote, supersede, or otherwise mutate Organization or Relationship state**. Repeated observed behaviour is not sufficient evidence that a policy, permission, contract term, or standing exception exists.

Those scopes may be edited directly by users or by agents acting under explicit user authorization. Automation may consume their contents as applicable context, but normal memory maintenance does not own them.

Conceptually, Organization and Relationship scopes behave like persistent, structured system/developer instruction addendums that Warlock composes into the active context. They should not be reduced to opaque prompt blobs merely because their runtime effect is instruction-like.

This leaves Insomnia/Dream responsible for learned contextual state, principally **Identity** and **Work**, while explicitly governed instruction state remains outside the autonomous memory pipeline.

## Insomnia scope classification

Insomnia's semantic extraction should remain concerned first with **what durable learned state exists**. The new memory persistence boundary is a later classification problem: each retained Memory must eventually be classified as either:

- `user`: durable Identity state that should follow the user across projects; or
- `project`: durable Work state belonging to the active project/workspace.

Examples of likely user state include stable identity/name conventions, general communication preferences, durable developer/working preferences, and other cross-project habits or constraints. Project implementation decisions, architecture, project facts, local constraints, project entities, and project history remain project state.

Atomicity matters. A mixed source statement such as a general user preference plus a project-specific consequence should become separate durable propositions before final scope ownership is assigned when they represent independently retainable state.

## Pass-boundary options

The current tuning harness is:

`semantic authority/disposition -> fixed groups -> metadata classification -> wording`

There are two viable ways to add persistence scope.

### Option A — add scope to the existing metadata pass

Extend the existing metadata classifier to return `scope = user | project` alongside `category`, `type`, and `lifecycle`.

Advantages:

- no additional model call;
- scope is naturally classification rather than extraction;
- synthesis can receive the selected destination and phrase the final Memory appropriately for that destination;
- publication can fork cleanly after synthesis.

Risk:

- Phylactery and Reliquary Memories are beginning to have meaningfully different metadata/provenance requirements;
- adding persistence-boundary responsibility to the metadata pass may couple two different classification concerns and make tuning/evaluation less clean.

### Option B — dedicated scope-classification pass

Use a separate narrow pass whose only substantive job is to decide `user | project` for an already-fixed durable Memory/group.

**This may be the better architecture and must remain an explicit option rather than being collapsed into the existing metadata pass by default.** The persistence boundary is consequential enough to justify independent tuning, stricter conservative policy for user-global state, and isolated regression tests.

A dedicated pass would also let ordinary semantic metadata evolve independently from the policy used to decide whether state belongs in the cross-project profile.

The exact placement is still open:

- **before synthesis:** scope is decided from the frozen durable proposition/group, then wording can be destination-aware;
- **after synthesis:** scope is decided from the final atomic Memory wording, which may make classification easier, but synthesis cannot use destination semantics unless a later normalization step exists.

The current working bias is **before synthesis** if destination-specific wording/schema materially differs, but this is not yet a decision. It should be tested rather than assumed.

A plausible four-pass experiment is therefore:

`semantic authority/disposition -> fixed groups -> semantic metadata -> scope classification -> destination-aware wording`

An alternate experiment should test scope immediately after fixed grouping and before ordinary metadata if the scope decision proves useful to selecting the appropriate metadata schema.

## Publication boundary

Scope classification and export permission are separate decisions.

A Memory may classify as `user` while project policy still forbids it from leaving the project. Insomnia should not equate "user-scoped" with "exportable".

Conceptually:

`durable Memory -> scope classification -> project export policy -> destination-specific synthesis/publication`

For project scope, publication remains in the project CVA with project provenance.

For user scope, permitted publication targets Phylactery. Source evidence is copied only when source export policy permits it. Otherwise the user Memory remains valid without source turns.

## Retrieval implication

The retrieval model is deliberately compositional rather than omni-memory search:

- query Phylactery for relevant cross-project user state;
- query the active Reliquary for relevant project state;
- combine the two result sets with current interaction context;
- do not include unrelated project CVAs in normal retrieval.

This keeps global user state small and durable while allowing individual projects to grow arbitrarily deep without degrading every other project's candidate set.

## Validation requirements

Before selecting the final pass arrangement, build scope-specific fixtures containing:

- obvious user-only Memories;
- obvious project-only Memories;
- mixed statements that should split into separate user/project propositions;
- project-specific preferences that must **not** leak into the user profile;
- general preferences expressed while discussing a project that should classify as user state;
- NDA/confidential examples where `user` classification is correct but export is forbidden;
- source-export-denied examples where user Memory publication remains valid without provenance;
- stale/mothballed project cases verifying that unrelated project state never enters ordinary retrieval.

Evaluate scope accuracy separately from extraction coverage, ordinary metadata accuracy, wording quality, and export-policy enforcement. A dedicated scope pass should be preferred if it materially improves isolation or makes the persistence boundary easier to audit without unacceptable inference cost.

## Open decisions

- physical/storage representation of explicitly governed Organization and Relationship instruction scopes, including how Warlock references and composes them without making Insomnia/Dream their owners;
- explicit mutation/authorization surface for user- or agent-directed Organization and Relationship edits;
- exact `.phy` Phylactery owner set, including required, optional, and forbidden owners;
- exact `.rel`/`.phy` header discriminator and legacy `.cva` migration mechanics;
- whether `CVA` survives only as a private generic-container implementation term or is renamed entirely;
- exact user-memory metadata schema;
- whether scope classification belongs inside the existing metadata pass or in a dedicated pass;
- if dedicated, whether it runs before ordinary metadata, after ordinary metadata, or after synthesis;
- whether user-scope synthesis must differ from project-scope synthesis;
- source-copy versus lineage representation when source export is permitted;
- project policy representation for memory/source export;
- how user Memories are corrected, superseded, deduplicated, or retired when they have no retained source evidence;
- how Dream/Graph operate over Phylactery versus project Reliquaries;
- how Ego/context assembly balances user-profile and active-project retrieval results.

## Related docs

- [Insomnia semantic validation — 2026-08-24](insomnia-semantic-validation-2026-08-24.md)
- [Roadmap](roadmap.md)
- [Architecture](architecture.md)
- [ADR 0012 — deterministic Episodes and Insomnia Memory authority](decisions/0012-deterministic-episodes-and-insomnia-memory-authority.md)
- [ADR 0017 — CVA workspace and Warlock host application](decisions/0017-cva-workspace-and-warlock-host-application.md)
- [ADR 0020 — Reliquary and Phylactery file kinds](decisions/0020-reliquary-and-phylactery-file-kinds.md)
