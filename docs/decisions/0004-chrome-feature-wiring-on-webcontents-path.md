# ADR 0004: Chrome Feature Wiring on WebContents Path

- Status: Accepted
- Date: 2026-02-23

## Context

ADR 0003 established Chrome Runtime as the default direction while keeping CBF embedded-first.
CBF currently uses multiple Chrome-layer integrations (for example profile services, themes, and DevTools frontend bindings), but still owns page lifecycle via direct `content::WebContents` management.

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

CBF will keep Browser-less embedded ownership with WebContents as the primary page boundary, and strengthen Chrome feature wiring on that path.

Concretely:

- Keep WebContents ownership/lifecycle in CBF Chromium integration.
- Increase explicit integration points for Chrome-layer capabilities required by CBF goals.
- Keep public CBF API browser-generic and maintain existing layer direction:
  `Application -> cbf -> cbf-chrome -> cbf-chrome-sys -> Chromium`.

Non-goals remain explicit:

- CBF will not provide Chrome shell UI surfaces (tab strip, omnibox/URL bar, toolbar, settings shell).
- CBF will not introduce a general Views abstraction layer.

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
- Continued use of WebContents-first ownership requires careful lifecycle/race handling.

## Alternatives Considered

### A. Keep content-first implementation and backport chrome features ad hoc

- Not selected due to recurring maintenance cost and weak capability predictability.

### B. Move to Browser-owned model as primary architecture

- Not selected due to conflict with embedded-first ownership and host UI responsibility.

### C. Adopt `DevToolsWindow` as primary DevTools integration

- Not selected because it introduces stronger Browser/UI coupling than current CBF scope allows.

### D. Keep DevTools path unchanged without documenting capability gaps

- Not selected because implementation completion criteria would remain unclear.

## Notes

- This ADR defines architecture direction and capability targets, not final class names.
- ADR 0002 remains as historical decision context for DevTools architecture rationale.
- Failure model remains unchanged: disconnects/crashes/timeouts are surfaced as non-fatal events/errors.

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
