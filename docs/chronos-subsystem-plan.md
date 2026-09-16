# Chronos subsystem plan

Parent index: [Documentation index](INDEX.md)

Decision owner: [ADR 0035](decisions/0035-chronos-shared-temporal-semantics.md)

## Status

Shared-core extraction, the first parser-independent indication/normalization seam, numeric/word-number relative day/week/month/year offsets, and deterministic bounded/open interval parsing are implemented; further deterministic grammar expansion and consumer integrations beyond Dream remain in progress.

Chronos now owns the shared temporal model, detector, bounded vocabulary normalizer, parser, calendar resolution, recurrence extraction, and matcher under `chronos*`. `chronos::detect` preserves original indication spans even when parsing fails; `chronos::analyze` may use a transient corrected view but maps any corrected evidence back to the exact original text. Dream preserves its existing public behavior through a thin Memory/source-time adapter and compatibility aliases for the former `DreamTemporal*` public type names.

## Overview

Chronos extracts the existing Dream temporal machinery into one shared deterministic-first temporal layer, then expands its coverage for Insomnia, Dream, and Perception without creating another semantic owner or temporal stack.

## Purpose

Chronos is Reliquary's shared temporal-semantics subsystem. It provides deterministic-first temporal detection, normalization, parsing, resolution, comparison, and valid-time interpretation to Insomnia, Dream, and Perception. Bounded inference handles only temporal meaning unresolved after deterministic analysis.

Chronos owns temporal interpretation mechanics, not Memories, Graph relationships, Entities, Observations, scheduling, or REL/PHY transaction history.

## Temporal axes

```text
source/reference time  when evidence originated; resolves relative language
valid time             when the proposition/state applies in the world
transaction time       when the REL/PHY committed the state
```

Chronos handles source/reference-time resolution and valid-time interpretation. Transaction/knowledge time comes from timestamped global/container versions.

## Shared processing shape

```text
semantic unit + reference chronology
    -> indication detector
    -> bounded normalization
    -> deterministic parser/resolver
    -> temporal analysis
         -> complete: return derived result
         -> unresolved + relevant: bounded inference fallback
```

The semantic unit differs by consumer; temporal primitives remain shared.

## Deterministic modules

### Indication detector

High-recall gate for possible world-time information. Detect at least:

- explicit dates/times, months, weekdays, years, seasons, quarters;
- relative tokens and numeric/word-number relative constructions;
- range/boundary language;
- recurrence/frequency and duration language; and
- bounded typo variants of known temporal vocabulary.

A positive indication means interpretation may be needed; it does not assert a temporal fact.

### Normalizer

Build a transient detection view without changing authoritative text. Support deterministic format normalization, word-number recognition, bounded fuzzy matching against temporal vocabulary, and original-span/evidence preservation. Do not perform broad spelling correction.

### Parser/resolver

Generalize the current Dream absolute/relative/recurrence parser. Expand coverage for:

- numeric and word-number days/weeks/months/years relative to reference time;
- explicit ranges and open boundaries;
- recurrence and durations;
- common calendar periods; and
- safe loose calendar forms with one deterministic interpretation.

Resolve against authoritative source/reference chronology. Never substitute Memory creation/import bookkeeping when semantic source time is required.

### Matcher/comparator and validity

Provide deterministic overlap, ordering, recurrence identity, compatibility, and proposition-level valid-time interpretation where syntax and semantic-unit structure make that safe.

The valid-time representation now includes `TemporalInterval` with independently optional start/end bounds and per-bound granularity. Existing bounded `Range` anchors remain as compatibility projections for Dream. Recurrence remains represented separately; add uncertainty only when actual semantics require it.

Current Dream temporal-match behavior is now implemented behind this seam with compatibility tests.

## Inference fallback

Run only when all are true:

1. temporal indications exist;
2. the consumer needs a temporal answer; and
3. deterministic Chronos cannot safely resolve it.

The model receives original semantic state, reference chronology, deterministic evidence/candidates, and a narrow unresolved question. It does not regenerate deterministic anchors already known.

## Consumer contracts

### Insomnia

Insomnia consumes Chronos during Memory processing for newly extracted, imported, user-created, or otherwise newly introduced bodies, plus replacement/correction bodies whose previous temporal inference no longer applies.

```text
Memory body
    -> Chronos deterministic analysis
    -> no indication: stop
    -> fully resolved: continue without model call
    -> unresolved: bounded Chronos inference
```

Body/version identity must make this idempotent and invalidate inferred temporal state after semantic replacement.

### Dream

