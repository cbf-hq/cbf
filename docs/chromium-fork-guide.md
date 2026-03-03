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

- For routine local builds, prefer `uv run tool patch build -t <target>` or
  `just patch build -t <target>`. Those flows resolve `depot_tools`
  automatically, so you do not need to modify `PATH` manually.
- `autoninja` is the direct fallback when you need to run Chromium builds
  yourself. It requires `depot_tools` on `PATH`.
- If you keep a local `./depot_tools` checkout at the repository root, run
  `. depot_tools.sh` first to prepend it to `PATH` for the current shell before
  invoking `autoninja` directly.

## 4. Patch and Drift Policy

- Keep CBF-specific changes traceable in `chromium/patches/cbf`.
- Follow the patch queue rules in `chromium/patches/cbf/README.md`:
  exported patch titles use short imperative English, and patch refinements
  should be folded with `fixup` / `squash` instead of appended as new fix
  patches.
- Prefer `uv run tool patch apply` to replay the exported patch queue onto
  `chromium/src`.
- Prefer `uv run tool patch export` after curating the `chromium/src` commit
  stack so `chromium/patches/cbf` stays aligned with the current history.
- When Chromium updates break bridge behavior, update contracts and repin known-good revisions.
- Avoid mixing product-domain behavior into fork patches; keep changes backend-generic for CBF.

## 5. Runtime Notes

- CBF behavior depends on `--enable-features=Cbf`.
- Runtime selection is chrome-only by default. `alloy` may exist as an explicit
  config value but should fail fast until implemented.
- IPC channel must match between browser launch args and Rust-side client.
- In normal library usage, prefer `start_chromium`; manual launch is for debugging only.
