# Entity resolution experiment

This separate calibration harness tests second-pass resolution over frozen first-pass `actual_entity_mentions`; it does not rerun extraction, validate binary identity, create durable IDs, or modify production behavior.

Usage:

`cargo run --example entity_resolution_experiment --features entity-calibration -- <first-pass-results.jsonl> <candidates.jsonl> <output-dir> <config-path> [workers] [low|medium|high] [model]`

The configured `insomnia_metadata` OpenAI Codex route and authentication are reused exactly as in v1. Defaults are 8 workers, low reasoning, and `gpt-5.6-sol`; workers are clamped to 16. No live experiment is run by this change.

The repeated-surface inventory is a provisional experiment stand-in for a real owner-local Entity store. It does not prove identity equivalence and does not implement Perception. Exact repeated surfaces and owner-self phrases resolve deterministically; remaining candidates receive one structured model decision per Memory. Malformed model output is preserved conservatively as unresolved.
