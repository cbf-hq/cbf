# Changelog

All notable changes to `cbf-chrome-sys` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [146.1.0-alpha.4] - 2026-04-09

### Added

- Bridge FFI support for the macOS Mach rendezvous child-launch contract, including explicit `prepare_channel_and_lock`, `pass_child_pid_and_unlock`, and `abort_channel_launch` entry points so hosts can keep the rendezvous lock held across spawn and abort safely on launch failures.

### Changed

- Regenerated `ffi_bridge_generated.rs` to expose the new bridge APIs used by `cbf-chrome` for lock-scoped child launch on macOS.

## [146.1.0-alpha.3] - 2026-04-02

### Added

- FFI and generated binding support for create-time browsing-context policy transport, including IPC allow-list initialization data and browsing-context extension capability state in the bridge client ABI.

### Changed

- Regenerated `ffi_data_generated.rs` and `ffi_bridge_generated.rs` to mirror the updated Chromium bridge headers for browsing-context policy transport.

## [146.1.0-alpha.2] - 2026-03-27

### Added

- FFI support for host-driven custom scheme request/response transport in the bridge ABI, including request metadata, response result values, body bytes, MIME type, CSP, and `Access-Control-Allow-Origin`.
- FFI support for launch-time custom scheme registration so Chromium can classify configured schemes as first-class web origins before the browser process starts.
- macOS bridge FFI support for overriding the base bundle ID used by Mach rendezvous, so packaged hosts can align bootstrap naming with a rebranded Chromium runtime bundle.
- FFI support for host-driven external drag destination transport in the bridge ABI, including external drag enter/update/leave/drop commands and negotiated drag-operation change events.
- FFI support for Chromium tab and extension popup background policy commands in the bridge client ABI.
- FFI support for browsing-context IPC commands and envelope fields in the bridge ABI, including channel, request/response metadata, text/binary payloads, and structured IPC error codes.
- Bridge event ABI extensions for page->host IPC notifications and corresponding Rust-side mapping support.
- FFI support for Chromium tab visibility commands in the bridge client ABI.
- FFI constants and bridge event fields for form-resubmission prompt transport:
    - auxiliary/prompt-ui kind value for form resubmission
    - repost reason enum values
    - repost reason and repost target URL fields on `CbfBridgeEvent`.
- Bridge ABI support for Chromium `FindInPage` / `StopFinding` commands and raw `FindReply` event transport, including match counts, active ordinal, final-update state, and selection rectangle fields.
- Crate-local bindgen generation tooling for both `ffi_data_generated.rs` and `ffi_bridge_generated.rs`, so the checked-in FFI mirror and runtime-loaded bridge API can be regenerated directly from the Chromium bridge headers.

### Changed

- `cbf-chrome-sys` now resolves `libcbf_bridge` at runtime with `libloading` instead of relying on Cargo link-time bridge configuration.
- `ffi_generated.rs` is removed and new `ffi_data_generated.rs` and `ffi_bridge_generated.rs` are now a complete bindgen mirror of `cbf_bridge_ffi.h` instead of a handwritten Rust-side ABI copy.
- `ffi_bridge_generated.rs` (former `bridge_api_generated.rs`) is now generated with bindgen dynamic loading from `cbf_bridge.h`, replacing the handwritten symbol table and bridge-call wrapper layer.
- FFI generation ownership now lives under `crates/cbf-chrome-sys`, with the repo-level tool entrypoint acting as a thin wrapper around the crate-local generator.

## [146.1.0-alpha.1] - 2026-03-16

### Added

- Initial public alpha release of the low-level Rust FFI and wire boundary crate for CBF.
- Established the Chromium milestone 146 crate line for Cargo users.
- First public ABI boundary crate used by `cbf-chrome` to communicate with the CBF bridge layer.

### Security

- Marked as an alpha release; ABI details and boundary behavior are still under active development and may still contain security bugs.

[146.1.0-alpha.4]: https://github.com/cbf-hq/cbf/compare/cbf-chrome-sys-v146.1.0-alpha.3...cbf-chrome-sys-v146.1.0-alpha.4
[146.1.0-alpha.3]: https://github.com/cbf-hq/cbf/releases/tag/cbf-chrome-sys-v146.1.0-alpha.3
[146.1.0-alpha.2]: https://github.com/cbf-hq/cbf/releases/tag/cbf-chrome-sys-v146.1.0-alpha.2
[146.1.0-alpha.1]: https://github.com/cbf-hq/cbf/releases/tag/cbf-chrome-sys-v146.1.0-alpha.1
