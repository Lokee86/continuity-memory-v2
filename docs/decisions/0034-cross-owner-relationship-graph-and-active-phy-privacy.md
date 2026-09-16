# ADR 0034: Cross-owner Relationship graph and active-PHY privacy boundary

Parent index: [Architectural decisions](INDEX.md)

Implementation planning: [Perception subsystem plan](../perception-subsystem-plan.md) and [Roadmap](../roadmap.md)

## Status

Accepted — 2026-09-08. Design decision; not yet implemented.

Amends ADR 0029 by reactivating relationship-specific semantic state without reintroducing Connection REL classes. Amends ADR 0033 by adding Relationship synthesis/maintenance as Perception-owned semantic structure after Entity resolution.

## Context

ADR 0021 explored a dedicated Connection Reliquary whose durable state belonged to a relationship. ADR 0029 correctly rejected Connection as a behavioral REL class and made current RELs homogeneous, but deliberately mothballed the underlying relationship-state problem until a concrete requirement existed.

That requirement is now concrete.

The same people may participate in materially different relationships across personal, organizational, and project contexts. For example, Sarah may work in the Vancouver office with her husband, brother, sister, and children. The Vancouver organization legitimately needs shared business relationship state about those people. Sarah may also have private family knowledge about the same people that must not be exposed to another Vancouver employee such as Bob. Project-specific working relationships may differ again from both the organization-wide and personal relationships.

A single global Entity model cannot safely collapse those contexts. A separate REL per person or per relationship is also impractical: it creates a new full Memory Web for every durable connection, complicates portability, and turns semantic relationships into heavyweight storage owners.

The required abstraction is therefore a sparse Relationship layer over Entities that continue to exist in their ordinary REL/PHY Memory Webs.

## Decision

### Relationship is a first-class semantic container, not a REL/PHY owner

A **Relationship** is a durable, typed association among one or more existing Entities. It may retain relationship-local derived state, but it is not a Reliquary, Phylactery, Community, or independent Memory Web.

Conceptually:

```text
Relationship {
    id
    owner
    kind / classification
    participants[]
    participant roles / cardinality
    evidence references
    relationship-local derived state
}
```

The exact persisted schema is deferred to implementation work.

A Relationship may contain or index relational Observations, directional perspective state, standing commitments, synthesized relationship state, or similar derived knowledge whose subject is the relationship itself rather than one participant in isolation. It does not own source transcripts, raw Memories, an independent Dream graph, Leiden Communities, or another recursive Perception universe.

### Entities remain in their original Memory Webs

Creating or using a Relationship does not copy an Entity into a new semantic owner.

Relationship participants are **owner-qualified Entity references**. A Relationship may therefore refer to Entities that live in different REL/PHY Memory Webs while those Entities retain their original identity, metadata, evidence, and owner authority.

Conceptually:

```text
PHY Sarah / Entity Sarah ---------\
                                    > Relationship R
Vancouver REL / Entity Brother ---/
```

The Relationship layer is therefore cross-REL/PHY referential, but ordinary Memory Graph authority remains owner-local. This ADR does not authorize persisted cross-owner Dream Memory-to-Memory edges or cross-owner Community structure.

### Relationships are sparse and semantically materialized

Entity existence does not imply Relationship existence. Reliquary must not create one Relationship container for every Entity or every possible Entity pair.

A Relationship is materialized only when evidence establishes durable relational meaning worth preserving. Explicit durable statements such as spouse, sibling, manager, client, owner, or project membership may be sufficient immediately. Otherwise repeated relational evidence may cross a configured materiality/frequency threshold before a Relationship is created.

The model must support n-ary Relationships where the semantics are genuinely collective rather than forcing all state into pairwise edges. Relationship kinds may define participant roles and cardinality constraints, but the implementation must not require a closed subclass for every business or social relation.

### Relationship ownership provides semantic disentanglement

The same real-world participants may legitimately have different Relationships owned by different scopes.

For example:

```text
Sarah PHY
    Sarah <-> Brother     sibling / private family state

Vancouver Office REL
    Sarah <-> Brother     coworker / shared business state

Project Phoenix REL
    Sarah <-> Brother     project-specific working state
```

These are not competing copies of one universal Relationship. They are owner-local relational views backed by the evidence available to that owner.

The owner answers **which semantic world knows this relationship state**. The participant Entity references answer **who or what the relationship concerns**.

### REL-owned relationship state is portable shared context

A REL carries the Relationship state that legitimately belongs to that REL.

