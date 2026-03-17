# Changelog

All notable changes to `cbf` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-alpha.2] - 2026-03-17

### Added

- Browser-generic browsing context visibility control with `Visible` and `Hidden` states.
- `SetBrowsingContextVisibility` command support for host-driven visibility changes.

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
