# Release Compatibility Matrix

This document records which public crate releases were evaluated and published
together, and which prebuilt Chromium runtime bundle they correspond to.

Use this matrix as a maintainer-facing index for cross-release correspondence.
It complements, but does not replace:

- crate-local `CHANGELOG.md` files for crate-specific change history
- `chromium/CHANGELOG.md` for runtime bundle history
- `versioning-and-release-metadata.md` for tag and versioning rules
- `release-process.md` for the packaging and publication flow

## 1. Scope

Each row in the matrix describes one release set that maintainers consider a
coherent public publication unit.

A release set may include:

- one or more published Rust crates
- one published runtime bundle release
- unpublished crates that were not yet part of the public set

This document is intentionally factual. It records published or planned
correspondence between release artifacts; it does not redefine compatibility
policy or replace Cargo dependency constraints.

## 2. Conventions

- `Status` indicates whether the row is only planned or has already been
  published.
- `not released` means the crate did not have a public release in that set.
- `N/A` means the field is not applicable for that row.
- Runtime bundle identifiers should use the canonical tag format defined in
  `versioning-and-release-metadata.md`.
- Release dates should reflect the public GitHub Release publication date when
  available.

## 3. Release Sets

| Release set | Status | Release date | cbf | cbf-chrome | cbf-chrome-sys | cbf-cli | cbf-compositor | Chromium | Runtime bundle tag | Notes |
|---|---|---|---|---|---|---|---|---|---|---|
| 2026-03 alpha.1 | released | 2026-03-16 / 2026-03-17 runtime | 0.1.0-alpha.1 | 0.1.0-alpha.1 | 146.1.0-alpha.1 | 0.1.0-alpha.1 | not released | 146.0.7680.31 | `cbf-chrome-runtime-v146.0.0-alpha.1+chromium-146.0.7680.31-r1` | First public pre-release set for milestone 146 |
| 2026-03 alpha.2 | released | 2026-03-27 | 0.1.0-alpha.2 | 0.1.0-alpha.2 | 146.1.0-alpha.2 | 0.1.0-alpha.2 | 0.1.0-alpha.1 | 146.0.7680.153 | `cbf-chrome-runtime-v146.0.0-alpha.2+chromium-146.0.7680.153-r1` | First public `cbf-compositor` release |

## 4. Interpretation Notes

- `cbf`, `cbf-chrome`, and `cbf-cli` may share the same version in a release
  set, but numerical alignment is not required by policy.
- `cbf-chrome-sys` tracks the Chromium milestone line. For example,
  `146.1.0-alpha.2` belongs to milestone `146`.
- Runtime bundle tags identify packaged Chromium artifacts, not crate releases.
  The left side of the runtime bundle tag is the runtime version, which is
  tracked independently from the crate columns in this matrix.
- A runtime bundle rebuild for the same runtime version and the same Chromium
  version should bump only the runtime release revision (`r1`, `r2`, ...).
- Each row still records crate releases together with the corresponding runtime
  bundle, even though crate versions and runtime version evolve independently.
- `cbf-compositor` participates in the documented release set only from its
  first public release onward. Earlier rows should keep it as `not released`
  rather than inferring backfilled compatibility.

## 5. Maintainer Update Rules

When adding a new row:

1. Copy the previous row and update only the fields that changed.
2. Use the published Git tag and GitHub Release title as the source of truth.
3. Keep notes short and limited to release-set-level facts.
4. Put crate-specific change details in each crate's `CHANGELOG.md`.
5. Put runtime packaging details in `chromium/CHANGELOG.md` and the runtime
   release notes.
