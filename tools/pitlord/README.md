# Reliquary Architecture Enforcement

Reliquary composes the shared Laughing Skull architecture-core Pitlord policy with repository-specific source ownership and static regression rules.

`semantic.json` assigns every authored Rust file under `src/` to exactly one architectural owner. The owners describe current responsibility seams inside the still-single Rust crate; they do not claim that those seams are already separate crates:

- `container`: physical REL/PHY substrate and global ordering;
- `archive`: source history, conversation state, Episodes, attachments, Echo, and repository correlations;
- `memory`: authoritative Memory state;
- `retrieval`: lexical/semantic retrieval plus vector/profile/generation state;
- `semantic-graph`: Graph, Communities, and typed semantic-node traversal;
- `perception`: durable Entity state plus candidate/admission/resolution workflows;
- `chronos`: shared temporal semantics;
- `ego`: Identity/Personality/Anchor/synthesis persistence;
- `runtime-inference`: Insomnia, Dream, providers, and RuntimeHost orchestration;
- `config-security`: machine-local configuration and credential/key state;
- `facade`: public Reliquary/Cva/Phylactery composition, migration, and reconciliation.

The ownership rule is deliberately strict: adding or moving a Rust source file requires assigning its owner in the same change. This makes the current flat `src/` layout auditable before any physical crate/module split.

The root policy also prevents the deleted generic `WorkspaceMetadata` / `workspace_type` authority from returning outside the explicit legacy migration compatibility boundary. It intentionally does not encode aspirational dependency direction yet. Dependency/cycle rules should be added only after Arcana evidence shows a boundary is both real and currently satisfied; current cross-owner knots are audit evidence for later extraction work, not reasons to baseline violations into policy.

Run locally with current evidence:

```text
python scripts/check_architecture.py
```

Refresh Lexicon/Arcana evidence and enforce in one command:

```text
python scripts/check_architecture.py --refresh
```

The checker resolves Pitlord, Lexicon, Arcana, and adapters from environment overrides, PATH, or the normal sibling development repositories. Reliquary follows the same sibling `engineering-standards` source used by its documentation checker, so the shared architecture-core include must be present there. The push/PR architecture workflow builds the tools, prebuilds the Rust Lexicon adapter in release mode, refreshes evidence, and runs the same policy check.

Policy changes that alter architectural ownership must update `docs/architecture.md` and `docs/maintainer-map.md`; consequential ownership changes also require the appropriate ADR. Do not change ownership merely to silence a Pitlord finding.