# Ego Memory-Web synthesis plan

Parent index: [Documentation index](INDEX.md)

## Purpose

Record the future-only policy, rationale, measurements, and calibration work for Ego Memory-Web synthesis without presenting unimplemented behavior as current architecture.

## Overview

The design is settled at the policy level but is not yet implemented. REL Web synthesis will use bounded REL-local activity rather than calendar age, and refresh will be checked periodically but performed only after enough Memory-Web mutation has accumulated. Exact activity-unit calibration remains open. Personality ownership and evidence boundaries are now settled, while its synthesis mechanics remain open.

## Status

This document records the current design for Ego Memory-Web synthesis and the measurements that motivated it.

## Scope

This plan is about the cached owner-local Memory-Web synthesis already supported by Ego persistence.

It does not define:

- Identity synthesis: Identity is explicit, PHY-owned, and high-inertia rather than synthesized;
- Personality synthesis mechanics: Personality is PHY-owned and PHY-derived, but its refresh policy, user-edit interaction, and output shape are still undecided;
- final multi-owner context assembly, hierarchy, prompt budgeting, or Warlock injection; or
- a new semantic relevance classifier.

## Governing principle

The Memory Web remains the durable source of detail. The Ego synthesis is a compact current-orientation layer, not a replacement for retrieval.

A useful synthesis may therefore omit dormant implementation detail as long as the underlying Memory Web and provenance remain retrievable. Separate REL ownership is important here: activity in another project must not make a dormant project's state older or less visible.

Do not make semantic relevance a deterministic preprocessing decision. Deterministic code may enforce explicit validity and activity mechanics; model synthesis owns semantic compression.

## What the measurements ruled out

The deterministic baseline is:

- exclude archived Memories;
- exclude explicitly `historical` Memories;
- collapse duplicate copies already identified by Dream;
- retain `future` Memories.

This cleanup is semantically useful but does not solve scale.

### REL measurements

The frozen 14-day REL reduces to 924 eligible Memories, 146,738 title/body characters, or roughly 36.7k tokens.

An earlier cut of the 28-day REL reduced to 2,079 eligible Memories, 298,130 characters, or roughly 74.5k tokens. After later Insomnia draining, the same experiment fixture contains 2,398 eligible Memories, 342,921 characters, or roughly 85.7k tokens. The later figure is the current stress measurement.

Category/type filtering is not a safe scaling mechanism:

- in the earlier 28-day cut, `decision + fact + constraint` represented about 83% of filtered REL text;
- `project` represented about 91% of filtered REL text;
- the dominant intersections were `decision × project`, `fact × project`, and `constraint × project`.

Those categories are the project state synthesis is supposed to preserve, so dropping them to meet a budget would be arbitrary.

### PHY measurements

The 28-day PHY reduces to 194 eligible Memories, 27,194 characters, or roughly 6.8k tokens.

Its substance is similarly concentrated rather than safely discardable:

- `instruction + preference` represented about 88% of filtered text;
- `process + communication` represented about 89%.

PHY is not currently the same scale problem as REL. Do not assume the REL activity-window policy must be copied unchanged into PHY without measurement.

## Rejected primary scaling approaches

### Whole-Web one-shot synthesis

Rejected as the default REL strategy. The 28-day stress fixture is already around 86k rough input tokens after objective cleanup and can continue growing without bound.

### Full community-by-community synthesis

Dream Communities remain useful semantic structure, but synthesizing every Community still requires eventually reading the whole eligible Web. It moves the context-limit problem into multiple calls without bounding total inference cost.

### Category/type exclusion

Rejected. Categories and types describe semantic role and can help structure a prompt, but the measured dominant categories are exactly the state that must usually survive.

### Calendar recency

Rejected. A project abandoned for six months must not lose current state merely because wall-clock time passed.

### Global user activity recency

Rejected for project state. Work in another project must not age a dormant REL. This failure mode is one reason project state is isolated into separate RELs.

## REL-local activity recency

Initial REL synthesis should operate over a bounded window of recent **REL-local activity**, not recent calendar time.

Let:

- `W` = synthesis activity window;
- `N` = refresh-check interval in activity units;
- `M` = Memory-Web mutation threshold required to justify resynthesis.

The exact definition and size of one activity unit is not yet fixed. It must be deterministic and satisfy these constraints:

- activity is owner-local to the REL;
- inactivity does not advance the activity position;
- activity in another REL does not advance this REL;
- a dormant REL resumes from the activity position at which it was left;
- the unit must not depend on how many Memories Insomnia happened to extract;
- it should not be dominated by model verbosity or tool-turn count; and
- calibration must use real interaction density rather than an arbitrary clock duration.

The design does **not** currently require semantic-community-local activity clocks. REL isolation is the first boundary; add finer semantic clocks only if measurements show REL-local activity is still too coarse.

## Initial synthesis

For an REL with no cached Web synthesis:

1. determine the most recent `W` activity units in that REL;
2. select Memories sourced from that activity window;
3. apply only objective eligibility rules: archived/historical exclusion and Dream-owned duplicate collapse, while retaining future state;
4. provide category/type/temporal/Community metadata as synthesis structure rather than deterministic relevance ranking; and
5. ask the synthesis model to produce a bounded current-orientation representation.

Older Memories remain in the Memory Web and remain retrievable. Falling outside `W` means only that they are not part of the default initial synthesis input; it does not assert that they are false or irrelevant.

## Refresh policy

Do not resynthesize when every Memory is added.

