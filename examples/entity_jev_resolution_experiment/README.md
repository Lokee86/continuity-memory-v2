# Jev Entity resolution experiment

This calibration harness exercises the dedicated `entity_resolution_decision` endpoint against the frozen Entity fixtures. It uses the same production `ConfiguredDecisionEndpoint` implementation as the runtime.

Modes:

- `admission` — finite create/unresolved/reject Choice experiment.
- `admission-noul` — probability that a first-seen mention is a durable Entity.
- `disambiguation` — finite choice among existing candidates plus create/unresolved/reject.

Production policy is deliberately narrower than the experiment. The runtime decision fast path is used only for non-empty candidate sets and can terminally return only `ResolveExisting`. The selected candidate must have probability >= 0.95 and a >= 0.15 margin over the next-highest choice. Everything else, including endpoint failure, falls through to the configured generative `entity_resolution` route.

Zero-candidate Admission remains generative because its successful path also materializes Entity `kind` and `summary`; adding a decision call there would not currently remove a generative call.

Usage:

`cargo run --example entity_jev_resolution_experiment --features entity-calibration -- <admission|admission-noul|disambiguation> <first-pass-results.jsonl> <candidates.jsonl> <output-dir> <config-path> [workers] [model]`
