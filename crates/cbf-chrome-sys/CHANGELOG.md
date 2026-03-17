# Changelog

All notable changes to `cbf-chrome-sys` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [146.1.0-alpha.2] - 2026-03-17

### Added

- FFI support for Chromium tab visibility commands in the bridge client ABI.

## [146.1.0-alpha.1] - 2026-03-16

### Added

- Initial public alpha release of the low-level Rust FFI and wire boundary crate for CBF.
- Established the Chromium milestone 146 crate line for Cargo users.
- First public ABI boundary crate used by `cbf-chrome` to communicate with the CBF bridge layer.

### Security

- Marked as an alpha release; ABI details and boundary behavior are still under active development and may still contain security bugs.

[Unreleased]: https://github.com/cbf-hq/cbf/compare/cbf-chrome-sys-v146.1.0-alpha.2...HEAD
[146.1.0-alpha.2]: https://github.com/cbf-hq/cbf/releases/tag/cbf-chrome-sys-v146.1.0-alpha.2
[146.1.0-alpha.1]: https://github.com/cbf-hq/cbf/releases/tag/cbf-chrome-sys-v146.1.0-alpha.1