If Bob and Sarah both open the Vancouver Office REL, both may access Vancouver-owned business Relationships involving Sarah, her brother, husband, sister, children, Bob, or other organization Entities, subject to the normal access policy for that REL.

The Vancouver REL must not depend on Sarah's PHY to explain its shared business relationship state. The REL is portable with its own organizational evidence, Entities, and Relationships.

If Sarah later opens the same Vancouver REL on another machine together with Sarah's PHY, the shared Vancouver relationship layer composes with Sarah's private relationship layer at runtime. The REL itself remains unchanged and does not absorb Sarah's private graph.

### PHY-owned relationship state is private to the active PHY

PHY relationship state is cross-PHY compatible in representation but **not cross-PHY visible**.

At runtime, only the currently active PHY may contribute private Relationship state. A non-active user's PHY must never be traversed, queried, or used as a bridge merely because it refers to Entities also present in an open REL.

Therefore Bob opening the Vancouver REL may learn organization-owned facts such as who works with whom, but cannot obtain Sarah's private family observations, personal perspective state, or other Sarah-PHY relationship data.

The governing rule is:

> **Relationship visibility follows Relationship owner visibility. Entity visibility does not grant visibility into another owner's Relationships.**

And for PHY specifically:

> **Exactly one active PHY contributes private relationship state to normal runtime composition.**

### Effective relationship topology is runtime-composed

There is no single persisted universal relationship graph spanning every user's PHY and every REL.

The runtime computes an **effective Relationship graph** from the Relationship layers of the owners currently permitted in context:

```text
EffectiveRelationshipGraph =
    ActivePHY.Relationships
    union
    PermittedActiveRELs.Relationships
```

`PermittedActiveRELs` includes the active REL set and whatever dependency/ancestry closure the normal context resolver authorizes. Sibling or inactive RELs do not enter merely because they contain matching Entities.

This graph is a composed view. It does not transfer Relationship ownership, merge underlying Memory Webs, canonicalize private state across users, or publish new cross-owner Dream edges.

### References may cross ownership; visibility never does

A Relationship owned by one permitted owner may use owner-qualified Entity references that resolve into another mounted/permitted Memory Web. Such a reference may be temporarily unresolved when the referenced owner is not mounted.

Cross-owner references do not imply reciprocal access. Opening one endpoint's owner cannot be used to discover or traverse an otherwise inaccessible Relationship owner.

This is analogous to a foreign reference whose target identity is stable while authority remains with the object that owns the record.

### Relationship-local derived state does not become another Memory Web

Relationships need enough internal state to be useful semantic containers, but implementation must deliberately prevent them from recursively becoming miniature Reliquaries.

A Relationship may own/index:

- participant references and roles;
- classification/type metadata;
- evidence/support references;
- relational Observations;
- directional observer/observed or perspective state where useful;
- compact synthesized current relationship state; and
- lifecycle/material-change metadata.

A Relationship does **not** independently own:

- Archive/source payloads;
- raw Memories;
- an arbitrary Memory-to-Memory Dream Graph;
- independent Leiden Community hierarchies;
- nested Relationship Memory Webs; or
- an independent full Perception/Dream scheduler.

Any relationship-local Observation remains backed by semantic objects in normal owners through exact provenance/support references.

### Perception owns Relationship synthesis and maintenance

Relationship materialization depends on resolved Entity identity and therefore belongs after Perception's Entity resolution seam rather than in Insomnia or Dream.

Perception may use Insomnia Entity mentions, post-Dream Memory structure, resolved Entities, explicit relational language, participant frequency, and existing Relationship state to build a bounded candidate set. Probabilistic inference is used only where semantic judgment is required: whether a durable Relationship exists, its participant roles/classification, and whether new evidence materially changes relationship-local state.

Candidate discovery must remain sparse and local to affected Entities/evidence. No all-Entity-pairs inference pass is permitted.

The exact ordering relative to Observation contribution/extrapolation may be refined during implementation, but Relationship synthesis is a distinct semantic lane from Dream's Memory-to-Memory relation classifier.

## Example: Vancouver office portability and privacy

Shared Vancouver Office REL state may contain:

```text
Sarah works-with Brother
Sarah works-with Husband
Bob works-with Brother
Brother participates-in Project Phoenix
```

Sarah's PHY may separately contain:

```text
Sarah sibling-of Brother
Sarah spouse-of Husband
Sarah parent-of Child
private relationship Observations and perspectives
```

When Bob opens the Vancouver REL:

