# Changelog

All notable changes to `cbf-compositor` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial `cbf-compositor` implementation work for the first public alpha release.
- Scene-based browser surface composition for `BrowsingContext` and `TransientBrowsingContext`.
- Native window attachment and scene synchronization for host-managed browser surfaces.
- macOS compositor hosting through a single `CompositorViewMac` that manages multiple Chromium surfaces.
- macOS external drag destination routing for browsing contexts, including hit-tested drag enter/update/leave/drop command emission and negotiated drag-operation reflection back into native drag handling.
- Background policy propagation from composition items to browser-generic background policy commands.
- Validation that rejects compositions which attempt to display the same `SurfaceTarget` more than once across the live compositor state.
- Programmatic active-item switching through `Compositor::set_active_item`, allowing hosts to move browser input focus by `CompositionItemId` while reusing the compositor's native focus-routing path.

### Security

- Marked as an alpha-target crate; compositor behavior and platform integration remain under active development and may still contain security bugs.

[Unreleased]: https://github.com/cbf-hq/cbf/commits/HEAD/crates/cbf-compositor
