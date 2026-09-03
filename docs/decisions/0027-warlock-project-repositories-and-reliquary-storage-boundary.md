# ADR 0027: Warlock project repositories and Reliquary storage boundary

Parent index: [Architectural decisions](INDEX.md)

## Status

Accepted — 2026-09-02.

## Context

Reliquary accumulated project/file storage, historical recovery, whole-container ancestry, and conflicted-copy machinery while also serving as the purpose-built memory/context database for Warlock. A later migration plan proposed embedding Lore beneath the REL/PHY so Lore would own the physical storage and revision history for both project files and semantic state.

The Lore feasibility work proved that Lore is capable of single-artifact storage, ordinary working-tree reconstruction, repository history, branching, merge, binary/chunked content, and divergent-history recovery. The same investigation also clarified that these capabilities solve a different problem from Reliquary's Memory, Graph, Episode, Echo, vector, and provenance workloads.

Warlock also needs a consistent project-state model even for users who do not understand or intentionally operate version control. Software users may already have Git and should retain explicit ownership of that repository rather than receiving a second hidden VCS for the same files.

## Decision

Warlock separates **project state/history** from **agent memory/context state**.

### Every project has a repository

A Warlock project must have one project repository:

- if the project already uses Git, or the user explicitly selects Git, Git remains the project repository;
- otherwise Warlock automatically initializes and manages a Lore repository for the project.

Lore is the default managed repository for ordinary non-Git projects. Its routine checkpointing and history maintenance may remain invisible to users who do not need VCS concepts.

Git is treated as an advanced/user-owned repository. Warlock may inspect and mutate it through ordinary agent workflows, but repository-changing operations are explicit and visible rather than silently managed as hidden housekeeping.

Advanced explicit repository controls may also be exposed for Lore projects.

### Reliquary and Phylactery remain purpose-built semantic stores

REL and PHY do not become Lore repositories and do not migrate their semantic owners wholesale onto Lore.

Reliquary/Phylactery continue to own their existing purpose-built representations for:

- interaction transcript and source evidence;
- Episodes and Fragments;
- Memories and Memory revisions;
- provenance and semantic lineage;
- Dream and Graph authority;
- Echo;
- vectors, compatibility profiles, and retrieval indexes;
- owner/scope state; and
- operational durability, recovery, and other database-local state.

The opaque `ObjectRef` seam remains valid and should continue insulating semantic owners from physical offsets. Its target is an implementation-neutral Reliquary storage boundary, not an obligation to replace the current physical store with Lore.

### Project repositories own project-file history

Lore or Git owns:

- project file content and tree state;
- historical project snapshots;
- project-file ancestry;
- rename/move/delete history;
- branches and repository-level revisions;
- binary/project-file diff and merge mechanics; and
- exact historical retrieval of project files.

Reliquary may store durable references to repository file states for transcript/provenance purposes, but it does not duplicate repository history internally.

A repository-backed file reference must be able to identify the exact historical file state used as context. At minimum it requires a repository revision/commit reference plus sufficient file identity/path/content information to resolve the bytes deterministically.

### Uploads become project files

Uploaded files are context-ingestion events, not a separate permanent file-storage class.

For a managed Lore project:

1. hash the uploaded bytes;
2. search eligible tracked files in the current project state for an exact-content match;
3. if a match exists, reuse the first deterministic eligible match and create no duplicate `uploads/` entry;
4. otherwise materialize the file under the project's `uploads/` folder and checkpoint it through Lore; and
5. store the transcript/provenance reference to that exact repository file state.

Dedupe eligibility may exclude ignored/untracked/generated/temp locations and a small set of obviously disposable filename patterns. Content equality remains exact; the filtering only chooses whether an existing identical file is an appropriate historical reference.

After ingestion, an uploaded file is a normal project file. Later moves, edits, renames, or deletion do not change what the historical transcript reference means.

For Git projects, exact tracked matches can be referenced directly at the current commit. New uploaded files may be materialized under `uploads/`, but Warlock must not silently commit them onto the user's active branch. Exact historical capture should use a Warlock-owned Git snapshot/ref or equivalent Git-native object/reference mechanism that leaves normal branch history untouched.

### Project VCS and REL semantic history are separate timelines

