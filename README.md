# CBF (Chromium Browser Framework)

CBF is a Rust-oriented browser backend framework built on Chromium.
It provides a stable, application-agnostic API surface for controlling browsing contexts and receiving browser events, while isolating Chromium/Mojo implementation details behind an FFI boundary.

## Current Status

- Pre-1.0 alpha. Breaking changes are expected.
- CBF is still under active development. Unexpected crashes, incomplete features, and security bugs are still possible.
- You must build your own CBF-patched Chromium runtime for now. Prebuilt runtimes will be provided in the future.
- Currently supported runtime target: macOS on Apple Silicon (`aarch64-apple-darwin`).
- If you discover a security issue, do not open a public issue. See `SECURITY.md`.

## Vision

- Keep CBF independent from any specific product domain.
- Expose browser-generic vocabulary (`Browser`, `BrowsingContext`, `Navigation`, `Dialog`, `Permission`).
- Treat IPC failures as normal conditions (disconnects, timeouts, crashes).
- Improve framework quality so CBF can be reused by other browser projects.

## Who It's For / Not For

CBF is aimed at applications that want to build Chromium-based browser functionality primarily in Rust,
while minimizing how much application code must depend directly on Chromium-side implementation details.

CBF is not primarily a CEF-style customization layer for exposing fine-grained Chromium handler surfaces.
If your main goal is deep per-subsystem customization through Chromium-specific hooks such as JS binding,
lifecycle, process management, or other browser-internal events, that is outside the main scope of CBF.

## Platform Support

| Target  | Linux | macOS | Windows |
| ------- | ----- | ----- | ------- |
| x86_64  | ❌    | ❌    | ❌      |
| aarch64 | ❌    | ✅    | ❌      |

Only macOS on Apple Silicon is supported at this time.
Linux, Windows, and Intel macOS are not yet supported.

## Feature Overview

- **Page Lifecycle & Navigation**
  - ✅ Open / navigate / close webpage
  - ✅ Go back / forward / reload
  - ✅ beforeunload events
- **Surface & Input**
  - ✅ Surface creation & bounds
  - ✅ Mouse / keyboard events
  - ✅ General IME events
- **Content & Interaction**
  - ✅ Get DOM HTML
  - ✅ Drag and drop on webpage
  - ✅ Drag and drop from other apps
  - 🚧 Context menu events
- **Downloads & Print**
  - ✅ General download management
  - 🚧 Print dialog UI
  - ❌ Print preview UI
- **Profile & Extensions**
  - ✅ Open webpage with profile
  - ✅ Get profile / extension list & info
  - ✅ Extension inline UI
  - 🚧 Full extension support
- **Developer Tools & Built-in Pages**
  - 🚧 DevTools UI
  - ✅ `chrome://version`
  - 🚧 `chrome://history`
  - 🚧 `chrome://settings` (disabled by default; development-only opt-in via Chromium extra args and `--cbf-allow-unsafe-settings`)

