# GPT-5.6 vs GPT-6 omnibus comparison

## Result

**GPT-6 is not a clean drop-in replacement for the calibrated GPT-5.6 Sol/Luna mix yet.**

| Stage | GPT-5.6 control | GPT-6 run | Signal |
|---|---:|---:|---|
| Insomnia | 19/30 historical semantic-audit accepts across 3 runs | 100% anchor/state/omit; 87.5% grounding; 75% metadata | Promising, different scoring basis |
| Entity extraction | 6/10 exact; F1 0.861 | 6/10 exact; F1 0.824 | 6 lower recall/F1 |
| Entity admission | 8/8 | 7/8 | 6 regression |
| Entity disambiguation | 6/6 | 6/6 | Tie |
| Entity resolution | 9/10 | 8/10 | 6 regression |
| Chronos | 6 inference cases safely unresolved in preserved state | 1 verified, 5 invalid-output errors | **Blocker** |
| Dream | 37-39/40 historical full-context gate | 5/10 text-only selected proxy | Concerning, not apples-to-apples |

### Important details

- GPT-6 extraction had slightly higher precision (0.848 vs 0.838) but materially lower recall (0.800 vs 0.886), producing lower F1.
- Admission regressed on `presenter`: expected `create_new`, GPT-6 returned `unresolved / recurrence_required`.
- GPT-6 resolution added a new miss on `frozen-040` (expected reject, returned create_new) while retaining the zero-candidate `frozen-038` miss.
- Disambiguation remained clean on all six selected adversarial queries.
- Chronos is the clearest drop-in blocker: five of six model-mediated cases generated canonical expressions rejected by the deterministic verifier.
- Insomnia's mixed Sol→Luna→Sol run retained/omitted every selected anchor correctly. It had one grounding-contract miss and two metadata misses.
- Dream's old per-case Luna outputs and baseline CVA are not preserved. The 5/10 GPT-6 result therefore remains a text-pair proxy and should not be treated as a fair direct loss against the historical 37-39/40 gate.

## Existing broader calibration supports the same mixed conclusion

- Entity extraction v2: 5.6 F1 **0.8750** vs 6 **0.8526**.
- Entity admission v3: 5.6 **24/24** vs 6 **20/24**.
- 51-case disambiguation: 6 improved immediate correctness (**48/51** vs **45/51**) while both preserved all 51.
- Frozen-REL 41: 5.6 **41/41** vs 6 **40/41**.

All new run outputs are under `runs/gpt6/` in this directory.