At each `N` REL-local activity units, perform a cheap deterministic check. `N` should be considerably smaller than `W`.

If the Memory Web has accumulated at least `M` mutations since the last successful synthesis, resynthesize. Otherwise do nothing and carry the mutation count forward to the next check.

Conceptually:

```text
every N activity units:
    if memory_web_mutations_since_synthesis >= M:
        resynthesize from the current W-unit activity window
    else:
        keep the cached synthesis
```

There is deliberately **no age-based resynthesis backstop**. `W` is the amount of recent project activity considered during synthesis; it is not a maximum allowed synthesis age. A synthesis may remain valid for arbitrarily many activity units if the Memory Web barely changes.

`M` is a mechanical change threshold, not a semantic importance score. New-Memory count is the canonical example. Exact accounting for edits, archives, supersession/lifecycle changes, and other eligibility-changing mutations should be fixed during implementation rather than inferred ad hoc.

No model call is needed merely to decide whether another model call is warranted.

## Stress fixture

The current 28-day experiment is intentionally a harsh calibration case:

`fixtures/local/experiments/chatgpt-first28d-insomnia-codex-drain-20260904/project.prj.rel`

The Archive spans 29 Pacific calendar dates from 2026-05-15 through 2026-06-12 and contains 16,465 unique conversation turns. Using a 30-minute gap only as a diagnostic estimate of engaged conversation time, 23 of 29 dates exceed six engaged hours and 17 exceed eight hours. This diagnostic is not the activity-unit definition.

The newest three archive dates currently have no extracted eligible REL Memories, so the Memory-window measurement is anchored to the latest Memory-bearing day, 2026-06-09.

Cumulative eligible Memory input ending on that day is:

| Recent active dates | Memories | Rough tokens |
| ---: | ---: | ---: |
| 1 | 171 | 5.7k |
| 3 | 487 | 16.3k |
| 5 | 799 | 26.1k |
| 7 | 1,080 | 35.7k |
| 10 | 1,337 | 44.9k |
| 14 | 1,627 | 54.0k |
| 21 | 2,291 | 82.5k |
| 26 | 2,398 | 85.7k |

This shows that a fixed “last 30 active days” rule is still too large for a heavily used REL. The purpose of activity units is to normalize project recency more finely than calendar or active-day counts.

## Synthesis semantics

The synthesis should favour current orientation rather than historical completeness. Repeated, recent, cross-cutting, and still-referenced state is naturally more likely to survive model compression, while detailed dormant state may fall out and remain available through retrieval.

Do not convert that observation into a hand-built deterministic “loudness” score unless later measurements justify one. Recency/activity provides bounded candidate input; the model decides semantic compression inside that input.

## Personality boundary

Memory-Web synthesis is sufficiently specified to implement independently. Personality ownership and evidence scope are also now settled, but its synthesis mechanics are not.

Personality is:

- PHY-owned and persisted only in the PHY;
- synthesized only from PHY-owned Memory state;
- never derived from REL/project state;
- lower inertia than Identity;
- user-editable;
- optionally self-adaptive; and
- not a semantic authority over factual Memory state.

Personality should be a behavioural projection of the user, not a second generic user summary. Its synthesis evidence should therefore be restricted to PHY Memories that describe behaviour, interaction style, or durable preferences. Current intended evidence lanes are:

- `communication` preferences and interaction style;
- `process` preferences and recurring ways of working;
- `relationship` preferences or recurring interpersonal expectations where applicable; and
- recurring behavioural patterns represented in PHY state.

Generic user facts are not Personality evidence merely because they live in the PHY. Identity-like facts, education, employment history, possessions, location, and other factual biography should remain ordinary PHY Memory/Web state unless they directly encode a behavioural preference. REL/project Memories are excluded entirely.

Conceptually:

```text
PHY Memory Web
    ↓
behavioural evidence selection
    ↓
PersonalitySynthesizer
    ↓
PHY EgoPersonality
```

Still undecided:

- exact behavioural evidence-selection rules and category/type mapping;
- whether the generic PHY Web synthesis contributes any input in addition to selected Memories;
- Personality activity/window and resynthesis policy;
- treatment of user-authored Personality edits during later automatic synthesis; and
- output shape and budget.

Do not implement Personality inference by simply reusing the generic Web-synthesis policy until those remaining questions are resolved.

## Implementation/calibration sequence

1. Define a deterministic REL-local activity-unit representation that satisfies the constraints above.
2. Measure the 28-day stress fixture at multiple candidate `W` values in activity units.
3. Select initial `W`, `N`, and `M` from measured cost/coverage rather than arbitrary calendar durations.
4. Implement initial REL Web synthesis using the objective eligibility baseline and bounded activity window.
5. Implement periodic `N`-unit checks and `M`-threshold resynthesis without an age backstop.
6. Validate that dormant RELs do not age while other RELs are active.
7. Measure PHY separately before deciding whether to share the same activity policy.
8. Design Personality synthesis mechanics as a separate follow-up, using only PHY behavioural evidence and persisting the result in PHY.

## Related docs

- [Roadmap](roadmap.md)
- [Current limitations](current-limitations.md)
- [Architecture](architecture.md)
- [Memory scope design record](reliquary-phylactery-memory-scope-plan.md)
- [Dream design and validation record](dream-implementation-plan.md)

## Notes

When Web synthesis ships, move implemented behavior into the current-state architecture/API/storage owners and keep only unresolved calibration or follow-up work here.