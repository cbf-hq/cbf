# CBF (Chromium Browser Framework)

CBF is a Rust-oriented browser backend framework built on Chromium.
It provides a stable, application-agnostic API surface for controlling browsing contexts and receiving browser events, while isolating Chromium/Mojo implementation details behind an FFI boundary.

## Documentation

- Most of documentation is in `docs/` (concepts, usage guides, chromium integration).
- Contributing Guide: `CONTRIBUTING.md` (contribution process and commit conventions)

## Platform Support

| Target  | Linux | macOS | Windows |
| ------- | ----- | ----- | ------- |
| x86_64  | ❌    | ❌    | ❌      |
| aarch64 | ❌    | ✅    | ❌      |

## Vision

- Keep CBF independent from any specific product domain.
- Expose browser-generic vocabulary (`Browser`, `BrowsingContext`, `Navigation`, `Dialog`, `Permission`).
- Treat IPC failures as normal conditions (disconnects, timeouts, crashes).
- Improve framework quality so CBF can be reused by other browser projects.

## Layered Architecture

- `cbf` (browser-generic high-level Rust API)
    - Browser-generic public commands/events and session lifecycle.
- `cbf-chrome` (chrome-specific safe API/backend)
    - Chrome-specific backend implementation and safe extension surface.
- `cbf-chrome-sys` (low-level Rust FFI boundary)
    - Chromium bridge C ABI types/functions and linkage contract.
    - No high-level browser domain logic.
- Chromium fork (and `cbf_bridge` target)
    - Mojo-based IPC implementation and WebContents-side integration.
    - Chromium-specific threading/lifetime constraints.

Dependency direction:

- `cbf`: no internal crate dependency
- `cbf-chrome`: depends on `cbf` and `cbf-chrome-sys`
- `cbf-chrome-sys`: links to Chromium bridge/runtime

## API Model

CBF centers on two primitives:

- `BrowserCommand`: upstream -> backend operations.
- `BrowserEvent`: backend -> upstream facts/events.

Design principles:

- Event-driven and async by default.
- Command/response boundaries are explicit.
- Process crash/stop events are observable (`BackendReady`, `BackendStopped`, render crash events).

## Feature Overview

- **Page Lifecycle & Navigation**
  - ✅ Open / navigate / close webpage
  - ✅ Go back / forward / reload
  - ✅ beforeunload events
- **Surface & Input**
  - ✅ Surface creation & bounds
  - ✅ Mouse / keyboard events
  - 🚧 IME events
- **Content & Interaction**
  - ✅ Get DOM HTML
  - ✅ Drag and drop on webpage
  - 🚧 Context menu events
  - ❌ Drag and drop from other apps
- **Downloads & Print**
  - ✅ Download management
  - 🚧 Print dialog UI
  - ❌ Print preview UI
- **Profile & Extensions**
  - ✅ Open webpage with profile
  - ✅ Get profile / extension list & info
  - ✅ Extension inline UI
- **Developer Tools & Built-in Pages**
  - 🚧 DevTools UI (embedded)
  - ✅ `chrome://version`
  - 🚧 `chrome://history` / `chrome://settings`

→ See [docs/feature-matrix.md](docs/feature-matrix.md) for full details and notes.

## Ownership and Lifecycle

- `BrowsingContext` maps to Chromium `content::WebContents` as the core unit.
- Ownership should stay in the Chromium process (e.g., `TabManager`).
- Rust side uses stable logical IDs (`BrowsingContextId`), not raw Chromium pointers/IDs.
- Across async boundaries, avoid passing raw pointers. Use `ID + re-resolve` and weak ownership checks.

## Example: simpleapp

`simpleapp` is a single-window sample app using `winit` + `cbf`.

Run:

```bash
cargo run -p simpleapp -- \
  --chromium-executable /path/to/Chromium.app/Contents/MacOS/Chromium \
```

You can also set `CBF_CHROMIUM_EXECUTABLE` and omit `--chromium-executable`.

## Licensing

- CBF authored code: `BSD 3-Clause`
- Chromium/third-party components: follow each upstream license and notice requirements

See `docs/licensing.md` for policy details.

## CLI (MVP)

This repository now includes `cbf-cli` (`cbf` binary) for packaging CBF apps on macOS.

Create a macOS app bundle:

```bash
cargo run -p cbf-cli -- bundle macos \
  --bin-path /path/to/your/app/binary \
  --chromium-app /path/to/Chromium.app \
  --bridge-lib-dir /path/to/cbf_bridge/libdir
```

The command creates `<out-dir>/<AppName>.app` (default `dist/`) and bundles:

- app executable (`Contents/MacOS`)
- `libcbf_bridge.dylib` (`Contents/Frameworks`)
- `Chromium.app` (`Contents/Frameworks`)

Optional metadata can be configured in your app `Cargo.toml`:

```toml
[package.metadata.cbf.macos-bundle]
app-name = "MyApp"
bundle-identifier = "com.example.myapp"
icon = "assets/icon.icns"
```
