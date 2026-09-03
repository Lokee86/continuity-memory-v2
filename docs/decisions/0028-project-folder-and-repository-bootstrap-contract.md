# ADR 0028: Project folder and repository bootstrap contract

Parent index: [Architectural decisions](INDEX.md)

## Status

Accepted — 2026-09-02.

## Context

ADR 0027 separates project-file history from REL/PHY semantic history and requires every Warlock project to have a project repository: Lore by default or Git for the explicit advanced case.

That decision still leaves project creation/opening underspecified. A Project REL needs a stable relationship to an ordinary project folder, a deterministic on-disk home, explicit repository-selection behavior, and bootstrap ordering that does not accidentally place the REL inside the project repository's tracked history.

The REL also retains its own semantic history. Therefore the project repository and REL must be colocated and correlated without one recursively versioning the other.

## Decision

### A Project REL always belongs to a project folder

Creating a Project Reliquary requires selecting or creating an ordinary project folder.

The Project REL is stored beneath that folder:

```text
<project-folder>/
├── .warlock/
│   └── <project-name>.prj.rel
└── project files...
```

The project folder is the user-visible workspace/root. The REL is Warlock-owned semantic state associated with that folder rather than a free-floating Project REL.

The relationship must survive moving or renaming the complete project folder. Absolute paths may be cached operationally but must not be the sole durable identity of the relationship.

### `.warlock/` is never project-repository content

The project repository must exclude `.warlock/` from tracked project state.

This prevents recursive history in which project commits contain the REL while the REL independently records semantic history and project-revision references.

For Lore-managed repositories, Warlock configures the Lore repository to exclude `.warlock/` before the first project commit.

For Git repositories, Warlock should prefer repository-local exclusion such as `.git/info/exclude` rather than silently modifying a tracked `.gitignore`. If a repository arrangement requires another mechanism, Warlock must still ensure `.warlock/` is not unintentionally staged or committed.

### Default creation uses managed Lore

The normal project-creation path exposes the project folder and keeps repository details out of the primary flow.

If `Use Git for repository` is not selected:

1. resolve the selected project folder;
2. detect an existing Lore repository that already owns that folder and reuse it when valid;
3. otherwise initialize Lore for the project folder;
4. configure `.warlock/` exclusion;
5. create `.warlock/` and the Project REL;
6. make the initial Lore commit/checkpoint of the project working tree; and
7. record the resulting Lore repository identity/revision in the REL association/history metadata needed to correlate the two timelines.

The REL itself is not part of the Lore commit.

Managed Lore is the default. After bootstrap, Warlock may create routine project checkpoints automatically according to project-history policy.

### Advanced option: `Use Git for repository`

`Use Git for repository` is an advanced creation option.

When selected, Warlock:

1. detects the Git repository that contains/owns the selected project folder;
2. fails clearly if no usable Git repository can be resolved rather than silently substituting Lore or initializing Git;
3. associates the Project REL with that Git repository and selected project folder;
4. creates `<project-folder>/.warlock/<project-name>.prj.rel`;
5. ensures that `.warlock/` is locally excluded from Git tracking; and
6. records the current Git repository identity/revision state needed for project/REL correlation.

Warlock does **not** create a Git commit merely because a REL was created or opened.

Git repository mutations remain explicit and visible, following the same model as a conventional coding agent operating on a user-managed repository.

### Advanced option: `Manually manage Lore repository`

`Manually manage Lore repository` is an advanced option available only for the Lore path.

It does not remove the Lore repository requirement. Warlock still detects or initializes Lore, excludes `.warlock/`, creates the REL, and establishes the initial project commit so the project begins from a defined repository state.

After bootstrap, automatic routine Lore commits/checkpoints are disabled. Repository-changing operations are surfaced explicitly to the user/agent in the same general manner as advanced Git operations.

`Use Git for repository` and `Manually manage Lore repository` are mutually exclusive because the latter only controls management policy for the Lore repository.

### Existing repository discovery does not create nested repositories

