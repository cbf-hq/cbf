# Changelog

All notable changes to `cbf` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-alpha.4] - 2026-04-09

### Changed

- `BrowserEvent::ShutdownBlocked` now reports a single `dirty_browsing_context_id` for the browsing context that is currently blocking shutdown, instead of precomputing and returning a vector of potential blockers.
- Shutdown confirmation is now modeled as a sequential runtime flow, so the same shutdown request may emit multiple `ShutdownBlocked` events over time as each blocking browsing context requests confirmation.

## [0.1.0-alpha.3] - 2026-04-02

### Added

- Browser-generic browsing-context capability policy models in `cbf::data::policy`, including `BrowsingContextPolicy`, `CapabilityPolicy`, and `IpcPolicy`.
- Create-time browsing-context policy support on `BrowserCommand::CreateBrowsingContext`, allowing hosts to initialize IPC allow-lists and extension capability state when creating a browsing context.

### Changed

- `BrowserHandle::create_browsing_context` now accepts an optional `BrowsingContextPolicy` instead of requiring a separate `create_browsing_context_with_policy` helper.

## [0.1.0-alpha.2] - 2026-03-27

### Added

- Browser-generic browsing context visibility control with `Visible` and `Hidden` states.
- `SetBrowsingContextVisibility` command support for host-driven visibility changes.
- Browser-generic background policy control for browsing contexts and transient browsing contexts.
- `BackgroundPolicy` command support for host-driven opaque and transparent embedded surface behavior.
- Browser-generic external drag destination data models:
  - `ExternalDragEnter`
  - `ExternalDragUpdate`
  - `ExternalDragDrop`
- Browser commands for host-driven external drag destination handling:
  - `SendExternalDragEnter`
  - `SendExternalDragUpdate`
  - `SendExternalDragLeave`
  - `SendExternalDragDrop`
- Browsing-context event `BrowsingContextEvent::ExternalDragOperationChanged` so hosts can mirror the latest negotiated external drag operation in native UI.
- `BrowserSession::force_close()` for host-driven immediate shutdown without beforeunload confirmations.
- Browser-generic browsing-context IPC command surface with `EnableIpc`, `DisableIpc`, and `PostBrowsingContextIpcMessage`.
- Browser-generic IPC data models in `cbf::data::ipc` for config, payload, message type, error code, and message envelope.
- Browsing-context IPC inbound event `BrowsingContextEvent::IpcMessageReceived`.
- Browser-generic form-resubmission prompt support through `AuxiliaryWindowKind::FormResubmissionPrompt`, `AuxiliaryWindowResponse::FormResubmissionPrompt`, and `AuxiliaryWindowResolution::FormResubmissionPrompt`.
- Browser-generic repost reason model `FormResubmissionPromptReason` for reload/back-forward/other/unknown flows.

### Changed

- Reorganized native dialog implementation into `dialogs/` with a dedicated macOS module to reduce inline platform `cfg` usage and make future platform-specific presenters easier to add.
- `BrowserSession::Drop` is now a no-op so session teardown no longer implicitly requests graceful shutdown.
- `BackendStopped` now carries fact-only stop reasons; `BackendStopReason::ShutdownRequested` was removed in favor of runtime-local shutdown tracking.
- Documented IPC channel-name contract as non-empty in public API docs.
- Clarified `BrowsingContextEvent::NavigationStateChanged` semantics to explicitly cover same-document history updates (`pushState`/`replaceState`/same-document traversal), and documented that `is_loading` represents document-navigation loading state only.

## [0.1.0-alpha.1] - 2026-03-16

### Added

- Initial public alpha release of the browser-generic Rust API for CBF.
- Browser-generic command and event types for controlling browsing contexts and receiving backend facts.
- Stable logical ID vocabulary for Rust-side browser control surfaces.

### Security

- Marked as an alpha release; the framework is still under active development and may still contain security bugs.

[0.1.0-alpha.4]: https://github.com/cbf-hq/cbf/compare/cbf-v0.1.0-alpha.3...cbf-v0.1.0-alpha.4
[0.1.0-alpha.3]: https://github.com/cbf-hq/cbf/releases/tag/cbf-v0.1.0-alpha.3
[0.1.0-alpha.2]: https://github.com/cbf-hq/cbf/releases/tag/cbf-v0.1.0-alpha.2
[0.1.0-alpha.1]: https://github.com/cbf-hq/cbf/releases/tag/cbf-v0.1.0-alpha.1
