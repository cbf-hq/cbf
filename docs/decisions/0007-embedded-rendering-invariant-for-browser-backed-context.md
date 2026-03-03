# ADR 0007: Embedded Rendering Invariant for Browser-Backed Chrome Context

- Status: Proposed
- Date: 2026-03-03

## Context

ADR 0006 established that CBF should use browser-backed ownership for the
chrome-only runtime so Chromium-facing async work resolves by stable logical
IDs and re-resolves `WebContents` at execution time.

Issue #43 implemented that direction by creating a Chromium `Browser` and
placing each CBF page into a browser-owned tab. That satisfied
`BrowserWindowInterface` / `tabs::TabInterface` for Chrome WebUI flows, but the
initial implementation used the default visible Chromium browser window path.

That visible-window path regressed CBF's embedded rendering model on macOS:
CBF pages stopped producing exported surfaces for host embedding, so
`SurfaceHandleUpdated` no longer reached host applications such as
`examples/simpleapp`. The page still existed and received input/title updates,
but the host-controlled rendering path stopped working.

CBF's architecture requires host-controlled rendering via exported surfaces.
The project is a reusable browser backend, not a Chrome shell UI. For the
embedded runtime, showing a normal Chromium browser window is therefore an
invalid runtime behavior even if it satisfies Chromium's Browser/Tab
assumptions.

## Decision

CBF keeps the browser-backed ownership model from ADR 0006, but embedded
rendering is a non-negotiable runtime invariant.

"Browser-backed" means CBF must provide Chromium's Browser/Tab semantic context
required by Chrome WebUI and related handlers. It does not require using a
normal visible Chromium browser window.

To satisfy both constraints, CBF may provide a private non-visible browser
window shim inside Chromium-side CBF code. That shim may be supplied to
Chromium `Browser` creation so that:

- `BrowserWindowInterface` and `tabs::TabInterface` remain available
- browser-backed ownership and lifecycle remain intact
- embedded surface export remains the active rendering path
- visible Chromium browser windows are not created in embedded runtime

Visible Chromium browser windows are forbidden for the embedded CBF runtime.

This ADR extends ADR 0006. It does not replace it.

## Consequences

### Positive

- CBF preserves the Browser/Tab context required for Chrome WebUI, print
  preview, PDF, extension, and future Chrome-only integrations.
- Embedded applications keep receiving exported rendering surfaces.
- The runtime boundary stays aligned with the repository architecture:
  `Application -> cbf -> cbf-chrome -> cbf-chrome-sys -> Chromium`.
- The visible-window regression becomes an explicit architectural violation
  instead of an incidental bug.

### Negative / Trade-offs

- CBF must maintain a Chromium-side browser window shim with a broad
  compatibility surface.
- Some Chromium features may call deeper `BrowserWindow` methods over time,
  which can require incremental shim expansion.
- This keeps an internal dependency on Chromium `BrowserWindow` for now instead
  of eliminating it entirely.

## Alternatives Considered

### A. Keep using the default visible Chromium browser window

- Rejected because it regresses embedded rendering and violates CBF's runtime
  model.

### B. Revert to standalone `WebContents` ownership

- Rejected because it reintroduces the missing `BrowserWindowInterface` /
  `TabInterface` context that Issue #43 was intended to fix.

### C. Remove all internal `BrowserWindow` usage immediately

- Deferred. This may still be possible in a later refactor, but it is a larger
  design change and is not required to fix the current regression safely.

## Notes

- This ADR records the embedded runtime invariant. It does not decide a
  repository-wide replacement strategy for Chromium `BrowserWindow`.
- The non-visible browser window shim is an internal Chromium-side compatibility
  mechanism only. It must not leak into public Rust or FFI APIs.
- Any future Chrome-only feature work must treat exported-surface rendering as a
  compatibility requirement, not a best-effort behavior.

## Follow-ups

- Add a private Chromium-side non-visible browser window shim for CBF.
- Switch `CbfTabManager` to create browser-backed tabs through that shim instead
  of the default visible browser window path.
- Keep the existing browser-backed bootstrap split and ensure DevTools uses the
  same embedded browser creation policy.
- Add regression coverage that verifies CBF pages use a non-visible browser
  window path and continue to install the surface-export observer.
