# Entity validation experiment

This is a separate calibration harness, not production architecture. It consumes fixed first-pass `results.jsonl` spans, applies the frozen deterministic generic filter, and optionally makes one structured validator call per Memory with remaining candidates. It does not rerun extraction, perform existing-Entity resolution, or create durable IDs. It isolates semantic durable-identity validation over fixed first-pass spans.

Usage:

`cargo run --example entity_validation_experiment --features entity-calibration -- <first-pass-results.jsonl> <candidates.jsonl> <output-dir> <config-path> [workers] [low|medium|high] [model]`

The supplied config must contain the `insomnia_metadata` OpenAI Codex route and its credentials. Defaults are 8 workers, low reasoning, and `gpt-5.6-sol`; workers are clamped to 16. No live experiment is run by this change.
