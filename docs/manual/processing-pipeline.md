# Processing Pipeline

Parent index: [Reliquary operator manual](INDEX.md)

## Purpose

Explain which processing stage consumes which state and which results are durable.

## Overview

The current common front half is:

```text
source turns/files
    ↓
Archive + Episodes
    ↓
Insomnia
    ↓
Memories + routing metadata + Entity mention spans
```

From that durable Memory state, there are two semantic lanes:

```text
Dream lane:
Memories
  → verified Memory-to-Memory Graph/lifecycle structure
  → Memory-only Community/retrieval inputs

Perception lane:
Memories + Entity mentions
  → candidate retrieval + Entity resolution
  → durable Entities + Memory→Entity Graph associations
```

Dream's current production lane is implemented. Perception's durable Entity/Graph primitives are implemented, but the automatic connection from extracted mentions through candidate retrieval/V4 into Entity creation/association is **not yet wired end to end**. The final automated scheduling of Entity resolution relative to Dream is therefore not stated as settled here.

## 1. Ingest source state

Use `Cva::ingest_turn` for turn ingestion when attachments need to be part of the same provenance operation. Lower-level Archive calls exist, but application/runtime integrations should avoid recreating turn-ingestion ownership manually.

## 2. Materialize/finalize Episodes

Episodes are deterministic semantic-processing windows. Live runtime scheduling and bulk import paths own their respective Episode lifecycle.

Do not make PHY Episodes. A routed PHY Memory points back to source identity in the REL instead.

## 3. Run Insomnia

Configured finite execution is available through `ConfiguredRuntime::run_insomnia_files`. The runtime host owns long-lived scheduling.

Insomnia currently produces:

- Memory candidates and durable Memory publication.
- authority/grounding provenance.
- temporal metadata.
- lexical routing metadata.
- exact Entity mention spans.
- completion/retry operational state.
- optional REL→PHY User Memory routing.

Entity mention extraction means “this span may name a referent.” It does not mean an Entity has been identified.

## 4. Run Dream

Dream operates on owner-local Memories. It performs bounded candidate discovery, relation classification/verification, Graph publication, duplicate/supersession handling, lifecycle projection, and Community-related derived work.

Dream currently sees the Memory-only Graph projection. Entity-only Graph mutations do not enter Dream traversal.

## 5. Entity resolution / Perception status

The following primitives are current:

- durable Entity objects in REL/PHY.
- exact one-to-many canonical-name/alias candidate lookup.
- per-mention resolution-state machinery retained from earlier work.
- typed `Memory -> Entity` Graph association.
- full semantic Graph traversal and Knowledge read visibility.

The calibrated V4 resolver is proven when given a Memory, one extracted mention, and candidate Entity records.

The following production loop is still **not yet wired/proven**:

```text
zero Entities
  → extracted mention
  → bounded candidate retrieval
  → V4
  → create/associate
  → index created Entity
  → later mention retrieves it
  → converges on same Entity
```

Until that loop is completed, do not describe Entity resolution as an automatic production capability.

## 6. Derived state refresh

Community and Memory-retrieval derived state follows the Memory Graph projection watermark, not every semantic Graph write. Perception-only Entity associations therefore do not force unchanged Dream/Leiden/retrieval state to rebuild.

## Related docs

- [Insomnia semantic validation](../insomnia-semantic-validation-2026-08-24.md)
- [Dream design record](../dream-implementation-plan.md)
- [Perception subsystem plan](../perception-subsystem-plan.md)
- [Working with Memories and Entities](memories-and-entities.md)

## Notes

Do not compensate for bad Entity mention extraction inside the resolver. Extraction errors and identity-resolution errors are separate failure classes.
