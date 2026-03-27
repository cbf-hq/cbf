# Changelog

All notable changes to `cbf-cli` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `bundle macos` now bundles the configured `Chromium.app` as a CBF Runtime inside the generated application bundle.
- Added macOS runtime rebranding support for bundled runtimes, including custom runtime app names, bundle identifiers, helper bundle names, and runtime icons.

### Security

- Added optional code signing and post-sign validation for generated macOS app bundles via `--codesign-identity`.

## [0.1.0-alpha.1] - 2026-03-16

### Added

- Initial public alpha release of the CBF developer workflow CLI.
- First public crate line for command-line tooling used in CBF-oriented local development workflows.

### Security

- Marked as an alpha release; commands, workflows, and output are still under active development and may still change.

[Unreleased]: https://github.com/cbf-hq/cbf/compare/cbf-cli-v0.1.0-alpha.1...HEAD
[0.1.0-alpha.1]: https://github.com/cbf-hq/cbf/releases/tag/cbf-cli-v0.1.0-alpha.1
