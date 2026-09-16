# Chronos subsystem plan

Parent index: [Documentation index](INDEX.md)

Decision owner: [ADR 0035](decisions/0035-chronos-shared-temporal-semantics.md)

## Status

Accepted architecture; implementation not yet started as a standalone shared subsystem.

Current temporal behavior remains implemented under `dream_temporal*`. Chronos modularizes and expands that machinery rather than creating a parallel stack.

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

The valid-time representation must support points/intervals, open bounds, recurrence, and granularity. Add uncertainty only when actual semantics require it.

Current Dream temporal-match behavior should migrate behind this seam with compatibility tests.

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

Current Dream temporal behavior remains compatible while parser/matcher ownership migrates to Chronos.

### Perception

Perception consumes Chronos after selecting bounded evidence for Observation synthesis or reconsideration. Chronos may compare supporting Memory temporal evidence, derive Observation validity, expose temporal conflict/ambiguity, or invoke bounded fallback inference.

Perception still owns the Observation being synthesized.

## Persistence policy

Deterministic Chronos products remain derived by default. Do not persist parsed anchors, normalized typos, recurrence patterns, or deterministic intervals merely to avoid cheap recomputation.

Persist only non-deterministically inferred temporal conclusions that a durable consumer actually needs. Such state must retain enough semantic input/body/version identity to detect staleness.

A deterministic cache is permitted as disposable optimization state, never semantic authority.

## Implementation sequence

### A — Shared core

- Introduce the Chronos module boundary.
- Move/generalize current Dream temporal model/parser/matcher behind it.
- Preserve current Dream temporal tests and behavior.

### B — Detection and deterministic coverage

- Add indication detection independent of full parsing.
- Add bounded fuzzy temporal-vocabulary recognition with original-span preservation.
- Add numeric and word-number relative units including months/years.
- Expand range/boundary, duration, recurrence, and safe loose-calendar parsing.
- Measure typo recall and false positives.

### C — Insomnia integration

- Invoke Chronos for eligible Memory bodies.
- Gate temporal model calls behind deterministic unresolved state.
- Define body/version-bound inferred temporal persistence and staleness.

### D — Dream migration

- Replace Dream-specific temporal plumbing with Chronos calls.
- Preserve temporal candidate retrieval and classifier context behavior.
- Remove duplicate parser ownership after compatibility is proven.

### E — Perception integration

- Add Chronos input/output to Observation processing contracts.
- Define deterministic multi-evidence temporal synthesis.
- Use fallback inference only for unresolved semantics.

### F — Validation

Measure deterministic parse coverage, indication recall, fuzzy-detection false positives, inference rate/cost avoided, edit/replacement invalidation, and compatibility with existing Dream temporal retrieval fixtures.

## Open implementation decisions

- exact Chronos API surface;
- indication vocabulary and fuzzy thresholds;
- valid-time result representation;
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
