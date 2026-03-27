# User Setup

This chapter is for application developers using prebuilt CBF and `cbf_bridge` artifacts.
If you will modify Chromium-side code or bridge internals, use [Contributor Setup](../developer-guide/contributor-setup.md).

## 1. Prerequisites

- Stable Rust toolchain (install from https://rustup.rs/)

## 2. Prepare the CBF runtime artifacts

The CBF Chromium fork and `cbf_bridge` library are distributed together in the same runtime artifact on GitHub Releases.
Most users should download the prebuilt runtime package from the `cbf-chrome-runtime` tags.

Latest runtime release: [cbf-chrome-runtime-v146.0.0-alpha.2+chromium-146.0.7680.153-r1](https://github.com/cbf-hq/cbf/releases/tag/cbf-chrome-runtime-v146.0.0-alpha.2%2Bchromium-146.0.7680.153-r1)

If you need a custom build instead, you can also build both artifacts locally from the Chromium tree; see [Contributor Setup](../developer-guide/contributor-setup.md).

### Chromium fork binary

The CBF Chromium fork is a patched Chromium build required as the browser backend.
**Do not use stock Chromium** — it does not include the CBF bridge patches.

Platform-specific artifact names and `executable_path` values:

| Platform | Artifact              | `executable_path`                      |
| -------- | --------------------- | -------------------------------------- |
| macOS    | `Chromium.app`        | `Chromium.app/Contents/MacOS/Chromium` |
| Linux    | currently unsupported | ...                                    |
| Windows  | currently unsupported | ...                                    |

On macOS, `executable_path` must point to the binary inside the `.app` bundle, not the bundle itself.

### Bridge library path

Platform-specific library names:

| Platform | Library                                    |
| -------- | ------------------------------------------ |
| macOS    | `libcbf_bridge.dylib`                      |
| Linux    | `libcbf_bridge.so` (currently unsupported) |
| Windows  | `cbf_bridge.dll` (currently unsupported)   |

Set `CBF_BRIDGE_LIB_DIR` to the directory containing the bridge library. `cbf-chrome-sys` uses this as a runtime lookup hint:

```bash
export CBF_BRIDGE_LIB_DIR="/path/to/cbf_bridge/libdir"
```

You can also pin it per project in `.cargo/config.toml` instead of setting an environment variable:

```toml
[env]
CBF_BRIDGE_LIB_DIR = "/path/to/cbf_bridge/libdir"
```

Then verify crates compile:

```bash
cargo check -p cbf
cargo check -p cbf-chrome
cargo check -p cbf-chrome-sys
```

## 3. Launch Chromium through CBF

### Try it with simpleapp first

If you want to verify your setup before writing your own app,
[`examples/simpleapp`](https://github.com/cbf-hq/cbf/tree/main/examples/simpleapp)
is a working single-window reference app
that demonstrates the full integration. It also serves as a concrete example of the patterns
described below.

```bash
# Set the Chromium-fork executable path, then run:
CBF_CHROMIUM_EXECUTABLE=/path/to/Chromium.app/Contents/MacOS/Chromium \
  cargo run -p simpleapp
```

> **Note:** `simpleapp` currently supports macOS only.

### What `start_chromium` does (and does not do)

`start_chromium` spawns the Chromium process and establishes the IPC bridge.
It returns a session handle, an event stream, and a process handle — **no window is created**.

```rust
use std::path::PathBuf;
use cbf_chrome::backend::ChromiumBackendOptions;
use cbf_chrome::process::{
    ChromiumProcessOptions, ChromiumRuntime, RuntimeSelection, StartChromiumOptions,
    start_chromium,
};

let (session, events, process) = start_chromium(
    StartChromiumOptions {
        process: ChromiumProcessOptions {
            runtime: RuntimeSelection::Chrome,
            executable_path: PathBuf::from("/path/to/chromium"),
            user_data_dir: Some("./.cbf-user-data".to_owned()),
            ..Default::default()
        },
        backend: ChromiumBackendOptions::new(),
    },
)?;

let runtime = ChromiumRuntime::new(session, events, process);
runtime.install_signal_handlers()?;
```

Operational notes:

- Prefer explicit `user_data_dir` to avoid profile conflicts.
- `executable_path` should point to the CBF Chromium-fork binary obtained in §2.
- `start_chromium` remains the core tuple API; `ChromiumRuntime` is the opt-in lifecycle wrapper
  for signal forwarding and best-effort shutdown hardening.

### Windows and surface attachment

Displaying browser content requires two additional steps that the application is responsible for:

1. **Create a native window** using a windowing library such as `winit`.
2. **Attach a surface** — after `BackendReady` and browsing context creation, you will receive
   a `SurfaceHandleUpdated` event containing a platform-specific surface handle. Attach this
   handle to your host window using `cbf-compositor` or your own platform-native integration.
   If you later hide a browsing context and show it again, macOS may recreate the underlying
   compositor surface. In that case CBF can emit another `SurfaceHandleUpdated` event with a new
   handle, and the host should reattach or rebind the surface using that latest handle.

`simpleapp` implements this full cycle using `winit` and `cbf-compositor`.
See `examples/simpleapp/src/` for the platform-specific surface attachment and event loop wiring.

## 4. Validate behavior

- `start_chromium` launches and connects successfully.
- Baseline lifecycle events (`BackendReady`, `BackendStopped`) are observable.
- Crash/disconnect paths surface as events/errors, not silent hangs.

## 5. macOS app bundling

`cbf-cli` can package an app binary with Chromium + bridge into a `.app` bundle.

### Configure via Cargo metadata

Bundle settings are read from `[package.metadata.cbf.macos-bundle]` in your `Cargo.toml`.
This is the recommended way to declare app identity, as it is committed alongside your project:

```toml
[package.metadata.cbf.macos-bundle]
app-name = "My App"
bundle-identifier = "com.example.myapp"
icon = "assets/icon.icns"          # relative to Cargo.toml
runtime-app-name = "My App Engine"
runtime-bundle-identifier = "com.example.myapp.runtime"
runtime-icon = "assets/runtime.icns" # defaults to `icon` when omitted
category = "public.app-category.developer-tools"
minimum-system-version = "13.0"
```

`bundle-version` is taken automatically from `[package] version`.
`cbf-cli` uses `runtime` terminology in config and implementation, while the
default user-visible bundled runtime branding still uses `Engine`.

### Run the bundler

The following three inputs are always required on the command line (or via environment variables):

```bash
cargo run -p cbf-cli -- bundle macos \
  --bin-path /path/to/your/app/binary \
  --chromium-app /path/to/Chromium.app \
  --bridge-lib-dir /path/to/cbf_bridge/libdir
```

Environment variable alternatives:

- `CBF_CHROMIUM_APP` for `--chromium-app`
- `CBF_BRIDGE_LIB_DIR` for `--bridge-lib-dir`

### CLI overrides

CLI arguments take priority over Cargo metadata values when both are present:

| Cargo metadata key          | CLI override flag             |
| --------------------------- | ----------------------------- |
| `app-name`                  | `--app-name`                  |
| `bundle-identifier`         | `--bundle-identifier`         |
| `icon`                      | `--icon`                      |
| `runtime-app-name`          | `--runtime-app-name`          |
| `runtime-bundle-identifier` | `--runtime-bundle-identifier` |
| `runtime-icon`              | `--runtime-icon`              |

`category` and `minimum-system-version` are metadata-only (no CLI equivalent).

Additional CLI-only options: `--out-dir` (default: `dist`), `--codesign-identity`, `--package` (for workspaces).

The bundled runtime is placed under:

```text
<App>.app/Contents/CBF Runtime/<RuntimeAppName>.app
```

Branding changes are applied before signing. If you pass `--codesign-identity`,
`cbf-cli` will sign the final app bundle and validate it with `codesign` and
`spctl`. MVP runtime branding support currently targets release-style Chromium
runtime bundles (`is_component_build = false`).
