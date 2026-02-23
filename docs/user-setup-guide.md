# CBF User Setup Guide

This guide is for application developers who use prebuilt CBF/cbf_bridge artifacts.
If you plan to build or modify CBF internals, use `docs/developer-setup-guide.md`.

## 1. Prerequisites

- Rust toolchain (stable)
- A Chromium executable compatible with your CBF package
- A directory containing the `cbf_bridge` shared library
  (`libcbf_bridge.so` / `cbf_bridge.dll` / `libcbf_bridge.dylib`)

## 2. Configure Bridge Library Path

Set `CBF_BRIDGE_LIB_DIR` to the directory containing the `cbf_bridge` shared library:

```bash
export CBF_BRIDGE_LIB_DIR="/path/to/cbf_bridge/libdir"
```

You can also set this per-project in `.cargo/config.toml`:

```toml
[env]
CBF_BRIDGE_LIB_DIR = "/path/to/cbf_bridge/libdir"
```

This is useful when different projects use different bridge builds, or when you
want to avoid changing global machine environment variables.

Then verify Rust crates:

```bash
cargo check -p cbf
cargo check -p cbf-chrome
cargo check -p cbf-chrome-sys
```

## 3. Start Chromium from CBF (Default Path)

Use `start_chromium` as the default launch path.
It configures required CBF launch arguments and establishes the backend connection.

Minimal example:

```rust
use std::path::PathBuf;
use cbf_chrome::chromium_backend::ChromiumBackendOptions;
use cbf_chrome::chromium_process::{start_chromium, ChromiumProcessOptions, StartChromiumOptions};

let channel_name = "exampleapp".to_owned();
let (session, events, mut process) = start_chromium(
    StartChromiumOptions {
        process: ChromiumProcessOptions {
            executable_path: PathBuf::from("/path/to/chromium"),
            user_data_dir: Some("./.cbf-user-data".to_owned()),
            enable_logging: Some("stderr".to_owned()),
            log_file: Some("/tmp/chromium_debug.log".to_owned()),
            v: None,
            vmodule: None,
            channel_name: channel_name.clone(),
            extra_args: vec![],
        },
        backend: ChromiumBackendOptions::new(channel_name),
    },
)?;
```

Important:

- Prefer setting `user_data_dir` explicitly unless you have a strong reason not to.
- If `user_data_dir` is `None`, Chromium may use a default profile location.
- Sharing profile data between normal Chromium usage and CBF-driven runs can cause conflicts or data corruption risk (for example, profile/schema version mismatch).
- Set `executable_path` to the prebuilt Chromium-fork binary published for CBF.
- Do not point `executable_path` to a stock upstream Chromium build, because required CBF bridge integration may be missing.

## 4. Suggested Validation Checklist

- `cbf` compiles and tests pass.
- `cbf-chrome-sys` compiles and links correctly.
- Can launch Chromium via `start_chromium`.
- Can connect from Rust side and receive baseline lifecycle events.
- Crash/disconnect paths produce expected events/errors.

## 5. Troubleshooting

### Bridge library cannot be linked

- Symptom: linker cannot find `cbf_bridge`.
- Action: verify `CBF_BRIDGE_LIB_DIR` points to the directory containing the shared library file.

### Chromium fails to spawn

- Symptom: process spawn errors from `start_chromium`.
- Action: verify `ChromiumProcessOptions.executable_path` is correct and executable.

## 6. Bundle as macOS .app (MVP)

You can package an existing CBF app binary using `cbf-cli`:

```bash
cargo run -p cbf-cli -- bundle macos \
  --bin-path /path/to/your/app/binary \
  --chromium-app /path/to/Chromium.app \
  --bridge-lib-dir /path/to/cbf_bridge/libdir
```

Environment variable alternatives:

- `CBF_CHROMIUM_APP` for `--chromium-app`
- `CBF_BRIDGE_LIB_DIR` for `--bridge-lib-dir`

The generated bundle layout includes:

- `Contents/MacOS/<your executable>`
- `Contents/Frameworks/libcbf_bridge.dylib`
- `Contents/Frameworks/Chromium.app`

`cbf-cli` also verifies/adds `@executable_path/../Frameworks` as runtime search path (`rpath`) on the bundled executable.
