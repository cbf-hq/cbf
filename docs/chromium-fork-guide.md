# Chromium Fork Guide

This document describes Chromium-fork specifics for CBF contributors.
For setup steps, see `docs/developer-setup-guide.md`.

## 1. Scope

CBF relies on a Chromium fork that includes CBF-specific integration.
Do not assume stock upstream Chromium has equivalent behavior.

## 2. Relevant Source Layout

- CBF-related Chromium code: `chromium/src/chrome/browser/cbf/`
- Bridge implementation for `cbf-sys`: `chromium/src/chrome/browser/cbf/bridge`
- CBF patch set: `chromium/patches/cbf`

## 3. Build Targets

From `chromium/src`:

```bash
autoninja -C out/Default chrome
autoninja -C out/Default cbf_bridge
```

- `chrome`: browser binary with CBF feature path.
- `cbf_bridge`: shared library used by `cbf-sys`.

## 4. Patch and Drift Policy

- Keep CBF-specific changes traceable in `chromium/patches/cbf`.
- When Chromium updates break bridge behavior, update contracts and repin known-good revisions.
- Avoid mixing product-domain behavior into fork patches; keep changes backend-generic for CBF.

## 5. Runtime Notes

- CBF behavior depends on `--enable-features=Cbf`.
- IPC channel must match between browser launch args and Rust-side client.
- In normal library usage, prefer `start_chromium`; manual launch is for debugging only.
