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
  - 🚧 Context menu events
  - ❌ Drag and drop from other apps
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
  - 🚧 DevTools UI (embedded)
  - ✅ `chrome://version`
  - 🚧 `chrome://history` / `chrome://settings`

→ See [docs/feature-matrix.md](docs/feature-matrix.md) for full details and notes.

## Documentation

- Concepts and user guides: `docs/`
- User Setup: `docs/getting-started/user-setup.md`
- Contributor Setup: `docs/developer-guide/contributor-setup.md`
- Feature Matrix: `docs/feature-matrix.md`
- Contributing Guide: `CONTRIBUTING.md`
- Security Policy: `SECURITY.md`

## Quick Start

CBF currently requires a CBF-patched Chromium runtime and the `cbf_bridge` library.
For setup and first-run instructions, start with:

- User Setup: `docs/getting-started/user-setup.md`
- Feature Matrix: `docs/feature-matrix.md`

Planned release artifacts:

- GitHub Releases: `cbf-chrome-macos-<git-tag>.tar.gz`
  - Contains `Chromium.app` and `libcbf_bridge.dylib`
- crates.io:
  - `cbf`
  - `cbf-chrome`
  - `cbf-chrome-sys`
  - `cbf-cli`

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

## Ownership and Lifecycle

- `BrowsingContext` maps to Chromium `content::WebContents` as the core unit.
- Ownership should stay in the Chromium process (e.g., `TabManager`).
- Rust side uses stable logical IDs (`BrowsingContextId`), not raw Chromium pointers/IDs.
- Across async boundaries, avoid passing raw pointers. Use `ID + re-resolve` and weak ownership checks.

## Example: simpleapp

`simpleapp` is a single-window sample app using `winit` + `cbf`.
It currently supports macOS only.

Before running it, follow `docs/getting-started/user-setup.md` to obtain the CBF Chromium runtime
and configure `CBF_BRIDGE_LIB_DIR`.

Run:

```bash
cargo run -p simpleapp -- \
  --chromium-executable /path/to/Chromium.app/Contents/MacOS/Chromium \
```

You can also set `CBF_CHROMIUM_EXECUTABLE` and omit `--chromium-executable`.

## Licensing

- CBF authored code: `BSD 3-Clause`
- Chromium/third-party components: follow each upstream license and notice requirements

See `docs/developer-guide/licensing.md` for policy details.
