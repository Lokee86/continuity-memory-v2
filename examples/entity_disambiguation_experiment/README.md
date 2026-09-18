# Entity disambiguation experiment

This isolated calibration harness tests whether lexically identical mention surfaces are treated as ambiguous, context-sensitive evidence rather than deterministic identity. It does not change production source or either existing experiment.

Usage:

`cargo run --example entity_disambiguation_experiment --features entity-calibration -- <first-pass-results.jsonl> <candidates.jsonl> <output-dir> <config-path> [workers] [low|medium|high] [model]`

Defaults are 8 workers (clamped to 16), low reasoning, and `gpt-5.6-sol`. The configured `insomnia_metadata` OpenAI Codex route and authentication are reused. Owner-self is the only deterministic resolution; every other mention receives one decision in a single structured call per Memory. Malformed model output is conservatively unresolved; transport errors are fatal.

Prior Memory anchors are experimental stand-ins for an actual Entity candidate index. This can test whether lexical-match candidates are treated as ambiguous and context-sensitive, but cannot validate that a chosen prior anchor is truly the correct Entity: gold data has mention spans, not Entity IDs.
