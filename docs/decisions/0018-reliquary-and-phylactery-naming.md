# ADR 0018: Reliquary and Phylactery naming

Status: Accepted
Date: 2026-08-24
Owners: product identity, project-memory boundary, user-memory boundary
Supersedes: Continuity product naming
Superseded by: none

## Context

The `Continuity` product name is no longer suitable as the public/internal product identity. At the same time, the memory architecture is separating two distinct persistence and retrieval domains rather than treating all durable state as one omnipresent memory pool.

Project/workspace memory is provenance-rich, independently persistent, and retrieved only for the active or explicitly consulted project. User-global memory is intended to persist across projects and carry durable user identity, preferences, habits, and other genuinely cross-project state. The user-global store may retain source provenance when policy permits, but source turns are not required for its validity.

## Decision

The project/workspace memory system is named **Reliquary**.

The user-global memory/profile system is named **Phylactery**.

These names describe separate persistence/retrieval owners:

- **Reliquary** owns project/workspace-local durable state and its project provenance.
- **Phylactery** owns user-global durable state intended to follow the user across projects.

Normal context assembly may retrieve from both the active Reliquary and Phylactery. Unrelated Reliquaries are not part of ordinary retrieval merely because they belong to the same user.

The exact Phylactery storage schema and the Insomnia `user | project` classification boundary remain future design work. In particular, this ADR does not decide whether scope classification belongs in the existing metadata pass or in a dedicated pass.

## Rename boundary

The Rust package/crate and user-facing development surfaces adopt the new name:

- core package: `reliquary-memory`;
- Rust crate path: `reliquary_memory`;
- CLI package: `reliquary-cli`;
- CLI binary: `reliquary`;
- local config default: `reliquary.cfg`;
- transitional local master-key filename: `reliquary.master-key.json`.

The rename does **not** by itself change storage-format identity or deterministic identity domains. Existing `.cva` format markers, `CVA*` record magic, `CVCFG` config framing, and established hash/domain-separation constants remain stable unless a separate versioned format/identity decision changes them. Historical benchmark/source snapshots also retain the names present in their captured source material.

The repository checkout/remote name may remain `continuity-memory-v2` as a technical legacy path until repository hosting and linked-worktree migration are handled separately. That path is not the product identity.

## Consequences

- Warlock documentation and Rust integration refer to Reliquary rather than Continuity.
- New user-global memory design work uses Phylactery terminology rather than generic `Profile CVA` terminology.
- Existing CVAs and deterministic identifiers are not invalidated merely to complete a branding rename.
- Some legacy lowercase `continuity-*` strings remain intentionally inside compatibility probes, deterministic hash domains, historical fixtures, or filesystem test names; they are not user-facing product naming.

## Related docs

- [Reliquary and Phylactery memory scope plan](../reliquary-phylactery-memory-scope-plan.md)
- [Architecture](../architecture.md)
- [Roadmap](../roadmap.md)
- [ADR 0017 — CVA workspace and Warlock host application](0017-cva-workspace-and-warlock-host-application.md)
