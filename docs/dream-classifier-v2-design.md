# Dream classifier v2 candidate design

Parent: [Dream implementation plan](dream-implementation-plan.md)

## Purpose

This document freezes the next unmeasured Dream classifier prompt candidate and the evidence gates it must clear before replacing the measured v1 baseline.

## Overview

Classifier v1 preserves same-workstream recall but over-links broad project context. The rejected first v2 fixes those false positives by over-pruning almost every useful cross-entity workstream relation. The replacement design uses semantic workstream/scope as the decision boundary and is evaluated first with a 40-case exact-pair fixture before any full population run.

## Status

Design-only candidate. Classifier source remains on measured contract v1 until this prompt clears the exact-pair boundary harness with live inference.

## Why v1 and the rejected v2 both fail

The 40-case classifier boundary fixture isolates the tradeoff:

```text
contract        related        unrelated       overall
v1              29/30 96.7%     0/10  0.0%     29/40 72.5%
rejected v2      1/30  3.3%    10/10 100.0%    11/40 27.5%
```

v1 treats broad project/domain association as enough for `topical`. The rejected v2 corrected those false positives but effectively required near-literal entity identity and destroyed legitimate relations across one semantic workstream.

The missing abstraction is **semantic workstream/scope**.

A workstream can contain different entities and artifacts when they participate in the same specific protocol, workflow, implementation concern, operational context, user preference, or problem. Conversely, two Memories can belong to the same project and share vocabulary while still occupying different workstreams.

## Boundary rule

A pair is related when one of these is true:

- both Memories concern the same specific semantic workstream/problem and retrieving them together is useful;
- one is a constraint, policy, design, schema, implementation state, or observed behavior of the workstream described by the other;
- one is a summary and the other is a concrete state/capability encompassed by that summary;
- one is a general rule/preference and the other is a scoped application or specialization of it;
- one is an analogy or explanatory frame that directly informs the concept asserted by the other.

A pair is unrelated when its only bridge is:

- membership in the same project/domain/codebase;
- chronology or nearby implementation stages;
- project identity/history versus unrelated implementation detail;
- shared generic vocabulary across different entities/subsystems;
- vector similarity or existing Graph proximity without semantic support in the Memories.

The important distinction is **same workstream**, not **same noun** and not **same project**.

## Candidate system prompt

```text
You classify the semantic relationship between exactly two durable Memories for a persistent Memory graph.

The pair is presented in canonical MemoryId order as A and B. A/B do NOT mean source/candidate, old/new, cause/effect, or processing order. Determine direction only from semantic evidence in the Memories.

Primary decision rule: compare the Memories' most specific semantic workstream or scope, not just their literal entities or shared words. A workstream is a specific protocol, workflow, problem, implementation concern, operational context, user preference, design decision, or concept being reasoned about.

A relationship may exist even when the Memories describe different entities or artifacts if those entities participate in the same specific workstream. For example, a protocol migration constraint can relate to the current message schema; two unresolved states can relate when they are parts of the same user flow; a general interaction preference can relate to a scoped application of that preference; and a current-status summary can relate to a concrete capability that the summary encompasses.

Do NOT create a relationship merely because both Memories belong to the same project, domain, codebase, conversation, or time period. Project identity/history does not relate to unrelated implementation details. Different subsystems are not related merely because they share terms such as state, collision, packet, player, ship, spawning, or implementation. Existing Graph relations and retrieval similarity are context only, not proof.

Choose exactly one relation:
- none: no useful semantic relationship at the specific workstream/scope level.
- topical: the Memories belong to the same specific semantic workstream/problem and are useful to traverse together, but no narrower relation below applies.
- factual: one Memory supplies a fact, premise, constraint, design fact, schema, or state that materially informs interpretation of the other. Direction is from the supporting/context Memory to the Memory it informs.
- causal: one Memory explicitly causes, enables, prevents, or materially produces the state/event in the other.
- recurrent: distinct observations/occurrences of the same pattern, not semantically identical duplicates.
- duplicate_of: materially the same durable proposition or observation with the same operative scope and no meaningful semantic difference. General and scoped variants of the same preference/rule are not duplicates unless their operative scopes are materially equivalent.
- supersedes: one Memory explicitly corrects, replaces, invalidates, or becomes the operative version of the other.

Before choosing none, ask whether retrieving B while investigating A's specific workstream would provide directly useful context, or vice versa. Before choosing topical, ask whether the connection would still exist if the broad project/domain label were removed. If the only remaining bridge is generic vocabulary or project proximity, choose none.

Abstract positive boundaries:
- compatibility/migration policy <-> current protocol/message implementation: related;
- two states or defects inside one specific user flow: related;
- workspace operating policy <-> a concrete repository/configuration fact inside that workspace: related;
- general stepwise-guidance preference <-> a troubleshooting/prompt-granularity specialization: related;
- current implementation summary <-> a concrete current capability encompassed by that summary: related.

Abstract negative boundaries:
- project-history/identity observation <-> unrelated later logger/network/gameplay implementation: none;
- ship/runtime architecture <-> asteroid spawning merely because both are gameplay code: none;
- asteroid collision setup <-> ship collision lookup merely because both contain the word collision: none.

Prefer a narrower supported relation over topical, but do not choose none merely because the useful same-workstream relationship is not directional.

Direction rules:
- none -> direction none.
- topical, recurrent, duplicate_of -> direction undirected.
- factual, causal, supersedes -> direction a_to_b or b_to_a according to meaning.

Source timestamps are chronology context only. Deterministic temporal anchors/patterns are evidence about dates, ranges, and recurrence, but are not by themselves proof of causality, supersession, or recurrence between the two Memories. Existing Graph relations are supplemental context only and are not proof of the pair conclusion.

For every non-none conclusion, provide exactly two short verbatim evidence quotes: one copied from A title/content and one copied from B title/content. The quotes must support the direct semantic relationship, not merely broad shared context. For none, evidence must be empty. Do not paraphrase evidence.
```

## First live gate

Run `examples/dream_classifier_tune/` against `corpus/dream-classifier-regressions-v1.json` before any full population run.

Initial advancement threshold:

```text
related recall:      >= 27/30  (90%)
unrelated accuracy:  >=  8/10  (80%)
overall:             >= 35/40  (87.5%)
errors:               0
```

The 7 medium-confidence positives should be reported separately. A candidate may still advance if one or more of those fail, provided high-confidence behavior is strong and the failures justify revisiting the gold rather than widening the prompt.

## Population gate after boundary success

Only then rerun the durable 46-Memory population fixture at candidate limit 12 / pair concurrency 12. Required checks:

- 46/46 sources complete with zero recoverable failures;
- extracted backlog drains to zero;
- whole-web gold improves on classifier-v1's 87/98 baseline;
- the 10 audited false-positive pairs remain absent;
- troubleshooting/step-by-step relation is present;
- relation density is audited rather than optimized for being lower or higher by itself.

Do not tune candidate retrieval during this classifier experiment; retrieval already finds the tracked pairs at the intended population limit.

## Related docs

- [Dream implementation plan](dream-implementation-plan.md)
- [Development](development.md)
- [Rust API](api.md)
- [Behavioral contracts](behavioral-contracts.md)

## Notes

This page records an experimental prompt candidate, not shipped classifier behavior. The source classifier remains on contract v1 until a live exact-pair run clears the documented advancement gate and a subsequent population run confirms improvement.
