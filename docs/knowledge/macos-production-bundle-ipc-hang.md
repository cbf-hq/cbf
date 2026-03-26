# macOS Production Bundle IPC Hang (Mach Port Rendezvous ID Mismatch)

## Symptom

When running a CBF application as a production bundle on macOS, the startup process hangs at the `authenticate` step. While the Chromium child process is launched successfully in the background, it fails to establish a Mojo connection with the Rust host process.

## Investigation

The hang occurs because the Mojo IPC channel initialization fails during the "Mach Port Rendezvous" phase on macOS. 

In Chromium's macOS implementation, Mach ports are exchanged using a bootstrap name registered in the system-wide Mach bootstrap namespace. This name is constructed using `base::apple::BaseBundleID()` and the process ID (PID/PPID).

1.  **Parent (Rust Host)**: Loads `libcbf_bridge.dylib`. By default, it uses either `org.chromium.Chromium` or the main app's bundle ID (e.g., `io.github.cbf-hq.cbf.simpleapp`).
2.  **Child (Chromium Engine)**: In a bundled environment, the Chromium engine (e.g., `CBF SimpleApp Engine.app`) explicitly overrides its `BaseBundleID` to its own bundle ID (e.g., `io.github.cbf-hq.cbf.simpleapp.runtime`) during early initialization in `chrome_main_mac.mm`.
3.  **The Mismatch**: When the parent registers the Mach port under one bundle ID and the child searches for it under a different bundle ID, the rendezvous fails. Consequently, the Mojo connection is never established, leading to a hang during the authentication handshake.

## Root Cause

The `BaseBundleID` mismatch between the host process and the Chromium engine process in production bundles prevents the Mach port rendezvous from succeeding.

## Solution

To resolve this, the host process must align its `BaseBundleID` with the one used by the Chromium engine before initializing the IPC channel.

### 1. Bridge Extension

A new FFI function `cbf_bridge_set_base_bundle_id` was added to the bridge library:

- **Header (`cbf_bridge.h`)**:
  ```c
  CBF_BRIDGE_EXPORT void cbf_bridge_set_base_bundle_id(const char* bundle_id);
  ```
- **Implementation (`cbf_bridge.cc`)**:
  Calls `base::apple::SetBaseBundleIDOverride(bundle_id)` to ensure the bridge uses the specified ID for Mach rendezvous.

### 2. Rust Integration (`cbf-chrome`)

The `start_chromium` function in the `cbf-chrome` crate was updated to:

1.  Locate the `Info.plist` of the Chromium runtime bundle being launched.
2.  Extract the `CFBundleIdentifier` (Bundle ID) from the `Info.plist`.
3.  Call `IpcClient::set_base_bundle_id` with the extracted ID before calling `prepare_channel`.

This ensures that both the host and the engine use the exact same bundle ID to construct the Mach bootstrap name, allowing the rendezvous to succeed.

## Verification

This fix was verified by launching the `dist/CBF SimpleApp.app` production bundle. The application now successfully completes the authentication handshake and displays the UI as expected.
