# DevTools Integration Investigation for CBF

## Scope

This document summarizes what was verified while investigating issue #12:

- Why DevTools currently opens as a separate Chromium-hosted window.
- Why the current path does not provide usable embedded DevTools for CBF.
- Which Chromium integration path is applicable to CBF architecture.

## Current CBF Context Menu Path

CBF currently forwards context menu commands from host UI to Chromium-side menu execution:

- Rust-side allowlist filtering is implemented in `crates/cbf-chrome/src/data/context_menu.rs`.
- Chromium-side menu handling is in `chromium/src/chrome/browser/cbf/cbf_profile_service.cc`.
- Command dispatch goes through `CbfProfileService::ExecuteContextMenuCommand(...)`.

`Inspect Element` (`IDC_CONTENT_CONTEXT_INSPECTELEMENT = 50162`) is intentionally excluded from the Rust allowlist today.

## What Chromium Does for "Inspect Element"

In Chromium:

- `RenderViewContextMenu::ExecInspectElement()` calls:
  - `DevToolsWindow::InspectElement(render_frame_host, x, y)`
- `DevToolsWindow::InspectElement(...)`:
  - gets a `DevToolsAgentHost` for the tab
  - requests inspect-at-point
  - opens DevTools window with elements panel

Relevant files:

- `chromium/src/chrome/browser/renderer_context_menu/render_view_context_menu.cc`
- `chromium/src/chrome/browser/devtools/devtools_window.cc`

## Why CBF Falls Into Undocked / Separate Window Mode

`DevToolsWindow` docking behavior depends on finding a Chrome `Browser` that owns the inspected `WebContents`.

In CBF:

- pages are created via `content::WebContents::Create(...)` in `CbfTabManager`
- they are not attached to Chrome tab strip / Browser window

As a result, `DevToolsWindow::Create(...)` evaluates docking as unavailable and uses undocked flow (separate DevTools browser window).

Relevant files:

- `chromium/src/chrome/browser/cbf/cbf_tab_manager.cc`
- `chromium/src/chrome/browser/devtools/devtools_window.cc`

## Implication for CBF

CBF can display any `WebContents` surface through the existing CAContextID
pipeline (compositor-owned host view + `CALayerHost`).
However, `DevToolsWindow` is not a good architectural fit for CBF because it is designed around Chrome Browser/tab integration.

So:

- "Use the same compositor-backed surface delivery path" is valid for display mechanics.
- "Use `DevToolsWindow` as-is" is not valid as the primary integration strategy.

## Browser-less DevTools Pattern in Chromium

`content/shell` provides a Browser-less integration pattern:

- create dedicated frontend `WebContents`
- attach frontend/backend via `DevToolsFrontendHost`
- attach inspected target via `DevToolsAgentHost::GetOrCreateForTab(...)`

Relevant files:

- `chromium/src/content/shell/browser/shell_devtools_frontend.cc`
- `chromium/src/content/shell/browser/shell_devtools_bindings.cc`

This pattern aligns with CBF's custom host/UI architecture better than `chrome/browser/devtools/DevToolsWindow`.

## Practical Direction Derived from Investigation

For CBF DevTools support:

1. Keep Browser-less ownership model (`WebPageId` + `CbfTabManager`).
2. Add DevTools-specific control path in CBF service/API (not context-menu-only trigger).
3. Build DevTools integration around `DevToolsAgentHost` + frontend-host bridge (content/shell style).
4. Expose two modes:
   - external DevTools frontend page (separate page/window in CBF terms)
   - embeddable DevTools frontend page (host GUI chooses where/how to show it)

## Notes

- Context-menu inspect can still be re-enabled later as a convenience trigger,
  but it should target the new explicit DevTools lifecycle path.
- DevTools and inspected page lifecycle mapping must remain ID-based (`WebPageId`)
  to stay consistent with CBF async safety and ownership rules.
