# ADR 0004: Chrome Feature Wiring on WebContents Path

- Status: Accepted
- Date: 2026-02-23

## Context

ADR 0003 established Chrome Runtime as the default direction while keeping CBF embedded-first.
CBF currently uses multiple Chrome-layer integrations (for example profile services, themes, and DevTools frontend bindings), but still owns page lifecycle via direct `content::WebContents` management.
ADR 0006 later finalized a Browser-backed ownership model for the chrome-only
runtime track, and this ADR is interpreted in that updated ownership context.

Current code shape:

- Chrome dependencies are already present in CBF build/runtime integration:
  - `chromium/src/chrome/browser/cbf/BUILD.gn`
  - `chromium/src/chrome/browser/cbf/cbf_browser_service.cc`
  - `chromium/src/chrome/browser/cbf/mojo/cbf_mojom_converters.cc`
- Page creation/ownership remains WebContents-first:
  - `chromium/src/chrome/browser/cbf/cbf_tab_manager.cc`
- DevTools in CBF currently uses `DevToolsUIBindings + DevToolsAgentHost`:
  - `chromium/src/chrome/browser/cbf/profile/cbf_profile_devtools_controller.cc`
- `DevToolsWindow` exists in Chromium and provides Browser-integrated behavior, but carries Browser/UI coupling:
  - `chromium/src/chrome/browser/devtools/devtools_window.h`

At the same time, CBF wants to support browser capabilities typical in Chromium-based products (extensions, PDF/printing-related behavior) without adopting Chrome shell UI ownership.

## Decision

CBF keeps chrome capability wiring focused on the tab/WebContents execution path,
while ownership/lifecycle semantics follow ADR 0006 (Browser-backed model).
Runtime strategy is chrome-only-first; non-Chrome runtime alternatives (including
Alloy runtime design) are deferred.

Concretely:

- Keep feature wiring on CBF-managed tab/WebContents execution path.
- Treat ownership/lifecycle as Browser-backed (`Browser -> Tab`) per ADR 0006.
- Increase explicit integration points for Chrome-layer capabilities required by CBF goals.
- Keep public CBF API browser-generic and maintain existing layer direction:
  `Application -> cbf -> cbf-chrome -> cbf-chrome-sys -> Chromium`.
- Keep vocabulary boundary explicit:
  - `cbf` uses `BrowsingContextId`.
  - chrome runtime / bridge / FFI use `TabId`.

Non-goals remain explicit:

- CBF will not provide Chrome shell UI surfaces (tab strip, omnibox/URL bar, toolbar, settings shell).
- CBF will not introduce a general Views abstraction layer.
- `chrome://settings` may be exposed only as a limited-scope compatibility
  surface in embedded CBF.
- In MVP, `chrome://settings` is not a recommended primary user-facing settings
  surface.

Allowed UI exception:

- Chrome-provided UI that is functionally necessary for capabilities is allowed (for example autofill suggestion popup, print preview dialog).

DevTools policy for this ADR:

- Keep `DevToolsUIBindings + DevToolsAgentHost` as the primary CBF DevTools path.
- Do not adopt `DevToolsWindow` as primary integration at this time.
- Document and track capability gaps vs `DevToolsWindow` behavior for incremental closure.

## Consequences

### Positive

- Preserves embedded ownership and host-UI control while enabling Chrome-layer capability growth.
- Clarifies boundary between prohibited shell UI and allowed capability UI.
- Provides concrete, capability-based acceptance targets for implementation work.
- Avoids immediate Browser/UI coupling expansion from `DevToolsWindow` adoption.

### Negative / Trade-offs

- Requires explicit integration work for each targeted Chrome capability.
- Some behavior that `DevToolsWindow` provides out of the box remains gap work in CBF.
- Browser-backed lifecycle + tab re-resolution still requires careful race handling.

## Alternatives Considered

### A. Keep content-first implementation and backport chrome features ad hoc

- Not selected due to recurring maintenance cost and weak capability predictability.

### B. Move to Browser-owned model as primary architecture

- Not selected at the time of this ADR; later adopted in ADR 0006 without adopting
  Chrome shell UI ownership.

### C. Adopt `DevToolsWindow` as primary DevTools integration

- Not selected because it introduces stronger Browser/UI coupling than current CBF scope allows.

### D. Keep DevTools path unchanged without documenting capability gaps

- Not selected because implementation completion criteria would remain unclear.

## Notes

- This ADR defines architecture direction and capability targets, not final class names.
- ADR 0002 remains as historical decision context for DevTools architecture rationale.
- Failure model remains unchanged: disconnects/crashes/timeouts are surfaced as non-fatal events/errors.
- ADR 0006 partially supersedes this ADR for ownership/lifecycle semantics.
- Related implementation track: #40, #41, #42, #43, #44, #45.
- When Chrome-owned settings pages are made reachable for compatibility, the
  support contract is "safe to open, safe to degrade", not full UI parity.
- For MVP, unsupported settings actions that depend on top-level Chrome browser
  UI, native windows, or browser/OS dialogs must fail safely instead of
  crashing.
- Preference persistence alone does not imply recommended product support if
  CBF host UI/runtime does not meaningfully consume that preference.
- `chrome://settings` may be used for compatibility and internal validation in
  MVP, but it should not be treated as the preferred end-user settings
  experience.

Current DevTools gap themes (relative to Browser-integrated path) include:

- beforeunload interception behavior parity,
- policy gate behavior parity,
- keyboard/reload/inspected-tab URL handling parity.

## Follow-ups

- Define and implement Chrome feature wiring points for extension support on the CBF WebContents path.
- Define and implement Chrome feature wiring points for PDF/printing support (including print preview dialog path).
- Define and validate required extension browser-control API coverage (including `chrome.tabs.*` and `chrome.windows.*`).
- Define and validate autofill suggestion popup behavior on CBF-managed pages.
- Track DevTools capability gaps as explicit implementation issues while keeping current primary path.
- Keep feature wiring tasks aligned with ownership/lifecycle rollout order from ADR 0006.
