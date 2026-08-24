# ADR 0016: Native product surface and shared interaction runtime

## Status

Accepted — 2026-08-24.

## Context

Continuity now needs to become useful as a commercial product rather than only prove storage, retrieval, and agent-memory mechanics.

A developer-focused product can reasonably depend on an existing IDE, terminal agent, or protocol-capable host. A broader product cannot require a non-technical user to understand or install an agent protocol before they can use their own persistent project state.

ACP remains valuable because it can expose a structured live interaction stream from compatible external agents and clients. It is not universal, however, and protocol-specific transport details should not define Continuity's internal semantic model.

The product also needs a coherent way to browse and administer conversations, files, Memories, provenance, configuration, and health. Exposing raw store APIs directly to a UI would make the product boundary brittle, while inventing a generalized semantic object store would violate the purpose-built ownership model.

## Decision

Continuity will have a **native direct user interface as its primary product surface**, backed by a **shared long-lived interaction/runtime layer** and a **purpose-built management surface** over existing semantic owners.

ACP, imports, APIs, and future provider/agent integrations are adapters into that shared runtime. ACP is a first-class interoperability path, not a prerequisite for product use and not the canonical ingestion schema.

Conceptually:

```text
                     Native Continuity UI
                             |
                             v
                    shared live runtime
                     /       |       \
                    /        |        \
               ACP adapter  imports   APIs/adapters
                    \        |        /
                     \       |       /
                  normalized interaction/session events
                             |
                             v
                    purpose-built CVA owners
```

### Native interface is primary

The direct Continuity interface must be capable of ordinary product use without ACP, MCP, an IDE, or a terminal.

Its initial user model should expose useful concepts rather than storage internals:

- workspaces/projects;
- conversations and agents;
- files/artifacts;
- durable knowledge/Memories;
- provenance/history;
- model/agent configuration;
- health/status.

This does not require recreating every feature of a general chat application. The interface exists to make the user's persistent state usable and inspectable while also providing a direct conversational agent surface.

### Shared runtime is transport-neutral

The live runtime will define one normalized interaction/session contract above Archive ingestion.

Native UI events, ACP events, historical imports, and future protocol/provider adapters must converge into that contract before semantic storage. Protocol-specific metadata may be retained as provenance when useful, but it cannot become the semantic identity model for Archive, Memories, Echo, or future owners.

The runtime is responsible for live concerns such as session identity, stream assembly, acknowledgement/durability policy, adapter lifecycle, background work orchestration, capability caching, and context-injection coordination. It does not become a new semantic database.

### ACP is an interoperability adapter

ADR 0015 continues to govern the capture semantics of ACP-compatible sessions: observable events can be captured automatically, hidden provider state cannot be fabricated, and ordinary transcript persistence must not depend on model tool use.

This ADR narrows ACP's product role:

- ACP is preferred when interoperating with a compatible external agent/client path;
- ACP is not the primary Continuity user interface;
- Continuity's native interface does not need to speak ACP internally to use the runtime;
- ACP protocol evolution must remain isolated behind the adapter;
- non-ACP sources use equivalent adapters into the same normalized contract.

### Management is an application surface, not a database

A CVA management API/service will compose explicit operations from concrete owners for browsing, import/export, provenance, health, and permitted lifecycle changes.

It must not introduce:

- a generalized semantic object model;
- a CVA-wide mutable root;
- implicit cross-owner mutation;
- a generic relationship/store abstraction that bypasses owner authority.

The UI and external management clients consume this surface rather than reaching into physical Container mechanics.

### Semantic ownership does not change

Transport and UI choices do not alter semantic authority:

```text
source interactions / files       -> Archive
working durable semantic state    -> Memories
memory extraction authority       -> Insomnia
historical surfaced reasoning     -> Echo, when added
semantic relationships            -> Graph/Dream, when added
active context synthesis          -> Ego, when added
```

The shared runtime coordinates these owners but does not replace them.

## Consequences

- Non-technical users can use Continuity directly without adopting developer-agent infrastructure.
- Developer and external-agent users can continue using preferred clients through adapters such as ACP.
- Native and external interaction paths can produce equivalent internal source semantics.
- Protocol churn is isolated from CVA storage semantics.
- A coherent management API becomes a required layer between product UI and semantic owners.
- The runtime becomes the natural place for continuous Episode scheduling, background processing, capability caching, and context injection.
- The product can remain interoperable without making its own UI optional or secondary.
- The runtime/management boundary must be kept concrete to avoid becoming the generalized framework explicitly rejected elsewhere in the architecture.

## Open implementation decisions

- exact normalized interaction/session event vocabulary;
- process topology for native UI, runtime, and local IPC;
- durable acknowledgement point for streamed interactions;
- session identity across reconnect/resume and adapter migration;
- which tool/session events belong in source history versus separate future owners;
- native project/workspace semantics and their relationship to one or more CVAs;
- management mutation permissions before full historical retention/rollback exists;
- adapter privacy/consent and failure policy;
- whether external ACP agents are hosted directly inside the native UI, proxied through another client, or both.

## Rejected alternatives

### ACP-only product surface

Rejected. It makes product usability depend on external agent/client infrastructure and exposes protocol mechanics to users who should not need them.

### Native-UI-only closed product

Rejected. It would discard interoperability and recreate client lock-in around a customer-owned portable state artifact.

### Make ACP the canonical internal event model

Rejected. ACP is one transport and may evolve independently of Continuity's semantic requirements.

### Let the UI mutate stores directly

Rejected. Product operations need a stable management boundary that preserves concrete owner authority and validation.

### Introduce one generalized CVA object-management store

Rejected. Similar-looking product operations do not justify collapsing purpose-built semantic ownership.

## Verification

When implemented, focused tests should prove:

- equivalent native and adapter interaction fixtures normalize to the same source semantics;
- protocol-specific metadata cannot become Memory/source authority accidentally;
- acknowledged live source events survive restart according to one documented durability rule;
- management operations call concrete owner APIs and preserve owner validation;
- native product use requires no ACP dependency;
- ACP capture retains the guarantees from ADR 0015;
- adapters can be added or removed without changing persistent Archive semantics.

## References

- [Architecture](../architecture.md)
- [Roadmap](../roadmap.md)
- [ADR 0001](0001-purpose-built-database-ownership.md)
- [ADR 0011](0011-detachable-repo-local-cli.md)
- [ADR 0012](0012-deterministic-episodes-and-insomnia-memory-authority.md)
- [ADR 0015](0015-acp-inline-interaction-stream.md)
