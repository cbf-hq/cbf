# Changelog

All notable changes to CBF Chromium-side runtime history will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

This changelog tracks the Chromium-side runtime baseline associated with
`cbf-chrome-runtime` tags, including `Chromium.app`, `libcbf_bridge.dylib`, and
other Chromium-side changes that affect the runtime version used by CBF crates.

Runtime tags recorded here may exist even when no prebuilt runtime artifacts are
published yet. In that case, the entry records the intended runtime baseline for
source-built use rather than a downloadable binary bundle.

## [Unreleased]

### Changed

- Refactored the Chromium-side shutdown flow so `request_shutdown` performs a non-destructive dirty-tab snapshot and emits `ShutdownBlocked` before any managed browsing context begins closing.
- Shutdown proceeding now uses the confirmed CBF close path after host approval, with clean tabs closing immediately and dirty tabs resuming through auto-confirmed `beforeunload` handling only after `ShutdownProceeding`.

### Fixed

- Prevented clean browsing contexts and host UI surfaces from disappearing as soon as shutdown became blocked by dirty pages.
- Added Chromium unit and browser-test coverage for blocked, cancelled, confirmed, and clean-only shutdown flows, including the regression where clean tabs remained visible while shutdown confirmation was pending.

## [cbf-chrome-runtime-v146.0.0-alpha.3+chromium-146.0.7680.153-r1] - 2026-04-02

### Added

- Create-time browsing-context policy flow across the Chromium bridge, tab manager, and profile service so new browsing contexts can initialize IPC allow-lists and extension capability state when they are created.
- Browser-test coverage for create-time browsing-context IPC initialization and extension suppression behavior on policy-controlled browsing contexts.

### Changed

- Chromium now stores browsing-context extension capability state per `WebContents` and uses that policy to suppress tab-scoped extension behavior for denied contexts, including helper attachment, tab lookup, script injection, and host-triggered action popup activation.

### Fixed

- Extension popups that query the last-focused CBF embedded browser window now resolve the active page origin correctly instead of falling back to `about:` in site-specific UI such as Dark Reader's per-site dark mode toggle. Embedded browser windows now track logical activation through `SetTabFocus()` and participate in Chromium's last-active browser resolution path.
- Mouse input forwarded into CBF embedded browser tabs now routes through Chromium's input event router even for embedded-browser windows, restoring OOPIF iframe hit testing and click delivery instead of dropping forwarded mouse events on out-of-process subframes.

### Notes

- Runtime version: `146.0.0-alpha.3`
- Bundled Chromium: `146.0.7680.153`
- Release revision: `r1`
- `cbf`: `0.1.0-alpha.3`
- `cbf-chrome`: `0.1.0-alpha.3`
- `cbf-chrome-sys`: `146.1.0-alpha.3`
- `cbf-compositor`: `0.1.0-alpha.5`

## [cbf-chrome-runtime-v146.0.0-alpha.2+chromium-146.0.7680.153-r1] - 2026-03-27

### Added

- Host-driven custom scheme responder flow for `app://...` resources across the Chromium bridge, browser service, and loader pipeline, including response metadata for body bytes, MIME type, CSP, and `Access-Control-Allow-Origin`.
- Launch-time custom scheme registration that classifies configured schemes as standard, secure, CORS-enabled, and web-safe origins across browser, renderer, and storage policy setup.
- Browser-test coverage for top-level `app://` navigation rendering and same-origin subresource fetches through the custom scheme responder.
- Host-driven external drag destination flow for macOS webpage drops, including bridge/FFI transport, profile drag-controller routing, and negotiated drag-operation updates back to native views.
- Host-driven browsing context visibility control in the Chromium runtime bridge and profile service.
- macOS surface handle refresh after visibility recovery when the compositor CAContextID changes.
- Host-driven browsing context background policy control through the bridge, profile service, and browser tests.
- Transparent embedded surface handling that applies both page base background color and browser-side view background color for tabs and extension popups.
- Host-disconnect shutdown handling that terminates the browser process without beforeunload once the authenticated Rust host disconnects from the Mojo bridge.
- Host-driven form resubmission prompt flow that replaces direct Chromium repost dialog usage in `ShowRepostFormWarningDialog` with Prompt UI / Auxiliary Window control.
- Form-resubmission prompt metadata transport (reason and target URL) across Chromium Mojo observer events and bridge FFI events.
- Browser-test coverage for POST reload resubmission prompt request/deny flow in `CbfProfileServiceBrowserTest`.
- Browsing-context IPC v1 bridge flow across Chromium browser/renderer boundary, including:
  - context-scoped IPC enable/disable control
  - page->host invoke delivery through dedicated Mojo path
  - host->page notification delivery with text/binary envelope support
  - origin allow-list enforcement via `allowed_origins` and deny-all default when empty
  - browser-test coverage for allow/deny, navigation re-evaluation, and lifecycle failure paths
