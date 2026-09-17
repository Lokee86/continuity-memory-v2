# Entity calibration runner

Runs the Insomnia post-wording Entity/lexical enrichment pass against a curated JSONL gold set without rerunning Insomnia semantic extraction.

Build/run with the `entity-calibration` feature. The runner requires the configured `insomnia_metadata` route, preserves that route's credential, overrides the model to `gpt-5.6-luna`, and uses `INSOMNIA_TEST_REASONING=low|medium|high` (default `low`).

Gold cases are evaluated one Memory per inference so every score uses the same inference condition. Independent cases run concurrently; the optional final argument controls workers (default 8, maximum 16). Endpoint/transport failures stop the run, while deterministic invalid-output failures are recorded on the affected case.

The runner writes `results.jsonl` plus `summary.json` with exact-span Entity precision/recall/F1, exact-case and zero-Entity accuracy, required lexical-term recall, and invalid-output counts.
