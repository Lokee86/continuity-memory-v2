# Entity calibration runner

Runs the Insomnia post-wording **Entity-mention enrichment** pass against a curated JSONL gold set without rerunning Insomnia semantic extraction.

Build/run with the `entity-calibration` feature. The runner requires the configured `insomnia_metadata` route, preserves that route's credential, and overrides the model to `gpt-5.6-luna`. Reasoning effort is an explicit positional argument (`low|medium|high`, default `low`) so benchmark routing cannot silently change with process environment.

Gold cases are evaluated one Memory per inference so every score uses the same inference condition. Independent cases run concurrently; the fourth argument controls workers (default 8, maximum 16) and the fifth controls reasoning effort. Endpoint/transport failures stop the run, while deterministic invalid-output failures are recorded on the affected case.

The runner writes `results.jsonl` plus `summary.json` with exact-span Entity precision/recall/F1, exact-case and zero-Entity accuracy, and invalid-output counts.

Lexical routing is no longer model-calibrated. REL and PHY derive lexical candidates deterministically from complete Memory title/content through the disposable Memory lexical index, using the same lexical machinery as Archive search.
