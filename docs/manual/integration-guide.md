# Integration Guide

Parent index: [Reliquary operator manual](INDEX.md)

## Purpose

Embed Reliquary behind an application such as Warlock without duplicating its semantic workflow or violating owner boundaries.

## Overview

Prefer the highest-level library surface that owns the workflow you need.

A product integration normally composes:

```text
application / Warlock
    ↓
ConfiguredRuntime + ReliquaryRuntimeHost
    ↓
InteractionRuntime
    ↓
REL (+ optional attached PHY)
```

## Use ConfiguredRuntime for configured model work

`ConfiguredRuntime::open(config_path)` resolves configured capability routes.

Use its high-level finite workflows, including `run_insomnia_files`, instead of constructing provider/model/ownership routing in UI or CLI code.

## Use ReliquaryRuntimeHost for a long-lived project session

The runtime host owns long-lived worker coordination and capability routes around one REL.

Relevant lifecycle operations include:

- start active/inactive according to whether Insomnia should claim work.
- attach/detach one PHY explicitly.
- retrieve REL/PHY Memories through owner-local lanes.
- expose Archive/session reads.
- dispatch Dream/Insomnia/vector work through configured boundaries.

Do not create a second scheduler in the UI for work already owned by the host.

## Keep owner boundaries explicit

REL and PHY are separate durable owners.

When routing a User Memory from REL processing to PHY:

- the PHY stores its own Memory.
- the PHY does not copy the REL Episode/turn payload.
- provenance remains an identifier-only external source reference.

Do not write user-global semantic state into whichever REL happens to be open.

## Respect optimistic versions

Mutable semantic owners expose current versions/revisions for a reason.

Typical rules:

- Memory/Entity revision writes require the exact expected object revision.
- Graph writes require the full current `graph_version`.
- stale derived retrieval indexes are rebuilt, not force-used.

Treat conflicts as concurrency/state information. Do not retry by silently substituting “latest” without reconsidering the intended mutation.

## Use the correct Knowledge surface

A user-facing Knowledge graph must include Entities and Perception relations.

Use the runtime Knowledge read surface/full semantic Graph, not the Memory-only `graph_relations()` projection.

Communities returned beside Knowledge state remain a Memory-derived overlay.

## Flush at application durability boundaries

Use `sync()` when the application needs durable completion and the selected high-level workflow has not already documented ownership of that sync.

## Avoid these integration mistakes

- storing a second authoritative transcript beside the REL when Reliquary is meant to own it.
- rebuilding Insomnia/Dream orchestration in application code.
- treating derived indexes as semantic authority.
- using exact alias equality as Entity identity proof.
- using Memory-only Graph APIs for a general Knowledge graph.
- copying REL source payload into PHY.
- assuming a Graph version and Memory Graph projection version are interchangeable.

## Related docs

- [Rust API](../api.md)
- [Local configuration](../configuration.md)
- [Architecture](../architecture.md)
- [Core concepts](core-concepts.md)
- [Troubleshooting](troubleshooting.md)

## Notes

Warlock owns application orchestration and presentation. Reliquary owns its internal semantic storage/runtime contracts; integration code should cross that boundary through public/high-level APIs.
