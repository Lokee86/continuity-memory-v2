# Architectural Invariants

These constraints are intentionally stronger than convenience abstractions.

1. **One file, several databases.** A `.cva` is a container for explicit purpose-built databases, not one generalized database.
2. **One mutable domain, one owner.** Every independently mutable semantic domain has one concrete database implementation that owns its rules.
3. **Stable IDs cross database boundaries.** Databases cross-index through IDs, never shared mutable in-memory ownership.
4. **No semantic knowledge in the container substrate.** Container code cannot distinguish kinds of Archive changes, memory semantics, graph meaning, or vector source meaning.
5. **No generic semantic dependency engine.** There is no CVA-wide `Dependency` abstraction that interprets cross-database meaning.
6. **No semantic-equivalence publication rules.** Atomic publication compares/installs prepared database states; the substrate never decides that two different domain states are equivalent for some consumer.
7. **Cross-database atomicity is explicit.** A caller may prepare several database updates and publish them together; this does not merge their ownership models.
8. **Derived state remains owned locally.** Each database decides which of its structures are derived/rebuildable and how they are invalidated or replaced.
9. **Runtime SQLite is disposable.** Operational work state cannot become semantic authority.
10. **Deletion of derived state cannot change semantic truth.** Rebuildable structures may affect performance, never meaning.
11. **Old published snapshots remain readable while retained.** New publication does not mutate old state in place.
12. **Prefer boring duplication over semantic generalization.** Shared code is justified only by identical physical mechanics, not merely similar-looking domain behavior.
13. **No code reuse by default.** Previous Continuity code must earn its way into this repository component by component.
14. **Architecture before migration.** Compatibility with development CVAs is subordinate to preserving this ownership model.
