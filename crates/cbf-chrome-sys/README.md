# cbf-chrome-sys

`cbf-chrome-sys` is the low-level Rust FFI, wire-boundary, and runtime bridge-loader crate for CBF.

It defines the bridge-facing ABI used by `cbf-chrome`, loads `libcbf_bridge` at runtime, and tracks compatibility by Chromium milestone line.

The runtime bridge API is generated with bindgen's dynamic-loading mode. `bridge.rs`
owns CBF-specific library path resolution and singleton management, while the generated
loader struct in `bridge_api_generated.rs` owns required symbol loading.

CBF is currently in alpha. ABI details, boundary behavior, and security coverage may still change.

See the repository root README.md for overall project status and runtime setup.
