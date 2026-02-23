# ADR 0002: DevTools Integration Without Chrome Browser Dependency

- Status: Proposed
- Date: 2026-02-21

## Context

CBF issue #12 requires two DevTools display modes:

- Embedded/in-page display
- Separate-window display controlled by CBF host UI

Current context-menu inspect behavior follows Chromium's `RenderViewContextMenu -> DevToolsWindow` path.
In CBF, this path is problematic because CBF manages pages via `content::WebContents` directly (`CbfTabManager`) and does not use Chrome `Browser` tab ownership.
`DevToolsWindow` docking and lifecycle are Browser-dependent and degrade to undocked/separate behavior in this environment.

Relevant sources:

- `chromium/src/chrome/browser/cbf/cbf_tab_manager.cc`
- `chromium/src/chrome/browser/cbf/cbf_profile_service.cc`
- `chromium/src/chrome/browser/devtools/devtools_window.cc`
- `chromium/src/content/shell/browser/shell_devtools_frontend.cc`
- `chromium/src/content/shell/browser/shell_devtools_bindings.cc`

## Decision

CBF will not use `chrome/browser/devtools/DevToolsWindow` as the primary DevTools integration mechanism.

Instead, CBF will implement a Browser-less DevTools integration pattern based on content-layer primitives:

- Target attachment via `content::DevToolsAgentHost`
- Frontend bridging via `content::DevToolsFrontendHost` (shell-style bindings approach)
- DevTools frontend as a CBF-managed `WebContents` so existing surface delivery (`CAContextId` -> `BrowserViewMac`) can render it

CBF API/IPC will expose explicit DevTools lifecycle operations bound to `WebPageId`, supporting:

- open DevTools for inspected page
- optional inspect-element mode trigger
- close/focus DevTools frontend
- map inspected page <-> devtools page relationships

## Consequences

### Positive

- Preserves CBF's Browser-less ownership and layering model.
- Supports both required display modes through host-controlled UI composition.
- Reuses existing CBF surface pipeline and page lifecycle infrastructure.
- Avoids hidden coupling to Chrome tab/window internals.

### Negative / Trade-offs

- Requires new CBF-specific DevTools plumbing (mojom/bridge/Rust API/event mapping).
- More integration code compared with simply calling `DevToolsWindow`.
- Must explicitly define lifecycle/error behavior for DevTools frontend pages.

## Alternatives Considered

### A. Reuse `DevToolsWindow` directly

- Not selected because CBF pages are not Browser-owned tabs, and docking/window behavior depends on Browser integration.

### B. Keep context-menu inspect as the only entry point

- Not selected because it does not satisfy explicit host-controlled embedded/external display requirements and gives weak lifecycle control.

### C. Start with remote debugging only (`--remote-debugging-port`) and no in-app frontend

- Not selected as primary direction because issue #12 requires user-facing in-framework display options.

## Notes

- This ADR selects architecture direction, not exact API names.
- Context-menu inspect may be reintroduced later as a trigger that routes into the same explicit DevTools lifecycle path.
- Keep `cbf` public API browser-generic; Chromium-specific details stay in `cbf-chrome` / `cbf-chrome-sys`.

## Follow-ups

- Define DevTools command/event shape in CBF command model and Chromium bridge mojom.
- Implement inspected-page to devtools-page mapping in CBF profile service.
- Prototype shell-style frontend bindings in CBF fork and verify attach/inspect/close flows.
- Add failure/lifecycle tests for target missing, detached, and late-close races.
- Update setup/implementation docs once first usable API lands.
