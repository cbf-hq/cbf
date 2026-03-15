# Release Process

This guide defines the MVP release flow for pre-built macOS artifacts.
It is intentionally local and manually invoked. Artifact upload is still a
maintainer step outside the repository automation.

For crate versioning and GitHub Release tag/title rules, see
`docs/developer-guide/versioning-and-release-metadata.md`.

## 1. Scope

The MVP release bundle contains these top-level files:

- `Chromium.app`
- `libcbf_bridge.dylib`
- `CBF_LICENSE.txt`
- `THIRD_PARTY_LICENSES.txt`
- `SOURCE_INFO.txt`

The default archive name is:

```text
cbf-chrome-macos-<git-tag>.tar.gz
```

The archive is written under `dist/release/`.

## 2. Release build requirements

Release packaging always reads Chromium outputs from:

```text
chromium/src/out/Release
```

The `out/Release/args.gn` file must exist and must contain these values:

```gn
is_debug = false
dcheck_always_on = false
is_component_build = false
is_official_build = true
```

`target_os` and `target_cpu` are recorded in `SOURCE_INFO.txt` when present.

## 3. Required tools

The local release flow expects:

- `uv`
- Python 3
- `tar`
- `shasum` or `sha256sum`
- `chromium/src`
- `./depot_tools`

The runtime release bundle identifier is taken from the single git tag that
points at `HEAD`. Use the canonical runtime bundle tag format defined in
`docs/developer-guide/versioning-and-release-metadata.md`.
If `HEAD` is untagged, the release flow fails.

## 4. Task entrypoints

The root `Taskfile.yml` is the entrypoint for the release flow.

Validate prerequisites:

```bash
task release:check
```

Build release artifacts:

```bash
task release:build
```

Generate license files:

```bash
task release:licenses
```

Generate source metadata:

```bash
task release:source-info
```

Assemble the release archive:

```bash
task release:package
```

Run the full flow:

```bash
task release
```

If a maintainer intentionally needs to package a dirty tree for investigation,
they can pass `ALLOW_DIRTY=true` to the task invocation. Normal releases should
keep both the repository root and `chromium/src` clean.

## 5. Generated files

The release helper writes generated files under:

```text
dist/release/<git-tag>/
```

Generated outputs:

- `CBF_LICENSE.txt`
- `THIRD_PARTY_LICENSES.txt`
- `SOURCE_INFO.txt`

The final release archive is written to:

```text
dist/release/cbf-chrome-macos-<git-tag>.tar.gz
```

## 6. Manual upload

Artifact publication remains manual for the MVP stage.

After `task release` completes:

1. Inspect the generated files under `dist/release/<git-tag>/`.
2. Inspect `SOURCE_INFO.txt` and confirm the recorded revisions and GN args.
3. Upload `dist/release/cbf-chrome-macos-<git-tag>.tar.gz` to the matching
   runtime bundle GitHub Release.

No GitHub Actions workflow, Jenkins job, or scheduled release automation is part
of this MVP process.
