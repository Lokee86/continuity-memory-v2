# Troubleshooting

Parent index: [Reliquary operator manual](INDEX.md)

## Purpose

Map common operator/integration failures to the correct recovery action.

## Overview

Most Reliquary errors are deliberate fail-closed checks. Fix the stale/missing prerequisite rather than bypassing validation.

## Stale Memory retrieval index

Symptom:

```text
MemoryRetrievalError::StaleIndex
```

Action:

1. rebuild `MemoryRetrievalIndex`.
2. retry retrieval with the rebuilt index.

Common causes are Memory changes, Memory-Graph projection changes, Community refresh/generation changes, or selected-profile vector-binding changes.

Entity-only Graph associations do not stale the Memory retrieval index.

## Graph revision conflict

Symptom: Graph write reports an expected/current version mismatch.

Action:

1. read current `graph_version()`.
2. re-evaluate whether the intended mutation still applies.
3. publish against that version.

Do not substitute the newer version mechanically if another writer may have changed relevant semantic state.

## Missing Entity during association

Symptom: `set_entity_association` fails because the Entity endpoint does not exist.

Action:

1. verify the Entity exists in the same owner.
2. publish/create it if identity has actually been established.
3. retry the association with the current Graph version.

Do not create a placeholder “unresolved Entity.”

## Entity mention has no candidate

Current automatic bootstrap is not yet wired end to end.

If you are exercising the primitives manually, an empty candidate set does not itself prove that a new Entity should be created; the calibrated resolver is intended to make that judgment from the Memory/mention/candidate context.

## Community appears stale

Compare the snapshot's derived Graph watermark with `memory_graph_version()`, not full `graph_version()`.

A Perception-only Entity association can advance full Graph version while leaving the Community snapshot current.

## Knowledge graph is missing Entities

Check which relation API the caller uses.

Wrong for a general Knowledge view:

```text
graph_relations()
```

Correct source:

```text
runtime Knowledge read / semantic_graph_relations()
```

The old method intentionally returns only the Memory projection.

## Missing vectors or incompatible profile

Verify the selected Compatibility Profile and Memory-vector coverage. Use configured vector build/probe surfaces or deterministic development tooling as appropriate.

Do not mix vector bindings across incompatible profiles.

## Configuration fails verification

Run the CLI's `config show` and `config verify`. Check that each configured model capability references a compatible credential/provider route.

Codex/ChatGPT OAuth refresh can be performed explicitly; automatic refresh during model requests is not currently implemented.

## Old file will not open as current

Use explicit migration rather than editing headers or copying chunks:

```text
reliquary migrate <source> <output>
```

Keep the original until the migrated output verifies.

## Reconciliation returns a conflict

Treat it as a semantic conflict. Inspect the conflict family and decide at the application/user level which state should survive or whether a domain-specific merge is possible.

Do not use physical file mtime as semantic conflict resolution.

## Related docs

- [Current limitations](../current-limitations.md)
- [Rust API](../api.md)
- [Migration and reconciliation](migration-and-reconciliation.md)
- [Local configuration](../configuration.md)

## Notes

When a failure is unclear, determine the owner first: Archive, Memory, Entity, Graph, Community/retrieval derived state, vector/profile state, config, or runtime orchestration. That usually identifies the correct recovery surface.
