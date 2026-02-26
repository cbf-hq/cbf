# DevTools Empty/Wrong Target Investigation (Issue #23)

## Scope

This note captures the root cause and fix for unstable/empty DevTools behavior
in CBF's browser-less DevTools integration.

Observed symptoms:

- DevTools sometimes rendered empty panels (Elements/Console unusable).
- DevTools occasionally attached to unexpected embedded content (e.g. YouTube iframe-like target) instead of the intended page.
- "Select element in page" behavior was unreliable.

## What Was Confirmed

### 1. Frontend messaging path was working

Investigation logs confirmed:

- `ReadyToCommitNavigation` did create `frontend_host_`.
- Frontend sent `Target.setAutoAttach`.
- Backend returned `Target.attachedToTarget`.

So the earlier hypothesis "messages are dropped because `frontend_host_` is null"
was not the primary issue for this failure mode.

### 2. Attached target type was wrong

`Target.attachedToTarget` for the inspected page URL (`https://www.google.com/`)
was observed with:

- `target_type=other`

For normal page inspection flow, DevTools frontend expects page/frame-oriented
target typing. `other` breaks primary page target establishment and can lead to
empty or wrong-inspected content.

## Root Cause

`RenderFrameDevToolsAgentHost::GetType()` delegates target type resolution to
`DevToolsManagerDelegate::GetTargetType(web_contents)` when available.

In Chromium's `ChromeDevToolsManagerDelegate`, non-tab `WebContents` fall back
to `kTypeOther` unless they match known categories (tab/webview/extension/etc).

CBF-managed `WebContents` are intentionally not Chrome tab-strip tabs, so they
fell into this fallback and were reported as `other`.

## Fix

Add CBF-aware target typing in `ChromeDevToolsManagerDelegate::GetTargetType`:

- If a `WebContents` is managed by CBF, return `DevToolsAgentHost::kTypePage`
  instead of `kTypeOther`.

Implementation details:

- Added `CbfBrowserService::IsManagedWebContents(content::WebContents*)`.
- Called it from `ChromeDevToolsManagerDelegate::GetTargetType(...)` via
  `cbf::CbfService::GetForProcess()`.

## Additional Integration Notes

The DevTools controller side keeps browser-less architecture intact:

- DevTools frontend `WebContents` is created directly.
- Attach is performed once at creation (`bindings->AttachTo(agent_host)`).
- Delegate transfer handles frontend cross-RFH replacement.

This matches Chromium behavior more closely while avoiding `DevToolsWindow`
ownership coupling.

## Validation Signal

After fix, a healthy session should show `Target.attachedToTarget` for the
inspected page with page-appropriate target typing (not `other`).
