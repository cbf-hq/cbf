# Versioning and Release Metadata

This guide defines how CBF versions Rust crates and GitHub release artifacts.
It separates crate compatibility from packaged Chromium runtime revisions so
maintainers can publish crate-specific releases and can re-release
`Chromium.app` and `cbf_bridge` without forcing an unrelated crate publish.

## 1. Goals

CBF publishes two kinds of versioned outputs:

- Rust crates on crates.io
- GitHub Releases for crate history and prebuilt runtime bundles

These outputs have different compatibility concerns:

- `cbf` and `cbf-chrome` expose Rust APIs and should follow normal SemVer rules.
- `cbf-chrome-sys` tracks the Chromium generation that defines the low-level FFI
  and wire boundary.
- GitHub Releases serve two purposes:
  - crate-specific release history
  - packaged runtime bundle distribution

## 2. Crate Versioning

### `cbf`

`cbf` uses normal SemVer.

- Major/minor indicate public API compatibility in the browser-generic Rust API.
- Patch indicates compatible bug fixes and documentation-only corrections.
- Before `1.0.0`, maintainers should treat minor bumps as the main signal for
  breaking public API changes.

Examples:

- `0.1.0`
- `0.2.0`
- `0.2.1`

### `cbf-chrome`

`cbf-chrome` also uses normal SemVer.

- Its version tracks the Chrome-specific safe Rust API surface.
- It is versioned independently from `cbf`.
- Its release cadence may be higher than `cbf` because it sits closer to
  `cbf-chrome-sys`, Chromium integration, and backend-specific behavior.

Examples:

- `cbf = 0.3.0`
- `cbf-chrome = 0.7.2`

### `cbf-chrome-sys`

`cbf-chrome-sys` uses a Chromium-generation-oriented version scheme:

```text
<chromium-milestone>.<minor>.<patch>
```

Rules:

- `major`: Chromium milestone, such as `146` or `147`
- `minor`: additive compatible changes to the FFI/wire boundary within that
  Chromium milestone line
- `patch`: compatible fixes only

Examples:

- `146.1.0`
- `146.1.1`
- `146.2.0`
- `147.0.0`

Do not encode the full Chromium revision in SemVer build metadata as the
primary compatibility signal. For example,
`146.1.0+chromium-146.0.7632.160` is acceptable as descriptive metadata, but
Cargo dependency resolution does not treat build metadata as part of version
precedence.

## 3. Compatibility Model

The expected compatibility split is:

- `cbf` and `cbf-chrome` define the Rust-side API contract.
- `cbf-chrome-sys` defines the Rust-to-bridge ABI contract for a Chromium
  milestone line.
- crate releases identify changes to one Rust crate at a time.
- runtime bundle releases identify the concrete prebuilt runtime bundle that
  contains:
  - `Chromium.app`
  - `libcbf_bridge.dylib`

As a result:

- A crate may receive a GitHub Release even when no new runtime bundle is
  published.
- `cbf` and `cbf-chrome` do not need matching version numbers.
- Compatibility between `cbf` and `cbf-chrome` should be expressed through
  Cargo dependency requirements and release notes, not version-number alignment.
- A `Chromium.app` or `cbf_bridge` rebuild may require a new GitHub Release
  without requiring a new `cbf-chrome-sys` crate release.
- A new `cbf-chrome-sys` release is required when the ABI/wire boundary changes
  in a way that should be represented to Cargo users.

## 4. GitHub Release Types

CBF uses two GitHub Release types:

- crate releases
- runtime bundle releases

These release types should use different tag namespaces.

### Crate releases

Create a GitHub Release for each public crate publish so maintainers and users
can track crate-specific changes independently.

Canonical tag format:

```text
<crate-name>-v<crate-version>
```

Examples:

```text
cbf-v0.1.0
cbf-chrome-v0.1.0
cbf-chrome-sys-v146.1.0
cbf-compositor-v0.1.0-alpha.1
cbf-cli-v0.1.0
```

Recommended title format:

```text
<crate-name> v<crate-version>
```

Examples:

```text
cbf v0.1.0
cbf-chrome v0.1.0
cbf-chrome-sys v146.1.0
cbf-compositor v0.1.0-alpha.1
cbf-cli v0.1.0
```

Crate release notes should focus on:

- what changed in that crate
- compatibility expectations for downstream users
- links to related runtime bundle releases when relevant

### Runtime bundle releases

