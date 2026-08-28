# Community routing benchmark — 2026-08-28

Parent index: [Documentation index](INDEX.md)

## Purpose

Record why the 2026-08-28 46-Memory routing run is quarantined and define the fixture boundary required for any replacement routing experiment.

## Overview

The original routing run used the older monolithic 46-Memory tuning CVA, migrated wholesale into a Project REL and then populated by Dream. Although that produced a real persisted Dream Graph, it did not reproduce the current owner-split architecture because User Memories that should live in the Phylactery remained inside the Project REL.

The run is therefore historical implementation evidence only, not valid owner-split routing evidence. The generated RELs are quarantined locally, and the benchmark intentionally has no default REL fixture path.

## Status

**Superseded / quarantined. Do not use this run as architectural routing evidence.**

The generated local RELs from that experiment are quarantined under:

```text
target/quarantine/legacy-46-memory-routing-20260828/
```

`RELIQUARY_ROUTING_REL` must now be supplied explicitly when the ignored routing benchmark is run. There is intentionally no fallback path that can silently revive the monolithic 46-Memory fixture.

## Correct owner-split baseline

The authoritative same-owner validation from 2026-08-27 used the frozen 11-Episode / 314-turn corpus through production Insomnia ownership routing:

```text
Project REL
- 41 Memories
- 41 Memory-vector bindings
- Dream backlog drained to zero
- 121 active REL-local Dream relations

User PHY
- 8 Memories
- 8 Memory-vector bindings
- Dream backlog drained to zero
- 16 active PHY-local Dream relations
```

Dream processing was complete for both owners. The Project REL had one malformed classifier response on its first pass; the source remained `extracted` as designed and completed on retry. The PHY completed all eight sources without inference failures.

Those exact disposable REL/PHY files are no longer present in the local workspace. The validation result is frozen in [ADR 0024 — owner-local Dream processing](decisions/0024-owner-local-dream-processing.md) and [Development](development.md).

## Next routing experiment

Do not revive or regenerate the quarantined 46-Memory Project REL for community-routing work.

Prepare a fresh owner-split fixture through the normal path:

```text
frozen source corpus
→ fresh Project REL + fresh PHY
→ production Insomnia owner routing
→ Memory-vector fill in each owner
→ Dream(Project REL) to zero extracted
→ Dream(User PHY) to zero extracted
→ persist owner-local Community snapshots independently
```

Run Project routing against the Project REL only. PHY remains a separate owner-local graph and should be measured separately if its population is large enough to make the result meaningful.

The production decision remains unchanged: Community routing stays test-only until a materially larger persisted Dream Memory Web demonstrates high recall while excluding a useful fraction of fine-search vectors/nodes.

## Related docs

- [ADR 0024 — owner-local Dream processing](decisions/0024-owner-local-dream-processing.md)
- [ADR 0025 — owner-local derived communities](decisions/0025-owner-local-derived-communities.md)
- [Community scan-and-merge benchmark — 2026-08-27](community-scan-merge-benchmark-2026-08-27.md)
- [Development](development.md)

## Notes

The quarantined files live under `target/` and are intentionally untracked. Git history retains the superseded measurement details if they are needed for forensic comparison. The tracked benchmark code remains test-only and now requires an explicit current Project REL path.