Warlock must resolve repository ownership before initialization.

- Existing Lore is reused on the Lore path when it already owns the selected project folder.
- Existing Git is used only when the advanced Git option is selected.
- A hidden Lore repository is never created alongside an explicitly selected Git repository.
- Warlock must not create a nested Lore repository merely because the selected project folder is a subdirectory of an already-associated Lore repository.

The selected project folder and repository root may differ. The REL remains under the selected project folder's `.warlock/` directory; repository-relative paths/exclusions are resolved against the actual repository root.

### Open/reopen validates the association

Opening an existing Warlock project should normally begin from the project folder and discover its Project REL under `.warlock/`.

Opening a Project REL directly may infer the project folder from its `.warlock/` parent, but Warlock must validate the recorded repository kind/identity against the repository actually present before using repository-backed provenance/history.

If the expected repository is missing, changed, or ambiguous, Warlock must surface the mismatch rather than silently initializing a replacement repository and changing project-history authority.

Moving the complete project folder should remain a normal supported operation because the REL, `.warlock/`, and project files move together while repository association relies on durable repository identity plus relative structure rather than one absolute path.

### Bootstrap is fail-closed and non-destructive

Creating a Project REL must not overwrite an existing Project REL or destroy pre-existing repository/user files.

If the target `.warlock/<project-name>.prj.rel` already exists, creation stops and the existing project should be opened or explicitly resolved instead of silently replacing it.

Bootstrap should behave transactionally at the product level. On failure, Warlock may remove only incomplete artifacts that it created during that bootstrap attempt. It must not delete or rewrite a pre-existing Lore/Git repository, user project files, or unrelated `.warlock/` state.

Repository exclusions and association metadata should be written before Warlock reports project creation as successful. A project is not considered fully created until the REL opens successfully, repository association validates, and the required initial Lore commit has succeeded for Lore mode.

### The two histories are correlated, not synchronized by one clock

Project creation establishes two independent histories:

```text
Lore/Git project history  <---- correlation ---->  REL semantic history
```

Lore/Git owns project files. REL owns transcript, Memory, Graph, provenance, Echo, semantic versions, historical cuts, and runtime state.

A REL checkpoint/historical cut may record the relevant repository revision. This reference does not imply that every REL mutation creates a project commit or that every project commit creates a REL checkpoint.

## Consequences

A newly created Warlock project has one obvious physical home, one project repository, and one Project REL without requiring non-technical users to understand source control.

The default path is deterministic: choose folder, Warlock creates/manages Lore, creates `.warlock/<project>.prj.rel`, creates the first project commit, and correlates the initial states.

Git users explicitly opt into Git ownership and do not receive hidden commits or a second Lore repository.

Advanced Lore users can opt out of automatic checkpoint management without changing the storage/history architecture.

Keeping `.warlock/` out of the project repository preserves the independent REL semantic timeline and avoids recursive or noisy project history.

## Implementation direction

1. Add project-folder selection/association to Project REL creation.
2. Add the two advanced options with mutual-exclusion behavior.
3. Add Lore/Git repository discovery and repository-identity validation.
4. Add `.warlock/` creation and repository-specific exclusion setup.
5. Implement the managed-Lore bootstrap transaction and initial commit.
6. Persist enough repository association/revision metadata for reopen validation and timeline correlation.
7. Implement the Git path without automatic commits.
8. Implement manual-Lore policy as automatic-bootstrap plus explicit post-bootstrap repository management.

## Related decisions

- [ADR 0017 — CVA workspace and Warlock host application](0017-cva-workspace-and-warlock-host-application.md)
- [ADR 0020 — Reliquary and Phylactery file kinds](0020-reliquary-and-phylactery-file-kinds.md)
- [ADR 0022 — Durable typed owner identity](0022-durable-typed-owner-identity.md)
- [ADR 0027 — Warlock project repositories and Reliquary storage boundary](0027-warlock-project-repositories-and-reliquary-storage-boundary.md)
