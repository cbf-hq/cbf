# Changelog

All notable changes to `cbf-chrome` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Chromium backend wiring for browsing context visibility commands through the Rust API and FFI client.
- macOS surface handle refresh after visibility recovery when the underlying CAContextID changes.
- Chromium backend wiring for browsing context and extension popup background policy commands.
- Transparent background transport support that drives both page base background color and browser-side view background color for embedded Chromium surfaces.
- `ChromiumRuntime` as an opt-in runtime wrapper for Chromium process ownership, staged shutdown, and signal-driven shutdown handling.
- Runtime shutdown state reporting so hosts can distinguish expected shutdown from unexpected disconnects without relying on backend stop-reason inference.
- Chrome-transport IPC data models in `cbf-chrome::data::ipc` (`TabIpcMessage`, payload/type/error/config) with conversions to/from browser-generic `cbf::data::ipc`.
- Chromium backend command/event transport wiring for browsing-context IPC (`EnableTabIpc`, `DisableTabIpc`, `PostTabIpcMessage`, and IPC event mapping).
- Browser-test coverage for IPC enabled/disabled behavior, `allowed_origins` checks, host->page notification delivery, and binary/text envelope paths.

### Changed

- Hardened shutdown flow to use explicit force-close handling, staged process termination, and best-effort cleanup instead of relying on session drop side effects.
- `BackendStopped` emission now preserves fact-only disconnect reasons; shutdown intent is tracked locally in `ChromiumRuntime` rather than inferred from transport teardown.
- `simpleapp` now suppresses shutdown-time disconnect warnings and avoids duplicate shutdown requests by consulting `ChromiumRuntime` state.
- IPC bootstrap moved to renderer-side `window.cbf` installation and host->page delivery now uses isolated-world event dispatch to avoid navigation-time JS execution crashes.

## [0.1.0-alpha.1] - 2026-03-16

### Added

- Initial public alpha release of the Chromium-specific safe Rust API layer for CBF.
- First public crate line connecting the browser-generic `cbf` surface to the Chromium-backed runtime through `cbf-chrome-sys`.
- Chrome backend integration intended for current CBF-supported runtime targets.

### Security

- Marked as an alpha release; runtime behavior and backend integration are still under active development and may still contain security bugs.

[Unreleased]: https://github.com/cbf-hq/cbf/compare/cbf-chrome-v0.1.0-alpha.2...HEAD
[0.1.0-alpha.2]: https://github.com/cbf-hq/cbf/releases/tag/cbf-chrome-v0.1.0-alpha.2
[0.1.0-alpha.1]: https://github.com/cbf-hq/cbf/releases/tag/cbf-chrome-v0.1.0-alpha.1
