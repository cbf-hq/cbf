# ADR 0005: Host-Mediated Browsing Context Open and Disposition Mapping

- Status: Accepted
- Date: 2026-02-25
- Last Updated: 2026-02-25

## Context

CBF previously had a limited open-request shape (`NewBrowsingContextRequested { target_url, is_popup }`) that was insufficient for robust host policy control.

Since then, CBF has introduced explicit request/response lifecycles for open flows and now has two distinct host-mediated paths:

- `BrowsingContextOpen*` for routing primary content opens to an existing/new browsing context.
- `WindowOpen*` for host-managed window creation/selection when open intent requires window-level handling.

In parallel, CBF also introduced host-mediated auxiliary window control (`AuxiliaryWindow*`) for dialog-like secondary UI (for example extension install prompt). It is important to keep this separate from primary browsing-context/window open lifecycle.

CBF must remain browser-generic at the public API layer and avoid Chromium-specific UI vocabulary leakage.
Some CBF integrators do not implement a tab-strip UI, so API wording must not assume "tabs" as the primary host model.

## Decision

CBF defines host-mediated open handling as browser-generic, explicitly separating three concerns:

- `BrowsingContextOpen*`: primary content routing lifecycle.
- `WindowOpen*`: host-managed window lifecycle for window-level opens.
- `AuxiliaryWindow*`: secondary/dialog-like surfaces only.

Concretely:

- Public `cbf` API avoids `tab` terminology and uses browser-generic `browsing context` and `window` terminology.
- Normal page open requests do not flow through `AuxiliaryWindow*`.
- Open intent is represented as browser-generic hints in `cbf` (`BrowsingContextOpenHint`), where intent is advisory and host policy remains authoritative.
- `cbf-chrome` maps Chromium-side open/disposition semantics into CBF generic hints and routes to either `BrowsingContextOpen*` or `WindowOpen*` as appropriate.

Normative mapping policy:

- The mapping used in design discussion is illustrative, not final.
- During implementation, `cbf-chrome` should be grounded in Chromium-defined disposition vocabulary/behavior and keep mapping logic traceable in code.
- Major open paths should be covered by tests as implementation stabilizes.

## Current API Snapshot

As of this update, the public direction in this ADR is realized by:

- `BrowserEvent::BrowsingContextOpenRequested / BrowsingContextOpenResolved`
- `BrowserEvent::WindowOpenRequested / WindowOpenResolved / WindowOpened / WindowClosed`
- `BrowsingContextEvent::AuxiliaryWindowOpenRequested / AuxiliaryWindowResolved / AuxiliaryWindowOpened / AuxiliaryWindowClosed`

This ADR governs architectural boundaries; exact mapping details in `cbf-chrome` may evolve while preserving browser-generic semantics in `cbf`.

## Consequences

### Positive

- Keeps public CBF API browser-generic and host-UI-agnostic.
- Clarifies responsibility boundaries between primary open flow and auxiliary dialog flow.
- Enables host integrators to enforce allow/deny/reroute policy with explicit lifecycle signals.
- Reduces accidental Chromium UI coupling in CBF public surface.

### Negative / Trade-offs

- Requires additive API work across `cbf`, `cbf-chrome`, and bridge/FFI layers.
- Requires ongoing mapping maintenance as Chromium behavior evolves.
- Increases implementation complexity by splitting browsing-context and window mediation paths.

## Alternatives Considered

### A. Keep legacy `NewBrowsingContextRequested { target_url, is_popup }`

- Not selected because it lacks explicit lifecycle control and open-intent fidelity.

### B. Route normal opens through `AuxiliaryWindow*`

- Not selected because auxiliary windows represent dialog-like secondary surfaces, not primary browsing-context/window lifecycle.

### C. Expose Chromium `WindowOpenDisposition` directly in `cbf`

- Not selected because it leaks Chromium-specific semantics into the browser-generic API layer.

### D. Introduce `tab`-centric API in `cbf`

- Not selected because not all CBF integrators use tab-strip UI, and this would overconstrain host design.

## Follow-ups

- [x] Define concrete `cbf` event/command types for host-mediated open request/response lifecycle.
- [x] Introduce separate `AuxiliaryWindow*` lifecycle for dialog-like surfaces.
- [x] Introduce host-mediated `WindowOpen*` lifecycle for window-level opens.
- [ ] Document Chromium disposition survey result and explicit mapping rationale in code/docs.
- [ ] Add tests for representative open paths (foreground/background context, new window, popup, deny/abort).
- [ ] Publish host integration guide for routing policy across existing context/new context/new window.
