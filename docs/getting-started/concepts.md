# Concepts

CBF is a reusable browser backend framework for Rust applications.
Its design goal is a stable browser-generic surface with strict containment of Chromium-specific implementation details.

## 1. Layer model

- `cbf`: browser-generic public API
- `cbf-chrome`: chrome-specific safe backend
- `cbf-chrome-sys`: C ABI / FFI boundary and `cbf_bridge` binding target
- Chromium process (Mojo integration and browser runtime)

Dependency direction:

- `Application -> cbf + cbf-chrome`
- `cbf-chrome -> cbf`
- `cbf-chrome -> cbf-chrome-sys <-- IPC --> Chromium process`
- `cbf` must not depend on browser-specific crates.

Browser backends are adapters that depend on `cbf`, so `cbf` can stay focused on browser-generic vocabulary and remain extensible to additional backends (for example, a future Firefox backend).  
Applications may also depend on browser-specific backend crates (such as `cbf-chrome`) for practical integration points, including browser bootstrap, graphics/surface integration, and controlled escape-hatch APIs.

## 2. Interaction and failure semantics

CBF intentionally separates application code from browser internals through a strict asynchronous boundary.
This boundary is a core design principle, not just an implementation detail.

CBF models interactions as:

- `BrowserCommand`: upstream request from application to backend
- `BrowserEvent`: backend fact/notification to upstream

Because communication is asynchronous, every request is treated as potentially fallible by design.
Disconnects, timeouts, crashes, and protocol mismatch are normal outcomes that should be surfaced explicitly.

This model prevents tight coupling to browser-specific behavior and avoids implicit assumptions about browser guarantees.
It also helps CBF users naturally design recovery paths, while limiting direct blast radius from browser-side failures.

Typical failure states:

- backend not running
- disconnect/timeouts/protocol mismatch
- renderer or browser crash

## 3. Ownership and identity boundary

- `BrowsingContext` is mapped to Chrome-layer `Tab` in the current runtime model.
- `Tab` remains WebContents-backed internally, and `WebContents`/`Tab` ownership stays in the Chromium process.
- `cbf` public layer uses `BrowsingContextId`, while chrome-facing layers use `TabId`.
- `BrowsingContextId <-> TabId` is treated as a stable logical mapping at boundary conversion points.
- Rust-side APIs track stable logical IDs, not raw Chromium pointers.

## 4. API evolution policy

- Keep `cbf` vocabulary browser-generic.
- Prefer additive changes over breaking changes.
- Avoid unnecessary public surface expansion.
- Keep lifecycle and failure semantics explicit in API shape.