Dream consumes Chronos for temporal candidate discovery, source-relative content-time interpretation, anchor/range overlap, recurrence matching, temporal ordering, and pair-classification context.

Current Dream temporal behavior remains compatible with parser/matcher ownership now in Chronos.

### Perception

Perception consumes Chronos after selecting bounded evidence for Observation synthesis or reconsideration. Chronos may compare supporting Memory temporal evidence, derive Observation validity, expose temporal conflict/ambiguity, or invoke bounded fallback inference.

Perception still owns the Observation being synthesized.

## Persistence policy

Deterministic Chronos products remain derived by default. Do not persist parsed anchors, normalized typos, recurrence patterns, or deterministic intervals merely to avoid cheap recomputation.

Persist only non-deterministically inferred temporal conclusions that a durable consumer actually needs. Such state must retain enough semantic input/body/version identity to detect staleness.

A deterministic cache is permitted as disposable optimization state, never semantic authority.

## Implementation sequence

### A — Shared core — implemented

- `chronos::analyze` is the shared deterministic analysis boundary.
- The former Dream temporal model/parser/calendar/relative/recurrence/matcher modules now live under Chronos ownership.
- Generic `Temporal*` types are canonical; existing `DreamTemporal*` names remain compatibility re-exports.
- Dream resolves authoritative Memory source time and delegates analysis/matching to Chronos.
- Existing Dream temporal tests remain green, with direct Chronos contract tests added.

### B — Detection and deterministic coverage — partially implemented

- Parser-independent `chronos::detect` now reports explicit/calendar/relative/boundary/recurrence/contextual-duration indications with original byte spans.
- Bounded one-edit/transposition correction now covers selected temporal vocabulary with contextual guards; normalized tokens are transient and corrected parse evidence maps back to the exact original span.
- Numeric and English word-number relative day/week/month/year offsets are implemented through ninety-nine for word forms; month/year shifts use calendar-aware end-of-month clamping.
- Bounded `from … to/through/until …` ranges and open `since`/`until`/`before`/`after`/`starting`/`ending` boundaries are implemented as derived `TemporalInterval` values; bounded intervals also retain compatibility `Range` anchors.
- Boundary endpoints reuse deterministic explicit/relative parsing and support reference-bound bare months/weekdays, including cross-year month ranges.
- Expand standalone duration, recurrence, seasons, and broader safe loose-calendar parsing.
- Expand/calibrate the indication vocabulary and fuzzy thresholds against measured typo recall and false positives rather than broad spell correction.

### C — Insomnia integration

- Invoke Chronos for eligible Memory bodies.
- Gate temporal model calls behind deterministic unresolved state.
- Define body/version-bound inferred temporal persistence and staleness.

### D — Dream consumer completion — partially implemented

- Parser/matcher ownership and temporal candidate matching now route through Chronos.
- Preserve temporal candidate retrieval and classifier context behavior as deterministic coverage expands.
- Add shared temporal ordering/validity outputs to Dream only when those Chronos primitives are implemented; Dream retains Memory-to-Memory semantic authority.

### E — Perception integration

- Add Chronos input/output to Observation processing contracts.
- Define deterministic multi-evidence temporal synthesis.
- Use fallback inference only for unresolved semantics.

### F — Validation

Measure deterministic parse coverage, indication recall, fuzzy-detection false positives, inference rate/cost avoided, edit/replacement invalidation, and compatibility with existing Dream temporal retrieval fixtures.

## Open implementation decisions

- final Chronos API surface beyond the implemented `chronos::detect` / `chronos::analyze` boundaries, detection model, generic `Temporal*` model, and internal matcher;
- indication-vocabulary coverage and calibrated fuzzy thresholds beyond the implemented conservative first pass;
- remaining valid-time representation details beyond implemented optional-bound intervals, especially uncertainty and multi-evidence composition;
- persistence schema for inferred-only conclusions;
- model route used by fallback inference;
- consumer adapters for deterministic validity synthesis; and
- whether any deterministic analysis warrants a disposable cache.

## Notes

Transaction/knowledge time is now implemented by the Container/global-version stream. Chronos must consume that boundary where historical belief-state cuts matter but must not duplicate or reinterpret it as valid time.

## Related docs

- [ADR 0035](decisions/0035-chronos-shared-temporal-semantics.md)
- [Dream design and validation record](dream-implementation-plan.md)
- [Perception subsystem plan](perception-subsystem-plan.md)
- [Roadmap](roadmap.md)
- [Versioning, historical cuts, and rollback](version-history-plan.md)
