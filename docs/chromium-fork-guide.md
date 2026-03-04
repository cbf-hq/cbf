# Chromium Fork Guide

This document describes Chromium-fork specifics for CBF contributors.
For setup steps, see `docs/developer-setup-guide.md`.

## 1. Scope

CBF relies on a Chromium fork that includes CBF-specific integration.
Do not assume stock upstream Chromium has equivalent behavior.

## 2. Relevant Source Layout

- CBF-related Chromium code: `chromium/src/chrome/browser/cbf/`
- Bridge implementation for `cbf-chrome-sys`: `chromium/src/chrome/browser/cbf/bridge`
- CBF patch set: `chromium/patches/cbf`

## 3. Build Targets

From `chromium/src`:

```bash
# Build chrome browser binary with CBF feature path.
autoninja -C out/Default chrome

# Build cbf_bridge shared library for cbf-chrome-sys.
autoninja -C out/Default cbf_bridge
```

- For routine local builds, prefer `uv run tool build -t <target>` or
  `just build -t <target>`. Those flows resolve `depot_tools`
  automatically, so you do not need to modify `PATH` manually.
- `autoninja` is the direct fallback when you need to run Chromium builds
  yourself. It requires `depot_tools` on `PATH`.
- If you keep a local `./depot_tools` checkout at the repository root, run
  `. depot_tools.sh` first to prepend it to `PATH` for the current shell before
  invoking `autoninja` directly.

Common test targets:

```bash
# Build CBF-specific Chromium-side tests.
autoninja -C out/Default cbf_tests

# Build Chromium browser test binary.
autoninja -C out/Default browser_tests

# Build Chromium unit test binary.
autoninja -C out/Default unit_tests
```

- For routine local test builds, prefer `uv run tool build -t cbf_tests`,
  `uv run tool build -t browser_tests`, or
  `uv run tool build -t unit_tests`.

## 4. Patch and Drift Policy

- Keep CBF-specific changes traceable in `chromium/patches/cbf`.
- Follow the patch queue rules in `chromium/patches/cbf/README.md`:
  exported patch titles use short imperative English, and patch refinements
  should be folded with `fixup` / `squash` instead of appended as new fix
  patches.
- Prefer `uv run tool apply` to replay the exported patch queue onto
  `chromium/src`.
- Prefer `uv run tool export` after curating the `chromium/src` commit
  stack so `chromium/patches/cbf` stays aligned with the current history.
- Use `uv run tool commit ...` for `chromium/src` patch-stack commits so the
  patch workflow stays consistent with the current tool entrypoints.
- When Chromium updates break bridge behavior, update contracts and repin known-good revisions.
- Avoid mixing product-domain behavior into fork patches; keep changes backend-generic for CBF.

## 5. Runtime Notes

- CBF behavior depends on `--enable-features=Cbf`.
- Runtime selection is chrome-only by default. `alloy` may exist as an explicit
  config value but should fail fast until implemented.
- IPC bootstrap uses inherited Mojo endpoint (`--cbf-ipc-handle=<endpoint>`) and a
  per-session token (`--cbf-session-token=<hex>`). Both are injected by `start_chromium`; do
  not set them manually unless you are also managing the fd pair and token from the Rust side.
- In normal library usage, prefer `start_chromium`; manual launch is for debugging only.
