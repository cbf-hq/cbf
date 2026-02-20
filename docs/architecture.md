# CBF Architecture

Related documents:

- `docs/setup-guide.md`: setup entry point and role-based guide selection
- `docs/user-setup-guide.md`: build and run flow for users
- `docs/developer-setup-guide.md`: local build flow for contributors
- `docs/chromium-fork-guide.md`: fork-specific layout and patch policy
- `docs/implementation-guide.md`: implementation invariants for bridge and FFI contributors

## 1. Purpose

CBF (Chromium Browser Framework) is a browser backend framework for Rust applications.
Its role is to expose a stable, browser-generic API while hiding Chromium- and Mojo-specific implementation details behind a low-level boundary.

CBF is designed to be product-agnostic and reusable across multiple applications.

## 2. Non-Goals

- Encoding product-specific domain terms (workspace, knowledge, pane, etc.) into CBF public types.
- Exposing Chromium internal design directly to API users.
- Treating IPC as a guaranteed-success, synchronous function call.

## 3. Layering and Dependency Direction

Three-layer model:

- `cbf` (browser-generic high-level API)
- `cbf-chrome` (chrome-specific safe API/backend)
- `cbf-chrome-sys` (C ABI/FFI boundary)
- Chromium fork + `cbf_bridge` (Mojo implementation)

Dependency direction:

`Application -> cbf -> cbf-chrome -> cbf-chrome-sys -> Chromium process`

Key rule:

- Chromium/Mojo details must terminate at `cbf-chrome-sys` and conversion code, not leak into the public `cbf` API.
- Detailed implementation constraints are maintained in `docs/implementation-guide.md`.

## 4. API Model

CBF uses command/event primitives:

- `BrowserCommand`: operation requests sent upstream -> backend.
- `BrowserEvent`: facts/notifications sent backend -> upstream.

Expected session shape:

- Connect to backend and obtain a session handle + async event stream.
- Observe lifecycle events (`BackendReady`, `BackendStopped`) explicitly.
- Treat disconnect/crash as first-class outcomes.

## 5. Core Surface (Current Direction)

MVP-capable or already modeled in API/docs:

- browsing context open/manage (WebContents-backed)
- JavaScript dialog requests (alert/confirm/prompt/beforeunload)
- Window-open/new browsing context requests
- Title/cursor/favicon updates
- Navigation/loading state skeleton
- Render process crash notifications
- Backend start/stop notifications

Expected extensions:

- URL update granularity
- Loading start/finish granularity
- Form resubmission dialog surface
- Profile/data-dir and storage override ergonomics
- Broader permission/download/file-picker APIs

## 6. Ownership and Process Boundaries

- `BrowsingContext` is conceptually backed by Chromium `content::WebContents`.
- `WebContents` ownership stays in the Chromium process.
- Rust side tracks stable logical IDs (`BrowsingContextId`), not raw Chromium object identity.

Rationale:

- Avoid crossing process/thread lifetime ownership via IPC.
- Make teardown/restart behavior predictable.

Implementation rules for async/lifetime safety are specified in
`docs/implementation-guide.md`.

## 7. Failure Model

IPC failures are normal states, not exceptional edge cases:

- backend not running
- connection dropped
- timeout or protocol mismatch
- renderer/browser process crash

CBF should surface failures as events/errors that let upstream code choose recovery strategy (retry/recreate/fail-fast).

Test strategy details for bridge-side race and lifetime behavior are maintained in
`docs/implementation-guide.md`.

## 8. Testing Strategy

- `cbf` unit tests: command/event conversion, state transitions, failure mapping.
- Integration checks: minimal end-to-end path (startup, navigation, shutdown) against built bridge/runtime.
- Regression focus: shutdown races and crash/disconnect handling.

## 9. Evolution Policy

- Keep public API vocabulary browser-generic.
- Prefer additive changes over breaking changes.
- Minimize accidental `pub` expansion.
- Version and release CBF independently from application repositories.
