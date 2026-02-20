# CBF Developer Setup Guide

This guide is for contributors who build or modify CBF internals and Chromium bridge components.
If you only consume prebuilt artifacts, use `docs/user-setup-guide.md`.
For fork-specific layout and patch policy, see `docs/chromium-fork-guide.md`.
For the development tooling CLI usage, see `tool/README.md`.

## 1. Prerequisites

- Rust toolchain (stable)
- Git
- Python 3 (required by Chromium tooling)
- `depot_tools`
- Build tools required by Chromium on your OS (clang/ninja/etc.)

## 2. `depot_tools` Policy

Recommended policy:

- Do **not** vendor `depot_tools` in the CBF repository.
- Do **not** manage `depot_tools` as a git submodule.
- Clone `depot_tools` separately and pin a known-good revision in docs/CI.

Example:

```bash
git clone https://chromium.googlesource.com/chromium/tools/depot_tools.git ~/dev/depot_tools
export PATH="$HOME/dev/depot_tools:$PATH"
```

For reproducibility, document and pin a tested `depot_tools` commit SHA in CI.
For instructions on downloading Chromium source code, see
`https://www.chromium.org/developers/how-tos/get-the-code/`.

## 3. Build Chromium Targets

From `chromium/src`:

```bash
autoninja -C out/Default chrome
autoninja -C out/Default cbf_bridge
```

- `chrome`: Chromium browser binary target.
- `cbf_bridge`: Mojo IPC client bridge used by `cbf-chrome-sys`.

## 4. Configure Bridge Library Path

`cbf-chrome-sys` links against `cbf_bridge` from `CBF_BRIDGE_LIB_DIR`:

```bash
export CBF_BRIDGE_LIB_DIR="/path/to/chromium/src/out/Default"
cargo check -p cbf
cargo check -p cbf-chrome
cargo check -p cbf-chrome-sys
```

You can also set this per-project in `.cargo/config.toml`:

```toml
[env]
CBF_BRIDGE_LIB_DIR = "/path/to/cbf_bridge/libdir"
```

This is useful when different projects use different bridge builds, or when you
want to avoid changing global machine environment variables.

## 5. Run and Validate

- Use `start_chromium` in normal development flows.
- For manual launch flags and bridge debugging, see
  `docs/implementation-guide.md` ("Advanced: Manual Chromium Launch").

Validation checklist:

- `cbf` compiles and tests pass.
- `cbf-chrome-sys` compiles and links correctly.
- `cbf_bridge` builds against your target Chromium revision.
- Can launch Chromium and connect from Rust side.
- Crash/disconnect paths produce expected events/errors.

## 6. Troubleshooting

### Build drift between Chromium and bridge

- Symptom: bridge build failures after Chromium update.
- Action: update patches/contracts carefully, then repin tested revisions.
