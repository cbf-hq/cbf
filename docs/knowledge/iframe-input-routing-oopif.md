# Iframe Input Routing and OOPIF in CBF (macOS)

## 1. Scope

This note captures a CBF-specific iframe input issue and the long-term fix
strategy.

Focus:

- Why some iframes were interactive and others were not.
- Why right-click could appear to "pass through" iframe content.
- How to route mouse input safely with Chromium's OOPIF model.

## 2. Observed Symptoms

In CBF-based browser embedding, iframe behavior was inconsistent:

- Some iframes (for example, same-origin/simple embeds) accepted clicks.
- Some iframes (for example, YouTube embeds) ignored clicks.
- Right-click on affected iframes often opened the parent page context menu.

## 3. Root Cause

The CBF Chromium-side input path forwarded mouse events directly to a selected
`RenderWidgetHost` (`focused/main` fallback).

That bypassed Chromium's standard input hit-test router, which is responsible
for targeting the correct renderer view across frame/process boundaries.

For OOPIF (out-of-process iframe) content, direct forwarding can target the
wrong widget (often the main frame), causing:

- pointer events not reaching the iframe renderer,
- context menu ownership resolved on parent frame,
- behavior differences between in-process iframes vs OOPIF iframes.

## 4. Why Only Some Iframes Failed

Not all iframes are rendered the same way:

- In-process iframe: direct forwarding may still work by chance.
- OOPIF iframe: correct delivery requires coordinate-based routing through
  Chromium's input router.

Cross-origin embeds (such as many YouTube embeds) are frequently OOPIF, so they
are more likely to expose this bug.

## 5. Final Fix Direction

### 5.1 Use Chromium's input router first

Mouse and wheel events should be routed through:

- `WebContentsImpl::GetInputEventRouter()`
- `RouteMouseEvent(...)`
- `RouteMouseWheelEvent(...)`

with the root `RenderWidgetHostViewBase`.

### 5.2 Keep a defensive fallback

If router/root view is temporarily unavailable, fallback to the legacy direct
`RenderWidgetHost` forwarding path to avoid complete input loss during unusual
transitional states.

### 5.3 Apply same policy to synthetic wheel-end

Synthetic wheel-end events must follow the same routing policy (router first,
legacy fallback) to keep wheel target consistency across iframe boundaries.

## 6. Layering Guidance

For CBF architecture:

- Keep high-level `cbf` API browser-generic.
- Keep Chromium-specific routing semantics in Chromium-side CBF service.
- Avoid implementing frame hit-testing logic in Rust or public API layers.

This keeps CBF aligned with Chromium's native event model and reduces
maintenance risk across Chromium upgrades.

## 7. Practical Validation Checklist

When changing iframe input behavior, validate at least:

1. Click and right-click on known OOPIF embeds (for example, YouTube iframe).
2. Click and right-click on known in-process iframe samples.
3. Wheel scrolling over iframe boundaries.
4. No regressions in top-frame input and context menu behavior.
5. No crashes/assertions in input routing paths during rapid focus/frame
   transitions.

## 8. Operational Takeaway

For long-term reliability, CBF should treat Chromium's input router as the
primary contract for mouse/wheel delivery in multi-frame pages, especially when
OOPIF is involved.
