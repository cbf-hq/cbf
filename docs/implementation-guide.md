# CBF Implementation Guide

This guide is for contributors implementing `cbf-chrome-sys` and the Chromium-side bridge.
For setup flow, see:

- `docs/user-setup-guide.md` (prebuilt artifact users)
- `docs/developer-setup-guide.md` (contributors building Chromium/cbf_bridge)
- `docs/chromium-fork-guide.md` (fork-specific code layout and patch policy)

## 1. Purpose and Audience

This document defines implementation invariants for:

- Chromium bridge code (`cbf_bridge` and related Mojo plumbing)
- `cbf-chrome-sys` FFI boundary code
- Conversion points between low-level bridge events and high-level `cbf` API

It is not a setup or onboarding guide for users of the library.

## 2. IPC and Threading Invariants

CBF bridge is asynchronous and sequence-bound.

- Bind and use `mojo::Remote` only on the sequence where it is bound.
- Route bridge calls through the dedicated Mojo task runner or thread.
- Do not treat IPC calls as synchronous guaranteed-success calls.
- Handle disconnect and late callback paths as normal control flow.

## 3. Async Lifetime Safety Patterns

The bridge must be robust under shutdown, tab close, and process restart races.

- Do not capture raw `WebContents*` across async boundaries.
- Do not capture owning `this` pointers in callbacks that may outlive owners.
- Use stable logical IDs (for example `BrowsingContextId`) and resolve at execution time.
- Guard owners with weak pointers (`WeakPtr` pattern).
- If re-resolution fails, treat it as a safe no-op instead of crashing.

### Emergency hatch: restricted `base::Unretained`

Using `base::Unretained(...)` is allowed only as a temporary exception when all
conditions below are met:

- Object lifetime is strictly guaranteed for the full async task/callback window.
- Code comments explain why it is safe and which lifetime guarantee is relied on.
- A tracking reference (issue/TODO) is added for replacing it.
- Prefer replacing with `ID + re-resolution` in the next follow-up change.

### `CHECK` and `DCHECK` policy on race-prone paths

On shutdown/close race paths, avoid crashing on missing objects with `CHECK`.
Prefer:

- `DCHECK(<expected condition>)` for debug visibility.
- Guard + early return (no-op) when the object is unavailable at runtime.

## 4. FFI Boundary Contract

Layering must remain strict:

- Keep C ABI types and exported C functions in `cbf-chrome-sys`.
- Keep high-level browser domain API in `cbf`.
- Do not leak Chromium or Mojo implementation details into public `cbf` types.
- Restrict conversion logic to boundary layers instead of spreading it through public API code.

### Platform Boundary Rule

- Do not re-implement Chromium-specific behavior in `cbf` (Rust high-level layer).
- Keep Chromium-specific conversion/state-machine logic in Chromium code or `cbf-chrome-sys` (`cbf_bridge`).
- When Chromium already provides a conversion path, use it via `cbf-chrome-sys` instead of recreating logic in Rust.

### Constant Ownership Rule

- Do not duplicate Chromium-owned constants in `cbf`.
- If Rust needs Chromium constants or enum values, expose them through `cbf-chrome-sys`/`cbf_bridge`.
- Avoid manually mirroring phase/flag values in high-level Rust code.

### Bridge-First Rule

- Prefer bridge APIs that forward Chromium-native values directly.
- Rust should treat bridge output as the source of truth for Chromium-specific fields.

Dependency direction remains:

`Application -> cbf -> cbf-chrome -> cbf-chrome-sys -> Chromium process`

## 5. Failure Handling Contract

IPC failures are expected runtime states:

- backend not running
- connection dropped
- timeout or protocol mismatch
- renderer or browser process crash

Implementations should surface these as events or errors that allow upstream recovery choices (retry, recreate session, fail-fast).

## 6. Implementation Troubleshooting

### Build drift between Chromium and bridge

- Symptom: bridge build failures after Chromium update.
- Action: update patches/contracts carefully, then repin tested revisions.

### Async lifetime crashes

- Symptom: shutdown or close race crashes.
- Action: audit callbacks for raw pointer capture, then replace with ID re-resolution and weak-owner guards.

### IPC channel mismatch

- Scope: typically relevant in manual launch/debug flows, not in normal `start_chromium` usage.
- Symptom: Rust side cannot connect despite successful launch.
- Action: verify the channel name and handshake assumptions on both bridge and client sides.

## 7. Contributor Review Checklist

Before merging bridge/boundary changes, confirm:

- No raw pointer ownership crosses async boundaries.
- `base::Unretained` usage (if any) is justified, documented, and tracked.
- Race-prone paths use `DCHECK + return` style guards where missing objects are expected.
- New API surface additions stay browser-generic at `cbf` layer.
- Chromium-specific details remain contained in bridge or `cbf-chrome-sys`.
- Disconnect/crash paths have explicit behavior and tests where feasible.

## 8. Advanced: Manual Chromium Launch

For normal library usage, prefer `start_chromium` from `cbf`.
Manual launch is intended for debugging and bridge development workflows.

Example:

```bash
./out/Default/chrome \
  --enable-features=Cbf \
  --cbf-ipc-channel=example-channel \
  --enable-logging=stderr \
  --log-file=/tmp/chromium_debug.log
```

Flag notes:

- `--enable-features=Cbf`: enables CBF feature path.
- `--cbf-ipc-channel=...`: must match the channel used by the Rust client.
- logging flags: useful for bridge-side debugging.