→ See the [Feature Matrix](https://cbf-hq.github.io/cbf/feature-matrix.html) for full details and notes.

## Documentation

- Book: [CBF Book](https://cbf-hq.github.io/cbf/)
  - Concepts: [Concepts](https://cbf-hq.github.io/cbf/getting-started/concepts.html)
  - User Setup: [User Setup](https://cbf-hq.github.io/cbf/getting-started/user-setup.html)
  - Contributor Setup: [Contributor Setup](https://cbf-hq.github.io/cbf/developer-guide/contributor-setup.html)
  - Feature Matrix: [Feature Matrix](https://cbf-hq.github.io/cbf/feature-matrix.html)
- API Reference
  - `cbf`: [cbf API](https://docs.rs/cbf/latest/cbf/)
  - `cbf-compositor`: [cbf-compositor API](https://docs.rs/cbf-compositor/latest/cbf_compositor/)
  - `cbf-chrome`: [cbf-chrome API](https://docs.rs/cbf-chrome/latest/cbf_chrome/)
  - `cbf-chrome-sys`: [cbf-chrome-sys API](https://docs.rs/cbf-chrome-sys/latest/cbf_chrome_sys/)
- Contributing Guide: [`CONTRIBUTING.md`](CONTRIBUTING.md)
- Security Policy: [`SECURITY.md`](SECURITY.md)

## Support and Reporting

- Questions and general discussion: GitHub Discussions
- Bug reports and feature requests: GitHub Issues
- Security issues: follow [`SECURITY.md`](SECURITY.md)

## Quick Start

CBF currently requires a CBF-patched Chromium runtime and the `cbf_bridge` library.
For setup and first-run instructions, start with:

- User Setup: [User Setup](https://cbf-hq.github.io/cbf/getting-started/user-setup.html)
- Feature Matrix: [Feature Matrix](https://cbf-hq.github.io/cbf/feature-matrix.html)

Prebuilt release artifacts are planned, but are not available yet.
For now, you must build the CBF-patched Chromium runtime and `cbf_bridge` library yourself.

When available, planned release artifacts are:

- GitHub Releases: `cbf-chrome-macos-<git-tag>.tar.gz`
  - Contains `Chromium.app` and `libcbf_bridge.dylib`
- crates.io:
  - `cbf`
  - `cbf-compositor`
  - `cbf-chrome`
  - `cbf-chrome-sys`
  - `cbf-cli`

## Layered Architecture

- `cbf` (browser-generic high-level Rust API)
    - Browser-generic public commands/events and session lifecycle.
- `cbf-compositor` (desktop surface composition layer)
    - Scene-based compositor for arranging browser surfaces inside native host windows.
    - Keeps composition/window attachment concerns separate from backend-specific IPC details.
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
- `cbf-compositor`: depends on `cbf` and optionally `cbf-chrome` for Chrome backend adapters
- `cbf-chrome`: depends on `cbf` and `cbf-chrome-sys`
- `cbf-chrome-sys`: links to Chromium bridge/runtime

## Process Model

CBF uses a process-separated architecture.

The browser-side objects such as Chromium `WebContents` remain owned by the Chromium process.
Rust code does not hold or manage those objects directly. Instead, CBF communicates across the
FFI/IPC boundary using browser commands, browser events, and stable logical IDs such as
`BrowsingContextId`.

This separation is intentional: it keeps Chromium-specific lifetime/threading constraints out of
the public `cbf` API and makes disconnects, crashes, and backend restarts explicit parts of the
failure model.

### Design Influence

Some parts of CBF's architecture were informed by the process-separated, event-driven design seen in
ChatGPT Atlas's OWL architecture. CBF is not a clone of that system, but it shares the same general
preference for keeping browser-process ownership on the backend side and exposing a higher-level,
stable control surface to the application side.

## API Model

CBF centers on two primitives:

- `BrowserCommand`: upstream -> backend operations.
- `BrowserEvent`: backend -> upstream facts/events.

Design principles:

- Event-driven and async by default.
- Command/response boundaries are explicit.
- Process crash/stop events are observable (`BackendReady`, `BackendStopped`, render crash events).
- IPC channel names are part of the public contract and must be non-empty strings.

## Ownership and Lifecycle

- `BrowsingContext` maps to Chromium `content::WebContents` as the core unit.
- Ownership should stay in the Chromium process (e.g., `TabManager`).
- Rust side uses stable logical IDs (`BrowsingContextId`), not raw Chromium pointers/IDs.
- Across async boundaries, avoid passing raw pointers. Use `ID + re-resolve` and weak ownership checks.

## Example: simpleapp

[`simpleapp`](examples/simpleapp/README.md) is a single-window sample app using `winit` + `cbf`.
It currently supports macOS only.

Before running it, follow [User Setup](https://cbf-hq.github.io/cbf/getting-started/user-setup.html) to obtain the CBF Chromium runtime
and configure `CBF_BRIDGE_LIB_DIR`.

Run:

```bash
cargo run -p simpleapp -- \
  --chromium-executable /path/to/Chromium.app/Contents/MacOS/Chromium \
```

You can also set `CBF_CHROMIUM_EXECUTABLE` and omit `--chromium-executable`.

## Licensing

- CBF-authored code: BSD 3-Clause
- Chromium-derived portions of the CBF codebase: Chromium BSD-style license (see `LICENSE.chromium`)
- Chromium and other third-party components: distributed under their respective licenses and notice requirements

See [Licensing](https://cbf-hq.github.io/cbf/developer-guide/licensing.html) for policy details.
