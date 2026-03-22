# Changelog

All notable changes to `cbf` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Browser-generic browsing context visibility control with `Visible` and `Hidden` states.
- `SetBrowsingContextVisibility` command support for host-driven visibility changes.
- Browser-generic background policy control for browsing contexts and transient browsing contexts.
- `BackgroundPolicy` command support for host-driven opaque and transparent embedded surface behavior.
- `BrowserSession::force_close()` for host-driven immediate shutdown without beforeunload confirmations.
- Browser-generic browsing-context IPC command surface with `EnableIpc`, `DisableIpc`, and `PostBrowsingContextIpcMessage`.
- Browser-generic IPC data models in `cbf::data::ipc` for config, payload, message type, error code, and message envelope.
- Browsing-context IPC inbound event `BrowsingContextEvent::IpcMessageReceived`.

### Changed

- Reorganized native dialog implementation into `dialogs/` with a dedicated macOS module to reduce inline platform `cfg` usage and make future platform-specific presenters easier to add.
- `BrowserSession::Drop` is now a no-op so session teardown no longer implicitly requests graceful shutdown.
- `BackendStopped` now carries fact-only stop reasons; `BackendStopReason::ShutdownRequested` was removed in favor of runtime-local shutdown tracking.
- Documented IPC channel-name contract as non-empty in public API docs.

## [0.1.0-alpha.1] - 2026-03-16

### Added

- Initial public alpha release of the browser-generic Rust API for CBF.
- Browser-generic command and event types for controlling browsing contexts and receiving backend facts.
- Stable logical ID vocabulary for Rust-side browser control surfaces.

### Security

- Marked as an alpha release; the framework is still under active development and may still contain security bugs.

[Unreleased]: https://github.com/cbf-hq/cbf/compare/cbf-v0.1.0-alpha.2...HEAD
[0.1.0-alpha.2]: https://github.com/cbf-hq/cbf/releases/tag/cbf-v0.1.0-alpha.2
[0.1.0-alpha.1]: https://github.com/cbf-hq/cbf/releases/tag/cbf-v0.1.0-alpha.1