Project/file VCS functionality inside REL is frozen for expansion and should be retired or simplified as the universal project-repository layer lands. This applies only to machinery whose primary purpose is duplicating project repository capabilities such as project/blob history, project-file ancestry, project-file merge/replay, and project-filesystem reconstruction.

The REL's own historical timeline remains a product requirement. Reliquary must continue to support the versioning needed to answer what Warlock knew or believed at an earlier point, reconstruct a coherent historical semantic cut, and support restore/rollback/branch-after-restore where those capabilities are implemented.

That includes, as applicable:

- REL/global and domain-local semantic version clocks;
- immutable historical semantic records;
- Memory revisions and semantic supersession;
- Graph versions and relationship history;
- Episode/Fragment and transcript history;
- provenance and Echo history;
- vector generations and other versioned semantic/derived state where meaningful;
- database checkpoints, crash recovery, and historical reconstruction; and
- REL/PHY synchronization metadata and semantic reconciliation state.

The project repository and REL history are correlated rather than nested. A coherent REL historical checkpoint may record the associated Lore/Git project revision so Warlock can resolve both "what the agent knew" and "what the project looked like" at the same logical point. A project revision does not replace the REL version, and a REL version does not duplicate the project tree.

### REL/PHY synchronization remains a separate problem

Lore/Git project repositories do not solve synchronization of REL/PHY semantic databases.

Existing whole-REL reconciliation remains current-format compatibility machinery and may be simplified later around the smaller problem of reconciling agent/semantic state. It must not be expanded to duplicate project-file repository history.

## Consequences

Warlock always has a central, revisioned project state to point agents at without requiring non-technical users to understand VCS.

Git projects avoid a redundant hidden Lore repository and preserve developer ownership of repository policy and human-facing history.

Reliquary retains storage optimized for its actual semantic/retrieval workloads rather than inheriting per-object project-VCS overhead for Memories, Graph records, vectors, and similar state. Its own semantic historical timeline remains independent and first-class.

The two histories may be correlated by storing the relevant project repository revision with a REL historical cut/checkpoint, allowing historical semantic state and historical project state to be resolved together without making either one authoritative over the other.

Uploaded files no longer require a separate durable blob universe. They resolve to ordinary repository file states, with historical transcript/provenance references identifying the exact version used as context.

The completed Lore spike remains useful implementation evidence and a source for the managed Lore project-repository layer, but it is no longer a precursor to wholesale REL/PHY substrate migration.

## Rejected alternatives

### Embed Lore beneath all REL/PHY storage

Rejected. It duplicates abstractions already optimized for semantic storage and applies project-VCS machinery to workloads that do not need repository trees, checkout, branching, or merge semantics.

### Keep project/file history primarily inside REL

Rejected. Requiring every Warlock project to have Lore or Git provides project history directly and makes a parallel REL project-VCS layer redundant.

### Maintain a separate uploaded-file store

Rejected. Uploaded files should become normal project files so project files and context attachments share one file/history authority.

### Create a hidden Lore repository alongside Git

Rejected. A Git project already has a project repository. A second hidden VCS would duplicate file history and create split authority.

## Implementation direction

1. Keep the existing REL/PHY physical and semantic storage architecture, including `ObjectRef`.
2. Preserve the Lore feasibility spike as validated project-repository infrastructure/evidence.
3. Implement automatic Lore initialization/management for projects without Git.
4. Add durable repository revision/file references for transcript and provenance.
5. Implement upload deduplication and `uploads/` materialization for Lore projects.
6. Implement Git-safe attachment snapshots without silent user-branch commits.
7. Preserve and complete REL semantic history/version tracking while freezing and retiring only the portions that duplicate project-file VCS.
8. Add correlation between REL historical cuts/checkpoints and Lore/Git project revisions where historical project context is required.
9. Reassess REL/PHY cloud reconciliation independently as a semantic database synchronization problem.

## Related decisions

- [ADR 0001 — Purpose-built database ownership](0001-purpose-built-database-ownership.md)
- [ADR 0017 — CVA workspace and Warlock host application](0017-cva-workspace-and-warlock-host-application.md)
- [ADR 0019 — Cloud-backed CVA reconciliation](0019-cloud-backed-cva-reconciliation.md)
- [ADR 0020 — Reliquary and Phylactery file kinds](0020-reliquary-and-phylactery-file-kinds.md)
