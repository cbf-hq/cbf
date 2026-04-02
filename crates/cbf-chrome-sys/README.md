# cbf-chrome-sys

`cbf-chrome-sys` is the low-level Rust FFI, wire-boundary, and runtime bridge-loader crate for CBF.

It defines the bridge-facing ABI used by `cbf-chrome`, loads `libcbf_bridge` at runtime, and tracks compatibility by Chromium milestone line.

The ABI mirror in `ffi_data_generated.rs` and the runtime bridge loader in
`ffi_bridge_generated.rs` are both generated with bindgen. `bridge.rs` owns
CBF-specific library path resolution and singleton management.

Regenerate both generated files with `uv run tool ffi generate`, and verify
that the checked-in output is current with `uv run tool ffi verify`.

CBF is currently in alpha. ABI details, boundary behavior, and security coverage may still change.

See the repository root README.md for overall project status and runtime setup.
