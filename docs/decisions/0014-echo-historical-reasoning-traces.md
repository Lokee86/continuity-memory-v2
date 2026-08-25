# ADR 0014: Echo historical reasoning traces

## Status

Accepted — 2026-08-16. Refined from provider-export inspection on 2026-08-16/17.

## Context

Provider conversation/session exports can contain substantially more than the visible user/assistant transcript. In particular, some providers preserve assistant reasoning or reasoning-adjacent computational state that can be useful when resuming difficult work: hypotheses that were tested, why a direction was rejected, what uncertainty remained behind a terse progress update, which files or tools were inspected, and how a later conclusion was reached.

That material is qualitatively different from both visible conversation history and Reliquary Memories. A reasoning trace may contain abandoned hypotheses, contradictions, misunderstandings, speculative branches, incomplete inference, tool-planning, provider summaries, encrypted/opaque reasoning payloads, or only structural evidence that reasoning occurred. Treating it as ordinary Memory would incorrectly promote historical computation into durable truth. Injecting it routinely would waste context and allow stale reasoning to compete with canonical Memory state.

Provider inspection also shows that there is no universal native "CoT record" shape. ChatGPT, DeepSeek, Codex/Hermes, and Claude expose materially different topologies, granularities, and visibility levels. Echo therefore cannot be designed around any one provider's message schema.

The design needs a distinct owner for preserved historical reasoning that remains available without becoming normal Memory, plus a normalization boundary that preserves provider-native provenance without pretending that all providers expose equivalent reasoning.

## Decision

The source-scoped historical reasoning layer is named **Echo**.

Echo owns preserved historical computational state associated with imported or captured conversation history. An Echo trace is not a Memory, does not establish truth, and does not carry user authority merely because it existed in a prior model run.

The conceptual separation is:

```text
Archive   = what was said / source history
Memories  = what Reliquary learned and retained
Echo      = how prior model work reasoned, when that trace is available
Ego       = what is synthesized into active context
```

Echo remains a separate purpose-built CVA owner rather than being folded into Archive fragments, Memory revisions, Dream lifecycle state, or Insomnia operational logs.

### Retrieval boundary: exactly one visible source

Echo retrieval is deliberately **single-source scoped**.

Ordinary Archive/Memory retrieval must first identify one specific visible source event, normally a visible assistant output. Only then may Echo search the historical computational trace owned by that source.

Conceptually:

```text
ordinary retrieval
    ↓
select visible source B
    ↓
Echo(B, query)
    ↓
rank only Echo material owned by B
```

The source restriction is applied **before** lexical/vector ranking. Echo does not run a global search and filter the results afterward.

Initial Echo retrieval does **not** support:

- conversation-wide Echo search;
- source sets such as `{A, B, C}`;
- automatic neighboring-turn expansion;
- Episode-wide Echo search;
- global cross-conversation Echo search.

If ordinary retrieval returns several potentially relevant visible sources, an upstream caller must choose one before querying Echo. If Echo for that source is insufficient, another source may be selected and queried in a separate operation. This keeps context expansion explicit and bounded instead of allowing a locally relevant source to become a route for importing an entire conversation or candidate set into context.

A useful access invariant is:

> No Echo material may be retrieved unless its one parent visible source has already been selected or explicitly addressed.

User turns and other visible events may be retained as provenance anchors, but imported reasoning should normally be bound to the visible assistant output/result it helped produce. Where a provider represents one logical response as a tool loop followed by a final visible output, normalization may bind the whole response-cycle trace to that final visible source without turning the intermediate actions into separate retrieval sources.

### Echo trace versus provider-native record

The common abstraction is **one source-owned ordered Echo trace**, not "one reasoning field attached to one message."

A logical trace may contain ordered events such as:

```text
EchoTrace
├── parent_visible_source_id
└── events[]
    ├── reasoning
    ├── tool_action_ref
    ├── tool_result_ref
    ├── reasoning
    └── provider/structural metadata
```

Tool calls/results remain Archive/source-history evidence where Archive already owns them. Echo may retain their ordering and references so reasoning can be reconstructed in execution order; it should not create a second semantic authority copy of ordinary visible/tool history merely to represent the trace.

Echo distinguishes at least two granularities:

- **provenance unit** — the provider-native reasoning/thinking object or event that was actually supplied;
- **retrieval unit** — a searchable unit derived from one provenance unit when necessary.

Small provider-native reasoning items may be retrieval units unchanged. Large raw reasoning blocks may be segmented for vector/lexical ranking while retaining their parent provenance-unit identity and exact ordering. Derived segmentation must never replace or destroy the provider-native source representation.

