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
