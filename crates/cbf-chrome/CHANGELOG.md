# Changelog

All notable changes to `cbf-chrome` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Chrome-specific custom scheme registration and response transport for host-provided `app://...` resources, including:
  - launch-time custom scheme classification through `ChromiumBackendOptions`
  - `ChromeEvent::CustomSchemeRequestReceived`
  - `ChromeCommand::RespondCustomSchemeRequest`
  - response metadata for body bytes, MIME type, CSP, and `Access-Control-Allow-Origin`
- Chromium backend support for host-driven external native drag destinations on macOS, including:
  - external drag enter/update/leave/drop command transport
  - negotiated drag-operation observer events
  - native view and compositor routing for webpage drop targets
- Chromium backend wiring for browsing context visibility commands through the Rust API and FFI client.
- macOS surface handle refresh after visibility recovery when the underlying CAContextID changes.
- Chromium backend wiring for browsing context and extension popup background policy commands.
- Transparent background transport support that drives both page base background color and browser-side view background color for embedded Chromium surfaces.
- `ChromiumRuntime` as an opt-in runtime wrapper for Chromium process ownership, staged shutdown, and signal-driven shutdown handling.
- Runtime shutdown state reporting so hosts can distinguish expected shutdown from unexpected disconnects without relying on backend stop-reason inference.
- Chrome-transport IPC data models in `cbf-chrome::data::ipc` (`TabIpcMessage`, payload/type/error/config) with conversions to/from browser-generic `cbf::data::ipc`.
- Chromium backend command/event transport wiring for browsing-context IPC (`EnableTabIpc`, `DisableTabIpc`, `PostTabIpcMessage`, and IPC event mapping).
- Browser-test coverage for IPC enabled/disabled behavior, `allowed_origins` checks, host->page notification delivery, and binary/text envelope paths.
- Chromium Prompt UI transport support for host-driven form resubmission confirmation, including:
  - `PromptUiKind::FormResubmissionPrompt`
  - repost reason mapping
  - repost target URL mapping
  - `PromptUiResponse::FormResubmissionPrompt` response wiring
- Generic conversion support between Chrome Prompt UI and browser-generic auxiliary window models for form resubmission flows.
- Chrome-specific `FindInPage` command helpers and raw `FindReply` events for Chromium page-text search, including `StopFinding` actions and follow-up next/previous navigation.

### Changed

- macOS surface embedding guidance now points applications to `cbf-compositor` as the standard host-side integration path instead of the old `cbf-chrome`-local view helper.
- `cbf-chrome` now dispatches bridge calls through `cbf-chrome-sys` runtime-loaded symbol wrappers instead of relying on downstream crate `build.rs` linkage setup.
- `cbf-chrome` now consumes bindgen-generated `cbf-chrome-sys` ABI names and bridge loader methods directly, removing dependence on the old handwritten bridge symbol wrapper layer.
- `simpleapp` now serves embedded toolbar assets over `app://simpleapp/...` instead of resolving `file://` URLs from the Cargo manifest location, so the same UI loading path works in development and packaged builds.
- Hardened shutdown flow to use explicit force-close handling, staged process termination, and best-effort cleanup instead of relying on session drop side effects.
- `BackendStopped` emission now preserves fact-only disconnect reasons; shutdown intent is tracked locally in `ChromiumRuntime` rather than inferred from transport teardown.
- `simpleapp` now suppresses shutdown-time disconnect warnings and avoids duplicate shutdown requests by consulting `ChromiumRuntime` state.
- IPC bootstrap moved to renderer-side `window.cbf` installation and host->page delivery now uses isolated-world event dispatch to avoid navigation-time JS execution crashes.
- macOS external drag pasteboard conversion now follows Chromium's normalized `DropData` population instead of exposing raw platform pasteboard flavor strings through webpage-visible drag data.

### Removed

- Removed the legacy macOS `BrowserViewMac` embedding implementation and its host-owned choice-menu presenter from `cbf-chrome`; surface embedding now goes through `cbf-compositor`.

### Fixed

- macOS production bundle startup for rebranded runtimes by reading the launched runtime bundle identifier from `Info.plist` and aligning the host-side Mach rendezvous base bundle ID before bridge initialization.
- macOS packaged applications now locate `libcbf_bridge.dylib` from `Contents/Frameworks` through `cbf-chrome-sys` runtime bridge lookup instead of executable rpath rewriting.
- Bridge startup on macOS no longer fails during runtime symbol loading because `cbf_bridge_client_send_mac_event` is no longer declared as a required bridge export without a corresponding Chromium-side implementation.
- Chrome drag-operation bitmask conversion now maps `Move` to Chromium/AppKit value `16`, restoring external-drop handling for `dropEffect = "move"` targets.
- DevTools context menus now preserve the verified element-inspector commands in the Chrome-side allowlist, restoring right-click menu display for inspected nodes while continuing to filter unsupported actions.

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