### Representation fidelity

Echo must preserve what the provider actually supplied rather than manufacturing a false cross-provider equivalence.

A reasoning event may therefore have a representation such as:

- `raw` — provider exposed substantive reasoning text;
- `summary` — provider exposed only a reasoning summary/recap;
- `opaque` — provider exposed an encrypted/signed/otherwise unreadable reasoning object;
- `structural` — provider exposed only a marker, ordering event, or metadata proving that reasoning occurred.

A conceptual normalized event may therefore contain:

```text
EchoReasoningEvent
├── source_trace_id
├── provider_event_id
├── order
├── representation: raw | summary | opaque | structural
├── searchable_text?        # raw text when available, otherwise provider summary
├── provider_payload?       # preserved opaque/native object when useful
├── signature?              # provider integrity/provenance material when supplied
└── provider/model metadata
```

Opaque provider payloads are retained for provenance/round-trip value where appropriate but are not treated as semantically searchable plaintext. Echo search uses only text actually available to Reliquary: raw reasoning when exposed, otherwise a provider-supplied summary/recap. It must not imply that a summary is equivalent to hidden/raw reasoning.

### Provider observations that drive the abstraction

The following August 2026 inspections are design evidence, not permanent format contracts.

#### ChatGPT export

The prepared ChatGPT corpus contains separate reasoning graph nodes rather than large raw reasoning blocks embedded in visible assistant messages.

Observed source data included approximately:

- 75,697 source `thoughts` nodes;
- 80,673 unique thought items inside those nodes;
- additional `reasoning_recap` nodes;
- branch-expanded normalization that can duplicate shared graph ancestry while preserving source identity.

Measured normalized thought-item text was generally small:

- median: 71 characters;
- 75th percentile: 273;
- 90th: 341;
- 95th: 377;
- 99th: 457;
- maximum observed: 1,171;
- mean: about 150.5.

A single `thoughts` node can contain multiple thought items. The export/normalization preserves graph provenance such as reasoning node identity plus preceding/following visible anchors. For Echo, these small items can usually remain intact as retrieval units. The following visible assistant output is the natural parent source when the graph establishes that relationship.

This material does **not** resemble a verbatim token-by-token raw CoT stream. Some records contain substantive reasoning content; others are summary-only or recap-style events. Echo must preserve that distinction.

#### DeepSeek export

The inspected DeepSeek export contained 29 conversations and 282 message nodes. Reasoning is stored explicitly as `THINK` fragments inside assistant message nodes.

Observed data included:

- 93 assistant messages with reasoning;
- 208 `THINK` fragments;
- roughly 227,000 characters of reasoning text.

Two patterns were evident:

1. ordinary reasoner responses often contain one large `THINK` block followed by one visible `RESPONSE`;
2. tool-using responses contain many smaller `THINK` fragments interleaved with tool operations.

For 67 ordinary non-tool reasoner responses, the median reasoning block was about 2,630 characters, with some blocks above 6,000 characters and a maximum observed around 7,311. Tool-using responses could instead contain many small fragments; one inspected response had 17 separate `THINK` fragments totaling about 5,844 characters.

DeepSeek therefore gives explicit same-message ownership, but provider-native fragments are not always the ideal search granularity: small tool-interleaved thoughts may be indexed directly, while multi-thousand-character raw blocks may need derived segmentation.

#### Hermes / Codex session store

The live Hermes `state.db` uses a message schema that can persist multiple reasoning representations on an assistant row:

- `reasoning`;
- `reasoning_content`;
- `reasoning_details`;
- `codex_reasoning_items`.

The inspected database contained 508 sessions (319 Luna, 189 Sol) and 9,613 assistant rows. Of those, 8,201 carried reasoning data.

The dominant topology was a tool loop rather than reasoning attached only to the final visible response:

- 7,985 reasoning-bearing rows were contentless assistant tool-call rows with `finish_reason = tool_calls`;
- only 216 reasoning-bearing rows contained visible final content with `finish_reason = stop`;
- 7,882 rows had a plaintext reasoning summary;
- all 8,201 reasoning-bearing rows had Codex reasoning items containing encrypted provider reasoning objects.

Plaintext reasoning summaries were small (median about 43 characters, 95th percentile about 191, maximum observed 2,658). A typical `codex_reasoning_items` object contained an opaque `encrypted_content` payload plus a short provider summary.

A real logical response therefore looks more like:

```text
user visible turn
    ↓
assistant action: short reasoning summary + encrypted reasoning + tool call
    ↓
tool result
    ↓
assistant action: short reasoning summary + encrypted reasoning + tool call
    ↓
tool result
    ↓
...
    ↓
final visible assistant response
```