GitHub Release tags identify the packaged runtime bundle, not just crate
versions.

The canonical tag format is:

```text
cbf-chrome-runtime-v<cbf-chrome-version>+chromium-<chromium-version>-r<release-revision>
```

Example:

```text
cbf-chrome-runtime-v0.1.0+chromium-146.0.7632.160-r1
```

Field meanings:

- `<cbf-chrome-version>`: the published `cbf-chrome` crate line for this bundle
- `<chromium-version>`: the full Chromium runtime version included in the bundle
- `<release-revision>`: packaging and redistribution revision for the same
  crate/runtime combination, starting at `1`

Release revision increments when maintainers republish artifacts for the same
crate versions and the same Chromium version, for example:

- refreshed `Chromium.app`
- rebuilt `libcbf_bridge.dylib`
- corrected packaging or bundled notices
- regenerated archives due to packaging mistakes

Examples:

- `cbf-chrome-runtime-v0.1.0+chromium-146.0.7632.160-r1`
- `cbf-chrome-runtime-v0.1.0+chromium-146.0.7632.160-r2`
- `cbf-chrome-runtime-v0.1.0+chromium-147.0.7651.5-r1`

## 5. Runtime Bundle Release Titles

Use a human-readable title that mirrors the tag and makes the bundled Chromium
version obvious.

Recommended format:

```text
CBF Chrome v<cbf-chrome-version> for Chromium v<chromium-version> (Release <n>)
```

Examples:

- `CBF Chrome v0.1.0 for Chromium v146.0.7632.160 (Release 1)`
- `CBF Chrome v0.1.0 for Chromium v146.0.7632.160 (Release 2)`

## 6. Release Notes Metadata

Each runtime bundle release should explicitly record:

- `cbf` version
- `cbf-chrome` version
- `cbf-chrome-sys` version
- bundled Chromium version
- release revision (`r1`, `r2`, ...)
- notable changes since the previous release revision or version

The runtime bundle release notes should be derived from:

- `chromium/CHANGELOG.md`
- the crate changelogs when a runtime release depends on crate-side changes that
  should be called out explicitly

Recommended release note structure:

```text
Crates
- cbf: 0.1.0
- cbf-chrome: 0.1.0
- cbf-chrome-sys: 146.1.0

Bundled runtime
- Chromium: 146.0.7632.160
- Release revision: r1

Notes
- Initial public prebuilt release for Chromium milestone 146.
```

Each crate release should explicitly record:

- crate name
- crate version
- notable changes in that crate
- compatibility notes
- related runtime bundle release tags when the crate expects a specific runtime

Each crate release note should be derived from that crate's local
`CHANGELOG.md`.

## 7. Changelog Layout

CBF keeps changelogs close to the release unit they describe.

Recommended changelog files:

- `crates/cbf/CHANGELOG.md`
- `crates/cbf-chrome/CHANGELOG.md`
- `crates/cbf-chrome-sys/CHANGELOG.md`
- `crates/cbf-compositor/CHANGELOG.md`
- `crates/cbf-cli/CHANGELOG.md`
- `chromium/CHANGELOG.md`

Rules:

- Each crate changelog tracks only that crate's public release history.
- `chromium/CHANGELOG.md` tracks runtime bundle history for `Chromium.app`,
  `libcbf_bridge.dylib`, and other Chromium-side packaging changes that affect
  prebuilt runtime releases.
- GitHub Release notes should summarize and link back to the corresponding
  changelog entry instead of becoming the only source of change history.
- Maintainers may include only the relevant excerpt in GitHub Release notes, but
  the full changelog entry should remain in the repository.

## 8. Maintainer Guidance

- Treat crate versions and GitHub Release tags as separate identifiers with
  different responsibilities.
- Create crate releases for public crate publishes so each crate has its own
  visible release history.
- Maintain one changelog per public crate and one runtime changelog under
  `chromium/CHANGELOG.md`.
- Do not try to keep `cbf` and `cbf-chrome` numerically aligned when their
  actual release cadence differs.
- Do not force a `cbf-chrome-sys` publish only because a runtime bundle was
  rebuilt.
- Publish a new `cbf-chrome-sys` version when ABI or wire-boundary compatibility
  changes should be visible to Cargo users.
- Keep runtime bundle release notes explicit about both the crate line and the
  packaged Chromium revision.
- Keep the crate tag namespace and runtime tag namespace distinct.
