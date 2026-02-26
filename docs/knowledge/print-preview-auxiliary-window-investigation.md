# Print Preview and Auxiliary Window Investigation for CBF

Date: 2026-02-25
Related: ADR 0004, Issue #22

## Summary

- Chromium print preview is implemented as a constrained web dialog backed by a dedicated dialog `WebContents`.
- For CBF, print/PDF flow should be wired via Chromium printing entrypoints (`printing::StartPrint`, optionally `StartBasicPrint`).
- Current CBF bootstrap does not initialize printing helpers yet, so print preview pipeline is not attached for CBF-managed `WebContents`.
- Print preview can be modeled as CBF `AuxiliaryWindow` and host-controllable close path can be provided.

## Findings

### 1) Print preview UI shape

Chromium print preview uses `PrintPreviewDialogController` and creates dialog UI through `ShowConstrainedWebDialog(...)` with a `PrintPreviewDialogDelegate`.

Implication:
- This is not a CBF-owned custom UI.
- It is a dialog-like secondary surface and aligns with CBF `AuxiliaryWindow` semantics.

Key references:
- `chrome/browser/printing/print_preview_dialog_controller.cc`

### 2) Printing API entrypoints to use

Primary APIs:
- `printing::StartPrint(content::WebContents*, ..., print_preview_disabled, has_selection)`
- `printing::StartBasicPrint(content::WebContents*)` (system/basic path)

Important behavior:
- `StartPrint` selects the proper frame host for PDF and non-PDF content.
- It triggers preview flow via `PrintViewManager` when preview is enabled.

Key references:
- `chrome/browser/printing/print_view_manager_common.cc`
- `chrome/browser/ui/browser_commands.cc` (`Print` / `CanPrint` flow)

### 3) Required bootstrap in CBF WebContents path

`BootstrapCbfWebContents(...)` currently has a placeholder `AttachPrintingPdfHelpers(...)` and does not initialize printing.

Expected minimum wiring:
- call `printing::InitializePrintingForWebContents(web_contents)` from CBF bootstrap path.

Key references:
- `chrome/browser/cbf/profile/cbf_web_contents_bootstrap.cc`
- `chrome/browser/printing/printing_init.cc`

### 4) Current CBF AuxiliaryWindow constraints

Current CBF auxiliary window model is extension-install-prompt-specific:
- `CbfAuxiliaryWindowKind` has only `kExtensionInstallPrompt`.
- request/response/resolution payloads are extension-centric.
- bridge and Rust side mirror the same constrained schema.

Implication:
- Print preview cannot be represented as `AuxiliaryWindow` without schema expansion.

Key references:
- `chrome/browser/cbf/mojom/cbf_browser.mojom`
- `chrome/browser/cbf/cbf_profile_service.cc`
- `crates/cbf/src/data/extension.rs`
- `chrome/browser/cbf/bridge/cbf_bridge_ffi.h`

## Direction (pre-implementation)

To satisfy Issue #22 and keep API layer-correct:

1. Add print command path in CBF profile service (new command, not context-menu-only trigger).
2. Expand CBF auxiliary window schema to include print preview kind.
3. Emit auxiliary lifecycle events for preview dialog open/close.
4. Support host-initiated close via existing `CloseAuxiliaryWindow`.
5. Keep failures non-fatal and surfaced as events/errors.

## Open points

- Whether to expose only preview flow first (`StartPrint`) or also basic/system dialog command at initial rollout.
- Exact event naming granularity for print lifecycle beyond auxiliary open/close (requested/resolved/failure detail).
- How strictly to mirror Chromium `CanPrint` gating (policy/prefs/content restrictions) in CBF API response semantics.