For Echo, those intermediate assistant action rows are part of the response-cycle trace that produced the final visible source. They must not force Echo to expose a source set. The final visible output remains the one retrieval anchor; the internal trace may contain many provider-native reasoning/action events.

Codex also proves that Echo cannot assume the underlying reasoning is available as plaintext. The searchable representation may be only the provider summary while the encrypted reasoning object remains opaque provenance material.

#### Claude export

The inspected Claude export contained 11 conversations, 412 messages, and 443 first-class `thinking` blocks across 185 of 207 assistant messages.

Claude represents an assistant message as an ordered content trace containing combinations of:

- `thinking`;
- `tool_use`;
- `tool_result`;
- visible `text`.

A single assistant message can contain a very large reasoning/tool execution sequence. One observed message contained 47 `thinking` blocks, 50 `tool_use` blocks, 50 `tool_result` blocks, and roughly 61,480 characters of plaintext thinking under one assistant message UUID. Another had 50 thinking blocks and 50 tool cycles.

Claude also exposes multiple reasoning visibility states:

- 328 thinking blocks contained plaintext `thinking`;
- 113 were marked hidden with no raw reasoning text;
- 89 of those hidden blocks supplied a short summary;
- some hidden blocks supplied neither raw reasoning nor summary but retained a signature;
- 369 of 443 thinking blocks carried a signature.

For non-empty raw thinking blocks, observed size was much larger than ChatGPT's exported thought events:

- median: about 809 characters;
- 75th percentile: about 1,568;
- 90th: about 3,012;
- 95th: about 4,713;
- 99th: about 7,211;
- maximum observed: 13,199.

Claude therefore combines DeepSeek-like in-message ownership with much larger multi-event tool loops and explicit raw/hidden/summary/signature states. Its export is strong evidence for preserving an ordered source-owned trace rather than flattening all reasoning into one text field.

### Provenance and ordering

Echo records preserve enough provenance to distinguish historical reasoning from conclusions and visible output. Provider/import adapters should retain, when available:

- conversation/session identity;
- parent visible source identity;
- provider-native reasoning/event identity;
- source order within the logical trace;
- preceding/following visible anchors when the provider uses a conversation graph;
- tool-action/result references needed to reconstruct execution order;
- provider/model metadata;
- raw reasoning content and/or provider-supplied summary;
- representation kind (`raw`, `summary`, `opaque`, `structural`);
- provider signatures/encrypted payloads/native metadata where they carry useful provenance or replay value.

Visible progress updates and final assistant messages remain Archive history, not Echo reasoning records. If a visible progress update creates a real visible boundary, subsequent reasoning is bound to the next visible source rather than silently widening the prior source's Echo scope.

### Retrieval and active context

Echo is cold by default. It consumes no Ego context merely because it exists.

Ego may consult Echo only after ordinary retrieval has selected one relevant visible source and historical reasoning would materially help resume, diagnose, or explain prior work. Retrieved Echo units must retain provenance and remain distinguishable from Memory and visible conversation evidence in synthesized context.

A query model may be used to turn the current need plus selected visible source into a narrow local Echo query. This does not broaden the source scope. For example, a visible source saying that version allocation was changed may produce a local query such as "failure mode of pre-completion global-version allocation"; Echo then ranks only the trace owned by that visible source.

Echo should return a small bounded number of evidence units, not an automatic full-trace dump. If another visible source is needed, it is queried separately.

Dream must not process raw Echo records as ordinary Memory evidence or move them through the Memory lifecycle. Insomnia must not treat Echo as user authority. A future subsystem may derive a Memory from a situation involving Echo only through an explicit policy that preserves these authority boundaries and has visible/user-authoritative support; hidden historical reasoning alone cannot establish durable user intent or truth.

### Storage and vectors

Echo is intended to preserve provider-native reasoning losslessly where that material is actually available, while keeping retrieval accelerators derived and replaceable.

The planned implementation shape is:

```text
immutable compressed provider-native Echo trace
    ↓
derived source-local/search bindings
    ↓
optional lexical/vector ranking inside one selected source
```

Echo vectors are separate from Archive and Memory vectors. They may use normal Compatibility Profile machinery, but no Echo vector population is eligible for ordinary Archive/Memory search.

Because the retrieval universe is already constrained to one source before ranking, Echo does not require a globally discoverable reasoning index. Implementation may use source-keyed local indexes/bindings or another structure that makes `parent_visible_source_id` a mandatory precondition to ranking.

