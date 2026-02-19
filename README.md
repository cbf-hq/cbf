# CBF (Chromium Browser Framework)

CBF is a Rust-oriented browser backend framework built on Chromium.
It provides a stable, application-agnostic API surface for controlling browsing contexts and receiving browser events, while isolating Chromium/Mojo implementation details behind an FFI boundary.

## Documentation Index

- Setup Guide (Overview): `docs/setup-guide.md` (choose user or contributor path)
- User Setup Guide: `docs/user-setup-guide.md` (use prebuilt CBF/cbf_bridge artifacts)
- Developer Setup Guide: `docs/developer-setup-guide.md` (build Chromium/cbf_bridge locally)
- Chromium Fork Guide: `docs/chromium-fork-guide.md` (fork-specific layout and patch policy)
- Architecture: `docs/architecture.md` (design intent and layering model)
- Implementation Guide: `docs/implementation-guide.md` (IPC/threading/FFI rules for contributors)
- Licensing Guide: `docs/licensing.md` (BSD-3 and third-party notice policy)
- Contributing Guide: `CONTRIBUTING.md` (contribution process and commit conventions)

## Platform Support

| Target | Linux | macOS | Windows |
| --- | --- | --- | --- |
| x86_64 | ✖ | ✔ | ✖ |
| aarch64 | ✖ | ✔ | ✖ |

## Vision

- Keep CBF independent from any specific product domain.
- Expose browser-generic vocabulary (`Browser`, `BrowsingContext`, `Navigation`, `Dialog`, `Permission`).
- Treat IPC failures as normal conditions (disconnects, timeouts, crashes).
- Improve framework quality so CBF can be reused by other browser projects.

## Layered Architecture

- `cbf` (high-level Rust API)
    - Public commands/events and session lifecycle.
    - Converts low-level FFI events into stable Rust API types.
- `cbf-sys` (low-level Rust FFI boundary)
    - C ABI types/functions and linkage contract.
    - No high-level browser domain logic.
- Chromium fork (and `cbf_bridge` target)
    - Mojo-based IPC implementation and WebContents-side integration.
    - Chromium-specific threading/lifetime constraints.

Dependency direction:

`Application -> cbf -> cbf-sys -> Chromium process`

## API Model

CBF centers on two primitives:

- `BrowserCommand`: upstream -> backend operations.
- `BrowserEvent`: backend -> upstream facts/events.

Design principles:

- Event-driven and async by default.
- Command/response boundaries are explicit.
- Process crash/stop events are observable (`BackendReady`, `BackendStopped`, render crash events).

## Current MVP Surface (from existing implementation/docs)

Implemented or already modeled at API level:

- Open/manage browsing contexts (WebContents-based model)
- Mouse/keyboard input handling pipeline (in progress at boundary level)
- Window open requests (`NewBrowsingContextRequested`)
- JavaScript dialog requests (alert/confirm/prompt, beforeunload)
- Title updates
- Cursor updates
- Favicon updates
- Loading/navigation state skeleton
- Backend start/stop events
- Render process crash notifications

Still to expand:

- URL update event granularity
- Loading start/finish granularity
- Resubmission dialog support
- Profile/data-dir override ergonomics
- Permission/download/file-picker surface completion

## Ownership and Lifecycle

- `BrowsingContext` maps to Chromium `content::WebContents` as the core unit.
- Ownership should stay in the Chromium process (e.g., `TabManager`).
- Rust side uses stable logical IDs (`BrowsingContextId`), not raw Chromium pointers/IDs.
- Across async boundaries, avoid passing raw pointers. Use `ID + re-resolve` and weak ownership checks.

## Example: simpleapp

`simpleapp` is a single-window sample app using `winit` + `cbf`.

Run:

```bash
cargo run -p cbf --example simpleapp --features chromium-backend -- \
  --chromium-executable /path/to/Chromium.app/Contents/MacOS/Chromium \
```

You can also set `CBF_CHROMIUM_EXECUTABLE` and omit `--chromium-executable`.

## Licensing

- CBF authored code: `BSD 3-Clause`
- Chromium/third-party components: follow each upstream license and notice requirements

See `docs/licensing.md` for policy details.
