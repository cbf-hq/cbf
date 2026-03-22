# Changelog

All notable changes to CBF Chromium-side runtime history will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

This changelog tracks the Chromium-side runtime baseline associated with
`cbf-chrome-runtime` tags, including `Chromium.app`, `libcbf_bridge.dylib`, and
other Chromium-side changes that affect the runtime line used by CBF crates.

Runtime tags recorded here may exist even when no prebuilt runtime artifacts are
published yet. In that case, the entry records the intended runtime baseline for
source-built use rather than a downloadable binary bundle.

## [Unreleased]

### Added

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

### Changed

- Moved `window.cbf` IPC bootstrap to renderer-side extension install and replaced browser-side navigation-time main-world script execution with isolated-world event dispatch for host->page IPC delivery.

## [cbf-chrome-runtime-v0.1.0-alpha.1+chromium-146.0.7680.31-r1] - 2026-03-17

### Added

- Established the initial Chromium-side runtime baseline for the `0.1.0-alpha.1` CBF crate line.
- Recorded the Chromium milestone 146 runtime state corresponding to:
  - `cbf` `0.1.0-alpha.1`
  - `cbf-chrome` `0.1.0-alpha.1`
  - `cbf-chrome-sys` `146.1.0-alpha.1`

### Changed

- Captured the Chromium fork and bridge state used as the alpha.1 runtime reference point before the `alpha.2` crate cycle.

### Notes

- Bundled runtime target: Chromium `146.0.7680.31`
- Release revision: `r1`
- No prebuilt runtime artifacts were published for this tag.
- This entry exists to document the Chromium/runtime baseline expected by the alpha.1 crate line.

[Unreleased]: https://github.com/cbf-hq/cbf/commits/HEAD/chromium
[cbf-chrome-runtime-v0.1.0-alpha.1+chromium-146.0.7680.31-r1]: https://github.com/cbf-hq/cbf/releases/tag/cbf-chrome-runtime-v0.1.0-alpha.1+chromium-146.0.7680.31-r1