Vector/search input should use substantive raw reasoning content when available and fall back to provider-supplied summaries when raw content is hidden/opaque. Structural/timing-only recaps are provenance by default and should not normally be vectorized unless measurement shows retrieval value.

Provider exports remain the provenance source for imported reasoning. Import normalization may remove redundant serialization only when equivalence has been verified. It must not silently discard unique reasoning, summaries, ordering, signatures, or provider-native opaque payloads that are required for provenance/round-trip fidelity.

## Consequences

- Historical reasoning can be preserved without polluting ordinary context or becoming a shadow global memory system.
- A terse visible update can later be supplemented by the reasoning/work trace behind that **specific** source when the source is already relevant.
- Echo does not require providers to expose equivalent CoT. Raw reasoning, summaries, opaque encrypted objects, and structural markers remain explicitly different representations.
- Provider-native provenance and ordering survive even when retrieval uses smaller derived search units.
- Source sets and conversation-wide Echo expansion are excluded from the initial design to avoid context pollution.
- Echo cannot supersede, canonicalize, or archive Memories because it is outside the Memory lifecycle.
- Echo cannot independently establish factual truth or user intent.
- Echo storage/vector work can be deferred without changing Archive, Memory, Dream, Insomnia, or Ego ownership.
- Providers that expose no reasoning simply produce no Echo trace material for that source; valid conversation import does not depend on Echo availability.

## Rejected alternatives

### Store reasoning as Archive fragments

Rejected. Archive is the visible/source-history authority used by normal retrieval. Mixing hidden reasoning into the same fragment population would make it too easy to retrieve speculative computation as if it were ordinary conversation evidence.

### Convert reasoning directly into Memories

Rejected. Reasoning includes discarded and incorrect intermediate states. Durable Memory formation requires the existing authority and lifecycle rules rather than historical model cognition.

### Globally search all reasoning

Rejected. Global reasoning retrieval would allow stale hypotheses and incidental internal work to become a parallel discovery index over the user's entire history.

### Search Echo by conversation or source set

Rejected for the initial design. Conversation-wide and multi-source retrieval recreate the same context-pollution problem that Reliquary avoids elsewhere: selecting one relevant historical item must not implicitly authorize importing a whole conversation or a candidate set of internal reasoning. Wider retrieval requires separate explicit source selections.

### Treat one provider-native reasoning object as one universal Echo record

Rejected. ChatGPT exports small graph reasoning events; DeepSeek may expose multi-thousand-character `THINK` blocks or many small tool-interleaved fragments; Codex/Hermes exposes short summaries plus encrypted objects across action rows; Claude exposes ordered thinking/tool traces with raw, hidden, summary, and signed states. Provenance units and retrieval units must therefore be distinct concepts.

### Normalize all providers to plaintext CoT

Rejected. Equivalent plaintext does not exist. Some providers expose raw reasoning, some expose summaries, some expose opaque encrypted/signed data, and some expose only structural markers. Echo records what actually exists and labels its representation rather than fabricating missing reasoning.

### Treat reasoning as disposable operational logs

Rejected. Operational logs are transient coordination state and should be reclaimed aggressively. Echo is intentional historical source material whose future value comes specifically from preserving prior computational context.

## Verification

When implemented, focused tests must prove:

- Echo traces remain distinct from visible Archive events and Memories after reopen;
- provider-native trace/event identity, source ordering, and parent visible-source provenance survive persistence and compression;
- Echo search cannot execute without exactly one explicit parent visible source;
- conversation-wide, source-set, neighboring-turn, and global Echo searches are rejected by the initial API;
- ranking cannot return Echo material owned by another visible source;
- ChatGPT-style graph reasoning can bind to the correct visible source without losing preceding/following anchors;
- DeepSeek-style large `THINK` blocks can be segmented for retrieval while the original block remains intact;
- Hermes/Codex tool-loop reasoning can be grouped under the final visible source while opaque encrypted reasoning remains distinct from its plaintext summary;
- Claude-style ordered thinking/tool traces preserve order and raw/hidden/summary/signature distinctions;
- Echo vectors/search bindings cannot enter ordinary Archive/Memory search populations;
- Ego can request a small selected Echo result without automatically injecting the full trace;
- Dream/Insomnia authority paths cannot treat Echo as user-authoritative Memory evidence;
- imports with no provider reasoning remain valid.

## References

- [Architecture](../architecture.md)
- [Roadmap](../roadmap.md)
- [ADR 0001](0001-purpose-built-database-ownership.md)
- [ADR 0012](0012-deterministic-episodes-and-insomnia-memory-authority.md)
- [ADR 0013](0013-immutable-memory-vector-bindings.md)
