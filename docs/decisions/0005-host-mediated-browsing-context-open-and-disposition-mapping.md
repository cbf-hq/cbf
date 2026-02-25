# ADR 0005: Host-Mediated Browsing Context Open and Disposition Mapping

- Status: Accepted
- Date: 2026-02-25

## Context

CBF currently emits `NewBrowsingContextRequested { target_url, is_popup }` and leaves new page handling to the host.
This is sufficient for basic popup/new-page notification, but it has two gaps:

- It does not provide an explicit request/response lifecycle for host-controlled allow/deny/routing decisions.
- It does not carry enough open-intent information for robust host policy decisions (for example new foreground context vs new window vs popup).

In parallel, CBF is introducing host-mediated auxiliary window control (`AuxiliaryWindow*`) for dialogs such as extension install prompts.
There is a risk of overloading auxiliary-window concepts with normal browsing-context open flows.

CBF must also remain browser-generic at the public API layer and avoid Chromium-specific UI vocabulary leakage.
Some CBF integrators do not implement a tab-strip UI, so API wording must not assume "tabs" as the primary host model.

## Decision

CBF will define host-mediated browsing-context open as a dedicated browsing-context lifecycle, separate from auxiliary windows.

Concretely:

- Public `cbf` API will avoid `tab` terminology and use browser-generic `browsing context` terminology.
- Normal page open requests will use a request/response lifecycle under browsing-context events/commands (not `AuxiliaryWindow*`).
- Auxiliary windows remain for secondary/dialog-like UI surfaces (for example extension install prompt), not primary browsing-context creation.
- Open intent will be represented in `cbf` as a browser-generic hint enum (for example foreground context, background context, new window, popup), where intent is advisory and host policy remains authoritative.
- `cbf-chrome` will map Chromium open-disposition semantics into that generic hint model.

Normative mapping policy:

- The mapping in design discussions is illustrative, not final.
- During implementation, `cbf-chrome` must first survey Chromium-defined disposition vocabulary and behavior, then adopt a mapping grounded in that source of truth.
- The selected mapping must be documented in code comments and validated by tests for major disposition paths.

## Consequences

### Positive

- Keeps public CBF API browser-generic and host-UI-agnostic.
- Clarifies responsibility boundaries between primary browsing-context flow and auxiliary dialog flow.
- Enables host integrators to enforce policy (allow/deny/reroute) for new context requests with explicit lifecycle signals.
- Reduces accidental Chromium UI coupling in CBF public surface.

### Negative / Trade-offs

- Requires additive API changes across `cbf`, `cbf-chrome`, and bridge/FFI layers.
- Introduces migration work from the current `NewBrowsingContextRequested` shape.
- Requires disposition behavior survey and mapping maintenance as Chromium evolves.

## Alternatives Considered

### A. Keep current `NewBrowsingContextRequested { target_url, is_popup }`

- Not selected because it lacks explicit request/response control and limited open-intent fidelity.

### B. Route normal browsing-context opens through `AuxiliaryWindow*`

- Not selected because auxiliary windows represent dialog-like secondary surfaces, while browsing-context opens are primary content lifecycle.

### C. Expose Chromium `WindowOpenDisposition` directly in `cbf`

- Not selected because it leaks Chromium-specific semantics into the browser-generic API layer.

### D. Introduce `tab`-centric API in `cbf`

- Not selected because not all CBF integrators use tab-strip UI, and this would overconstrain host design.

## Notes

- This ADR decides boundary and API-shape direction, not final Rust type names.
- `cbf-chrome` mapping details are implementation artifacts and may evolve as long as browser-generic semantics remain stable in `cbf`.
- Existing `AuxiliaryWindow*` direction for extension install prompt remains valid and in scope.

## Follow-ups

- Define concrete `cbf` event/command types for browsing-context open request/response lifecycle.
- Add migration plan from `NewBrowsingContextRequested` to the new lifecycle API.
- Survey Chromium disposition definitions and behaviors used by CBF entry points (`OpenURLFromTab`, `AddNewContents`, related flows).
- Implement `cbf-chrome` disposition-to-generic-hint mapping based on that survey.
- Add tests for representative disposition paths (foreground context, background context, new window, popup, deny flow).
- Document host integration guidance for routing open requests to existing/new browsing-context UI.
