# Chromium Integration Rules

This document defines cross-layer design rules for Chromium integration in CBF.
It describes **invariants and policy** shared by:

- `cbf_bridge` (Chromium-side bridge and Mojo wiring)
- `chrome` target integration in Chromium
- `cbf-chrome-sys` boundary crates

For concrete implementation patterns, troubleshooting steps, and command snippets, see
[Chromium Implementation Guide](./chromium-implementation-guide.md).

## 1. Boundary contract

- Browser-generic API surface remains in `cbf`.
- Chromium-specific behavior and details stay in `cbf-chrome-sys` or Chromium-side bridge code.
- Public types in `cbf` do not include Chromium/Mojo internals.
- C ABI and exported C functions remain in `cbf-chrome-sys`.
- Chromium-owned constants are not duplicated into `cbf` public API.

Dependency direction must remain:

- `Application -> cbf + cbf-chrome`
- `cbf-chrome -> cbf`
- `cbf-chrome -> cbf-chrome-sys <-- IPC --> Chromium process`

## 2. Architecture and ownership model

- Treat `cbf` as a logical browser API; avoid embedding product/domain terms in this layer.
- Preserve stable logical IDs as the only stable cross-process identity (`WebPageId`, etc.).
- Resolve platform objects from IDs at execution time instead of caching and dereferencing
  platform pointers across asynchronous boundaries.

## 3. IPC contract

- IPC is asynchronous, non-blocking, and failure-aware.
- IPC APIs are sequence-sensitive; a bound channel must be used only on its bound sequence.
- Disconnects, late callbacks, and incomplete handshakes are regular runtime outcomes,
  not exceptional states.
- Design API and event flow so operations can be retried, deferred, or dropped safely.

## 4. Lifetime safety policy

- Raw platform pointers must not cross async boundaries.
- Avoid capturing owning `this` in callbacks that may outlive owners.
- Use weak ownership mechanisms (`WeakPtr`) where re-entry can happen.
- If an ID re-resolution fails, the correct behavior is safe no-op.
- On shutdown/close races, do not fail-fast on missing state.

## 5. Safety and failure policy

- Crash-unsafe assertions (`CHECK`) are forbidden on expected lifecycle races.
- Expected failure states should be represented as errors/events, enabling upstream recovery
  decisions (retry, restart session, or fail gracefully).
- Duplicate, late, or out-of-order lifecycle operations must be handled idempotently.
- Failure handling should prioritize availability and observability over strict assumptions.

## 6. Chromium process policy

- Default startup path is the Rust `start_chromium` flow.
- Manual launch without bridge attachment is a debug-only scenario and must not be treated as
  normal production behavior.
- Any code path that bypasses the standard startup path should carry explicit operational notes.

## 7. Review expectations

When reviewing Chromium-side integration changes, verify these rule-level outcomes:

- Boundary ownership and dependency direction are preserved.
- Identity and lifetime assumptions are explicit.
- Async paths avoid unsafe object capture and include safe no-op behavior on stale IDs.
- Failure states are surfaced as events/errors and are testable.
- Changes remain reviewable against this document before implementation details are finalized.
