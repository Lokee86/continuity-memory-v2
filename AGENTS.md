# AGENTS.md

This repository is the clean Reliquary storage/runtime rebuild.

## Required reading

Before architectural or storage work, read:

- `docs/architecture.md`
- `docs/invariants.md`
- `docs/storage-format.md`
- `docs/maintainer-map.md`
- `docs/current-limitations.md`
- `docs/roadmap.md`

For rollback/version-history work also read:

- `docs/version-history-plan.md`
- `docs/decisions/0002-branching-publication-history.md`

## Engineering rules

- Follow the shared `engineering-standards` repository.
- Documentation is part of implementation.
- Keep current behavior, plans, limitations, research, and agent guidance separate.
- One mutable semantic domain has one explicit owner.
- The CVA container owns physical mechanics, not semantic dependencies.
- Do not introduce a generalized store/root/dependency framework because multiple databases need similar-looking mechanics.
- Cross-database references use stable IDs; the referring database owns their semantics.
- A normal database write must not require construction of a CVA-wide semantic publication manifest.
- Derived indexes/checkpoints are acceleration state and must not become semantic authority.
- Prefer concrete seams and small focused Rust modules; split responsibilities before files become difficult to review.
- Reuse previous Reliquary code only after its ownership model is shown to fit this repository.

## Verification

For ordinary Rust changes run:

```text
cargo fmt --check
cargo check
cargo test
```

For documentation changes also run:

```text
python ../engineering-standards/tools/docs_policy/check.py --repo .
```

When the graph-corpus smoke path is affected, run the `archive_roundtrip` example against a prepared graph JSONL fixture.

## Completion report

Every implementation report includes:

```text
Documentation impact:
- Inspected:
- Updated:
- Not affected:
- Compliance check:
- Known documentation gaps:
```

Architectural changes also include:

```text
Architecture impact:
- Standards added or changed:
- Ownership or boundary impact:
- Pitlord enforcement impact:
- Other verification impact:
- Known architectural gaps:
```