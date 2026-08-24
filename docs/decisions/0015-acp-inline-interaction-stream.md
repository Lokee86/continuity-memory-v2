# ADR 0015: ACP inline interaction stream

## Status

Accepted for ACP adapter capture semantics — 2026-08-17. **Amended by [ADR 0016](0016-native-product-surface-and-shared-interaction-runtime.md)** on 2026-08-24: ACP remains a first-class external interoperability/capture path, but it is no longer the primary Continuity product surface. Protocol-specific proxy mechanics remain provisional while the ACP proxy-chain and MCP-over-ACP RFDs are Draft.

## Context

Continuity needs a reliable live-ingestion path for agent conversations. Requiring an agent to explicitly call an MCP tool such as `record_turn` is structurally weak: capture becomes optional model behavior, failures can silently create holes in source history, and every agent integration must be taught to persist its own transcript.

Post-hoc transcript scraping is also a poor primary path. It loses live session semantics, tends to require provider-specific ingestion, and may not expose the same structured events that were available while the interaction was occurring.

Agent Client Protocol (ACP) provides a more useful integration boundary because it carries the live interaction between a client and an agent. The current ACP **Agent Extensions via ACP Proxies** Draft goes further and explicitly proposes components that sit between client and agent, intercept and forward ACP messages, inject context, transform responses, and provide conversation-aware extensions. That shape closely matches Continuity's live-capture and context-injection requirements.

The architectural opportunity is broader than IDE integration. Continuity does not require an IDE to benefit from ACP. Any ACP-speaking predecessor — desktop application, terminal/TUI client, plugin host, chat surface, or another agent client — can conceptually place Continuity inline before the downstream agent.

## Decision

Continuity will treat **inline ACP-compatible interaction capture** as the preferred design direction for live agent sessions where an ACP path is available.

Conceptually:

```text
user / client / plugin host
          |
          | ACP-compatible interaction stream
          v
+-----------------------------+
| Continuity runtime          |
|                             |
| observe + archive           |
| retrieve + inject context   |
| preserve session identity   |
+-----------------------------+
          |
          | ACP-compatible interaction stream
          v
     downstream agent
          |
          v
   model / provider
```

Continuity is therefore **inline with the observable agent interaction stream**, not inside the provider's private model execution.

### Capture boundary

The capture rule is simple:

> If an interaction event traverses the Continuity ACP path and is exposed by the protocol/adapter, Continuity may persist it automatically according to the owning source-history rules.

This can include, when exposed:

- user prompts/messages;
- streamed and completed agent responses;
- session identifiers and lifecycle metadata;
- tool-call and tool-result events;
- attachments or structured content carried through the interaction;
- surfaced reasoning/thought events;
- usage or other session metadata when useful and explicitly available.

Continuity cannot capture data that never traverses the observable integration boundary. Provider-private chain of thought, hidden system context, latent/model state, internal orchestration, or other server-side information that the agent/provider does not emit remains unavailable.

This boundary is intentional. Continuity preserves **observable interaction history**, not hidden provider internals.

### Ownership of captured material

Inline capture does not change CVA semantic ownership:

```text
ACP-visible conversation/tool history -> Archive
surfaced historical reasoning         -> Echo, when applicable
retained durable semantic state       -> Memories through Insomnia
relationship synthesis                -> Dream
active context synthesis              -> Ego
```

An ACP event is not automatically a Memory merely because it was observed. User/agent messages remain source history. Surfaced reasoning remains non-authoritative historical computation under Echo's rules. Insomnia remains the authority that converts source material into working Memory.

### Automatic capture replaces explicit persistence calls

Normal live capture should not depend on the downstream model remembering to call a Continuity MCP tool.

The desired flow is:

```text
new user turn enters ACP path
    -> Continuity observes/persists source event
    -> Continuity may retrieve relevant prior state
    -> downstream agent receives the turn + permitted context

agent/tool events return
    -> Continuity observes/persists exposed events
    -> client receives them normally
```

The narrow `create_memory` tool remains useful for explicit user intent and immediate Episode finalization, but it is not the ordinary transcript-ingestion mechanism.

### ACP and MCP have different jobs

ACP and MCP are complementary rather than competing interfaces:

- **ACP-compatible path:** live conversational/session data plane; automatic observation, session continuity, and a place to inject relevant historical context before the downstream agent processes a turn.
- **MCP/API:** explicit memory/tool plane; search, retrieval, `create_memory`, archive queries, diagnostics, and other requested Continuity operations.

If MCP-over-ACP stabilizes and proves suitable, one ACP connection may eventually carry both the live interaction path and Continuity-provided MCP capabilities. Continuity must not depend on that Draft mechanism until its protocol contract is sufficiently stable.

### ACP is not an IDE dependency

Continuity must not architect this integration around editor-specific assumptions. The relevant abstraction is **client -> Continuity -> agent**, not **IDE -> Continuity -> coding agent**.

An IDE can be one ACP client, but so can a dedicated chat client, desktop Continuity surface, terminal interface, plugin host, or other agent front end.

