# Entity store resolution experiment

This is an isolated calibration harness for the intended two-stage Entity pipeline. **Pass 1 still only extracts mention spans.** This experiment starts after that extraction step.

The second stage searches an already-existing durable Entity store for bounded candidates. Exact canonical-name or alias equality is used only to discover candidates; it never asserts identity. Each candidate is an Entity record with a stable `entity_id`, aliases, kind, semantic summary, and supporting evidence.

For each query the resolver chooses `resolve_existing`, `create_new`, `unresolved`, or `reject`. When two to four Entity candidates exist, every candidate ordering is tested. Semantic results are compared by `entity_id`, so candidate-position bias is visible.

The adversarial fixture contains 17 query Memories: 10 should resolve to one of two same-surface existing Entities, 4 should create a third same-surface Entity, and 3 should remain unresolved.

The larger deterministic calibration corpus is generated separately and does not modify the 17-case smoke corpus:

`python examples/entity_store_resolution_experiment/make_large_fixture.py`

It writes the 150-query `entity-store-resolution-large-v1` fixture with ten gold categories. Its scorer reports query and permutation accuracy by expected decision and by gold category, plus merge/new/unresolved/reject error counts.

The real-data calibration corpus is generated from the frozen 28-day REL harvest:

`python examples/entity_store_resolution_experiment/make_frozen_rel_fixture.py`

It writes `entity-store-resolution-frozen-rel-v1`, preserving each source Memory ID and using manually curated Entity-ID gold backed by separate real Memories. It also records upstream extraction regressions separately when Pass 1 fails to provide a complete identifying mention surface, so resolution is not scored for reconstructing or merging spans it did not receive.

Generate the fixture:

`python examples/entity_store_resolution_experiment/make_fixture.py`

Run:

`cargo run --example entity_store_resolution_experiment --features entity-calibration -- <queries.jsonl> <entities.jsonl> <output-dir> <config-path> [workers] [low|medium|high] [model]`

Defaults are 8 workers (clamped to 16), low reasoning, and `gpt-5.6-sol`. The configured `insomnia_metadata` OpenAI Codex route is reused.

Score against gold separately so gold is never visible to inference:

`python examples/entity_store_resolution_experiment/score_gold.py <results.jsonl> <gold.json>`

The fixture is a calibration stand-in for a production Entity store. It tests resolution semantics and order sensitivity; it does not implement Perception or production Entity persistence.