- Same-document `NavigationStateChanged` emission coverage for SPA-style history updates (`pushState`/`replaceState`/same-document back-forward traversal) with dedicated browser-test assertions.
- Chromium find-in-page bridge flow for tabs, including bridge commands for `FindInPage` and `StopFinding`, `FindReply` observer/event transport, and host-visible match-count / active-match / selection-rect updates.
- Browser-test coverage for find-in-page search, next/previous follow-up navigation, stop-finding behavior, and empty-query no-op handling in `CbfProfileServiceBrowserTest`.
- Browser-test coverage for default-blocked `chrome://settings` behavior, including blocked new-page creation, no-op current-tab navigation, denied new-tab open flow, and unsafe runtime-switch opt-in re-enable coverage.

### Changed

- Moved `window.cbf` IPC bootstrap to renderer-side extension install and replaced browser-side navigation-time main-world script execution with isolated-world event dispatch for host->page IPC delivery.
- Updated navigation-state observer behavior to emit same-document history updates while suppressing duplicate `NavigationStateChanged` payloads via snapshot diffing.
- macOS external drag pasteboard normalization now reuses Chromium `PopulateDropDataFromPasteboard()` behavior instead of collecting arbitrary platform pasteboard flavor strings into webpage-visible drag data.
- Aligned the bridge headers with the generated Rust bindings by treating `cbf_bridge.h` and `cbf_bridge_ffi.h` as the source of truth for bindgen-generated `cbf-chrome-sys` artifacts.
- `chrome://settings` is now blocked by default in the Chromium runtime across direct tab creation, explicit host navigation, current-tab page navigation, and host-mediated tab-open requests, while allowing development-only opt-in through `--cbf-allow-unsafe-settings`.

### Fixed

- macOS production bundle startup for rebranded runtimes by resolving helper executables from the launched runtime name and by aligning Mach rendezvous bootstrap naming with the runtime bundle identifier used by the packaged Chromium engine.
- Removed the undeclared-in-practice `cbf_bridge_client_send_mac_event` export from `cbf_bridge.h`, avoiding runtime bridge loader failure when Rust requires all declared bridge symbols to exist in `libcbf_bridge.dylib`.
- Added prompt/dialog and download prompt enum definitions to `cbf_bridge_ffi.h` so the generated Rust ABI mirror stays in sync with the Chromium bridge header.
- Custom-scheme HTML documents now commit and render as web content instead of source text by providing Chromium with the expected response metadata and first-class scheme registration.
- Non-cryptographic custom-scheme subresource fetches no longer trigger renderer-side bad Mojo failures from unnecessary `SubresourceResponseStarted` IPC.
- Profile teardown stability in `CbfProfileService` by restoring download-prompt prefs before shutdown and removing stale profile-service registry entries during `OnProfileWillBeDestroyed`.
- External drag operation masks now preserve Chromium/AppKit `Move` semantics for `dropEffect = "move"` targets instead of silently degrading the allowed-operation bitmask across the Rust/Chromium boundary.

## [cbf-chrome-runtime-v146.0.0-alpha.1+chromium-146.0.7680.31-r1] - 2026-03-17

### Added

- Established the initial Chromium-side runtime baseline for runtime version
  `146.0.0-alpha.1`.
- Recorded the Chromium milestone 146 runtime state corresponding to:
  - runtime version `146.0.0-alpha.1`
  - `cbf` `0.1.0-alpha.1`
  - `cbf-chrome` `0.1.0-alpha.1`
  - `cbf-chrome-sys` `146.1.0-alpha.1`

### Changed

- Captured the Chromium fork and bridge state used as the alpha.1 runtime
  reference point before the next runtime pre-release cycle.

### Notes

- Bundled runtime target: Chromium `146.0.7680.31`
- Runtime version: `146.0.0-alpha.1`
- Release revision: `r1`
- No prebuilt runtime artifacts were published for this tag.
- This entry exists to document the Chromium/runtime baseline expected by the
  initial alpha.1 runtime version.

[cbf-chrome-runtime-v146.0.0-alpha.3+chromium-146.0.7680.153-r1]: https://github.com/cbf-hq/cbf/releases/tag/cbf-chrome-runtime-v146.0.0-alpha.3+chromium-146.0.7680.153-r1
[cbf-chrome-runtime-v146.0.0-alpha.2+chromium-146.0.7680.153-r1]: https://github.com/cbf-hq/cbf/releases/tag/cbf-chrome-runtime-v146.0.0-alpha.2+chromium-146.0.7680.153-r1
[cbf-chrome-runtime-v146.0.0-alpha.1+chromium-146.0.7680.31-r1]: https://github.com/cbf-hq/cbf/releases/tag/cbf-chrome-runtime-v146.0.0-alpha.1+chromium-146.0.7680.31-r1
