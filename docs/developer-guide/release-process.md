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
<git-tag>-macos-aarch64.tar.gz
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

For the MVP OSS browser distribution, the release helper also expects the
architecture and codec policy to be pinned explicitly:

```gn
# Pin the release architecture explicitly for deterministic packaging.
target_cpu = "arm64"  # or "x64"

# Keep the MVP distribution within the Chromium codec/license surface.
proprietary_codecs = false
ffmpeg_branding = "Chromium"
```

Recommended complete `out/Release/args.gn` example for the MVP stage:

```gn
is_debug = false
dcheck_always_on = false
is_component_build = false
is_official_build = true

target_cpu = "arm64"  # or "x64"

proprietary_codecs = false
ffmpeg_branding = "Chromium"
```

Notes:

- `dcheck_always_on = false` is required for release stability; do not reuse the
  development-only `out/Default` setting here.
- `proprietary_codecs = false` keeps the MVP bundle out of the additional
  proprietary codec licensing path.
- `ffmpeg_branding = "Chromium"` makes that policy explicit even though it is
  already the default for non-Chrome-branded builds.

`target_os` is recorded in `SOURCE_INFO.txt` when present. `target_cpu`,
`proprietary_codecs`, and `ffmpeg_branding` are always captured by the release
helper.

## 3. Required tools

The local release flow expects:

- `uv`
- Python 3
- `tar`
- `shasum` or `sha256sum`
- `chromium/src`
- `./depot_tools`

By default, the runtime release bundle identifier is taken from the single git
tag that points at `HEAD`. Use the canonical runtime bundle tag format defined
in `docs/developer-guide/versioning-and-release-metadata.md`.
If `HEAD` is untagged, the release flow fails.

If a maintainer needs to package a different tagged revision or `HEAD` has
multiple release-related tags, pass `--tag <tag>` to select the runtime bundle
tag explicitly.

## 4. Task entrypoints

The root `Taskfile.yml` is the entrypoint for the release flow.

Validate prerequisites:

```bash
task release:check
```

Explicit tag:

```bash
task release:check TAG=cbf-chrome-runtime-v146.0.0-alpha.2+chromium-146.0.7680.153-r1
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
dist/release/<git-tag>-macos-aarch64.tar.gz
```

## 6. Manual upload

Artifact publication remains manual for the MVP stage.

After `task release` completes:

1. Inspect the generated files under `dist/release/<git-tag>/`.
2. Inspect `SOURCE_INFO.txt` and confirm the recorded revisions and GN args.
3. Upload `dist/release/<git-tag>-macos-aarch64.tar.gz` to the matching
   runtime bundle GitHub Release.

## 7. Release legal checklist

Before publishing a binary release, confirm at least the following:

- `out/Release/args.gn` pins `proprietary_codecs = false`.
- `out/Release/args.gn` pins `ffmpeg_branding = "Chromium"`.
- `task release:licenses` was run from the exact source revision used for the
  release archive.
- `THIRD_PARTY_LICENSES.txt` is included in the final archive and was generated
  from the exact Chromium revision being distributed.
- `CBF_LICENSE.txt` is included in the final archive and matches the repository
  `LICENSE`.
- `SOURCE_INFO.txt` records the exact CBF commit, Chromium commit, patch queue
  state, and selected GN args used for the release.
- The release notes or download page state that CBF includes Chromium and other
  third-party components under their own licenses.
- FFmpeg remains within the Chromium/LGPL distribution path for the shipped
  build; do not enable GPL or nonfree FFmpeg options for the MVP release.
- If the release changes media/codec policy, packaging terms, or bundled
  third-party binaries, request legal review before publication.

No GitHub Actions workflow, Jenkins job, or scheduled release automation is part
of this MVP process.