This keeps the commercial product boundary coherent:

```text
Continuity runtime = product/core service
ACP/MCP/API         = agent integration surfaces
Desktop             = management/control surface
CLI                 = automation/headless/operator surface
.cva                = portable customer-held state artifact
```

### Compatibility and fallback

ACP cannot provide universal live capture by itself. A provider or agent path must speak ACP or be bridged/adapted to an equivalent observable event stream.

Therefore:

- ACP-native or ACP-adaptable agent paths should prefer inline capture;
- provider web applications and closed subscription interfaces that expose no compatible live protocol still require imports, provider-specific adapters, hooks/plugins, or other supported ingestion paths;
- Continuity must not claim access to hidden provider state merely because it is inline with an ACP-compatible client/agent exchange.

The integration layer should preserve one normalized internal source-event contract so ACP, future protocols, and provider-specific adapters feed the same Archive ownership boundary instead of creating protocol-specific Archive semantics.

### Protocol stability boundary

As of 2026-08-17, core ACP session communication is established, but the official **Agent Extensions via ACP Proxies** and **MCP-over-ACP** designs remain Draft RFDs. Continuity should therefore separate the product requirement from the transport detail:

> Product requirement: observe the complete exposed live interaction stream inline and optionally supply relevant prior context before downstream processing.

> Current preferred transport: ACP-compatible proxy/bridge semantics where available.

If ACP proxy mechanics change, the product requirement and Archive/Memory ownership model remain valid.

## Consequences

- Live transcript capture no longer depends on optional model tool use.
- The CVA can receive source history at the moment the user/agent interaction occurs rather than through later scraping.
- Continuity gains a natural interception point for retrieval/context injection before downstream agent processing.
- ACP can serve as a first-class external agential interface without requiring an IDE-specific product architecture.
- MCP remains valuable for explicit memory operations without carrying responsibility for transcript persistence.
- Provider-private reasoning and state remain outside Continuity unless explicitly emitted through the observable stream.
- ACP-specific transport churn is isolated behind the integration adapter rather than entering Archive semantics.
- Inline operation increases the trust/security responsibility of the Continuity runtime because it can observe potentially sensitive interaction traffic.

## Open implementation decisions

- exact ACP proxy/conductor integration if/when the Draft proxy-chain RFD stabilizes;
- exact hosting/proxy shape for external ACP agents inside or alongside the native Continuity product surface;
- mapping ACP session/message identifiers onto Archive conversation/node identity across reconnect/resume;
- chunk assembly and durable publication boundaries for streamed messages;
- treatment of edits/replacements/replays when protocol-level message identity is available;
- failure policy when the inline Continuity component is unavailable: fail-open, fail-closed, or user-selectable;
- context-injection policy and ownership between the live runtime, retrieval controller, and future Ego;
- privacy/consent controls for automatic live capture;
- adapter contract for agents/providers that do not natively expose ACP.

## Rejected alternatives

### Require agents to call an MCP persistence tool for every turn

Rejected as the primary live-ingestion path. It makes source-history completeness depend on model behavior and integration-specific prompting.

### Scrape or import every live transcript after the fact

Rejected as the primary path. Imports remain necessary for historical/closed-provider sources, but post-hoc ingestion should not replace structured live capture where an inline interaction stream exists.

### Treat ACP as an IDE-only integration

Rejected. IDEs are one ACP client category, not the architectural boundary Continuity needs.

### Treat all inline events as Memories

Rejected. Inline visibility changes ingestion reliability, not semantic authority. Archive, Echo, Insomnia, Memories, Dream, and Ego retain their distinct ownership rules.

### Claim capture of provider-private inference state

Rejected. Continuity can observe only what the client/agent/provider path actually emits through the supported interface.

## Verification

When implemented, focused tests should prove:

- every supported user/agent message traversing the inline test transport is captured exactly once;
- streamed chunks reconstruct the intended durable source event without duplication after retry/reopen;
- tool events and surfaced thought/reasoning events retain correct ownership and ordering;
- hidden/non-emitted provider state is never fabricated as captured data;
- capture does not require an MCP persistence call;
- session resume/reconnect preserves or deliberately remaps Archive identity according to one documented rule;
- retrieval/context injection can occur before downstream prompt processing without rewriting the authoritative user source event;
- ACP transport metadata cannot accidentally become Memory authority;
- failure/restart behavior does not silently lose already acknowledged durable interaction events.

## References

- ACP Architecture: https://agentclientprotocol.com/get-started/architecture
- ACP RFD — Agent Extensions via ACP Proxies: https://agentclientprotocol.com/rfds/proxy-chains
- ACP RFD — MCP-over-ACP: https://agentclientprotocol.com/rfds/mcp-over-acp
- [Architecture](../architecture.md)
- [Roadmap](../roadmap.md)
- [ADR 0012](0012-deterministic-episodes-and-insomnia-memory-authority.md)
- [ADR 0014](0014-echo-historical-reasoning-traces.md)
- [ADR 0016](0016-native-product-surface-and-shared-interaction-runtime.md)