```text
Bob PHY + Vancouver REL
    -> Bob's private Relationship layer
    -> Vancouver shared Relationship layer
    -> no Sarah-PHY Relationship state
```

When Sarah opens the same Vancouver REL:

```text
Sarah PHY + Vancouver REL
    -> Sarah's private Relationship layer
    -> Vancouver shared Relationship layer
    -> no Bob-PHY Relationship state
```

The same portable REL therefore presents the shared organization relationships to both users while each active PHY contributes a different private relational overlay.

## Consequences

The former Connection REL requirement is replaced by a lighter semantic container that preserves relational locality without creating another `.rel` file or full Memory Web for every person/connection.

Organization/project relationship knowledge remains portable with the REL that owns it. Personal relationship knowledge remains portable with the PHY that owns it.

The same participant set can have distinct personal, organization, and project Relationships without semantic collision.

Cross-owner relationship queries become possible without weakening ordinary owner-local Dream/Graph authority.

Ego may later use the effective Relationship graph as a routing/context signal while continuing to compose normal Memory/Observation evidence from authorized owners.

The model introduces a new privacy-critical traversal boundary: implementation must check Relationship-owner visibility before dereferencing or surfacing relationship-local state.

## Rejected alternatives

### Restore Connection as a special REL class

Rejected. ADR 0029's homogeneous REL decision remains correct. A dedicated Connection REL would create heavyweight files and independent Memory Webs for semantic objects that normally need only bounded relationship-local derived state.

### Create one Relationship container per Entity

Rejected. Entity identity and relational meaning are different concerns. Relationships are materialized only when durable relational evidence warrants them.

### One global persisted relationship graph across all PHYs

Rejected. This would make private user relationship state discoverable through shared Entities and would couple unrelated users' personal graphs.

### Copy personal Entity/relationship state into a shared Organization REL

Rejected. The organization must own its own legitimate shared business knowledge. User-private state remains in the user's PHY and composes only for that active user.

### Keep all relationship state as ordinary Entity metadata

Rejected. Relational state often belongs to the interaction among participants rather than any participant individually, and the same participants can have materially different relationships in personal, organization, and project contexts.

### Treat every relationship as an independent nested Memory Web

Rejected for practicality. Nested full semantic graphs multiply Dream/Perception/community maintenance and create recursive ownership/synchronization problems without evidence that relationship-local state requires those capabilities.

## Open implementation questions

- exact Relationship ID, persistence, versioning, and reconciliation format;
- exact owner-qualified `EntityRef` representation and dangling-reference behavior when another owner is not mounted;
- initial Relationship type/classification and participant-role vocabulary;
- cardinality constraints and n-ary Relationship representation;
- materialization thresholds for explicit versus repeated relational evidence;
- representation of relationship-local Observations, perspective state, and synthesized summaries;
- Relationship lifecycle, stale/retired state, and historical retention;
- deterministic candidate indexes and bounded Perception inference contracts;
- how Relationship evidence/support participates in Observation receptors and reconsideration;
- Ego retrieval/context rules over the effective Relationship graph; and
- authorization policy for shared REL relationship state beyond the owner-visibility baseline fixed here.

## Verification

Implementation must protect at minimum:

- no Relationship creation solely because an Entity exists;
- no all-Entity-pairs inference path;
- durable participant identity through owner-qualified Entity references;
- separate personal, organization, and project Relationships for the same participants;
- REL portability without a dependency on another user's PHY;
- shared REL relationship visibility for authorized users;
- strict exclusion of non-active PHY Relationship state;
- no traversal from a visible Entity into an inaccessible Relationship owner;
- runtime composition from active PHY plus permitted active RELs only;
- no cross-owner Dream Memory edges or Community authority introduced by this layer; and
- relationship-local derived state retaining exact evidence/provenance links to ordinary semantic owners.

## Related docs

- [ADR 0021 — Typed Reliquary scopes and Connection state](0021-typed-reliquary-scopes-and-connections.md)
- [ADR 0029 — Homogeneous Reliquaries and dependency-based context inheritance](0029-active-rel-hierarchy-and-deferred-connection-scope.md)
- [ADR 0033 — Perception entities, observations, and ambiguity handling](0033-perception-entities-observations-and-ambiguity.md)
- [Reliquary and Phylactery scope design record](../reliquary-phylactery-memory-scope-plan.md)
- [Perception subsystem plan](../perception-subsystem-plan.md)
- [Roadmap](../roadmap.md)
