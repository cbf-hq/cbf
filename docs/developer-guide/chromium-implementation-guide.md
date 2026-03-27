# Chromium Implementation Guide

This guide is for contributors implementing Chromium-side behavior in `cbf-chrome-sys` and
`cbf_bridge`-adjacent code.

For the higher-level invariants and design constraints, see
[Chromium Integration Rules](./chromium-integration-rules.md).

For setup flow, see:

- [Contributor Setup](./contributor-setup.md)
- [Chromium Fork Workflow](./chromium-fork-workflow.md)

## 1. Purpose and audience

This document provides implementation guidance for:

- `cbf_bridge` and Mojo plumbing updates
- `cbf-chrome-sys` boundary code and FFI surfaces
- conversion or dispatch layers between Chromium process data and `cbf` events/commands

It is not a product API guide.

## 2. IPC and threading invariants (implementation)

IPC contract policy is defined in [Chromium Integration Rules §3](./chromium-integration-rules.md).

Concrete Mojo/threading patterns:

- Bind each `mojo::Remote` on its intended sequence and keep all use-sites on that sequence.
- Route bridge calls through the task runner / sequence for the bound remote.
- Add explicit sequence checks where practical to catch regressions during development.
- Express async continuation as callbacks or futures — do not block threads waiting for IPC results.
- Treat disconnect and late callbacks as regular control-flow branches (`drop`/`retry`/surface error).

## 3. Async lifetime safety patterns (implementation)

Lifetime safety policy is defined in [Chromium Integration Rules §4](./chromium-integration-rules.md).

Concrete implementation checklist:

- Capture stable IDs (e.g., `WebPageId`) and resolve platform objects (e.g., `WebContents*`) at task
  execution time — do not cache raw pointers across async boundaries.
- Treat unresolved IDs as expected; apply a guarded no-op and emit a structured error/event where
  applicable.

### Emergency hatch: restricted `base::Unretained`

`base::Unretained(...)` is acceptable only as temporary technical debt:

- You can prove lifetime for the full async window.
- The reason is documented in code comments.
- A follow-up task/TODO tracks conversion to safe ownership.
- The preferred next step is replacing it with `ID + re-resolution`.

## 4. FFI boundary implementation

### Platform boundary rule

- Keep Chromium-specific behavior out of public `cbf` APIs.
- Keep conversion/state-machine logic in bridge or `cbf-chrome-sys`.
- Convert to Rust-facing API shapes at boundary layers only.

### Constant ownership rule

- Do not duplicate Chromium constants in `cbf`.
- Expose required constant values through `cbf-chrome-sys`/bridge when Rust needs them.

### Bridge-first rule

- Prefer native Chromium values from bridge outputs.
- Treat bridge outputs as the source of truth and avoid reconstructing Chromium state in `cbf`.

### Runtime bridge loader rule

- Keep `cbf_bridge` symbol loading in `cbf-chrome-sys`; higher crates should call
  through `bridge()` rather than maintaining their own symbol tables.
- Regenerate the runtime bridge API with `uv run tool ffi generate` after bridge export
  changes in `cbf_bridge.h`.
- Do not hand-edit `bridge_api_generated.rs`; treat it as bindgen output.

## 5. Failure handling implementation

Failure policy is defined in [Chromium Integration Rules §5](./chromium-integration-rules.md).

Expected runtime conditions to handle gracefully (not exceptional):

- backend not running
- backend disconnects
- timeout and protocol mismatch
- renderer/browser crash

Additional implementation note:

- Retain in-progress state for recoverable retries only when it is safe to restore.

## 6. Implementation troubleshooting

### Build drift between Chromium and bridge

If bridge compilation fails after Chromium changes:

- rebase/refresh CBF patches
- regenerate bridge contracts where needed
- validate ABI expectations in `cbf-chrome-sys`

### Async lifetime crash patterns

If shutdown/close races crash:

- inspect captured values in callbacks
- replace raw pointer captures with IDs and `WeakPtr`
- remove synchronous assumptions from callback chains

### IPC connection failures

If connect/auth path fails:

- confirm inherited endpoint and session-token are passed consistently
- verify fd flags are prepared as expected before spawn
- ensure parent side closes its copied endpoint appropriately after spawn
- confirm Rust and Chromium agree on the selected session token

## 7. Contributor review checklist

Before merging bridge/boundary changes, verify:

- no raw pointer ownership crosses async boundaries
- any `base::Unretained` use has explicit justification + replacement plan
- race-prone paths use guarded no-op behavior rather than hard assertions
- new APIs keep Chromium details inside bridge/FFI layers
- disconnect/crash paths have explicit behavior and, where feasible, tests

## 8. Chromium-side targets and execution

For Chromium-side impact, build and run:

- `chromium/src/out/Default/cbf_tests`
- `chromium/src/out/Default/browser_tests`
- `chromium/src/out/Default/unit_tests`

Prefer focused filters first (for example `Cbf*` suites), then expand.

Example commands:

```bash
chromium/src/out/Default/browser_tests --gtest_filter='Cbf*'
chromium/src/out/Default/unit_tests --gtest_filter='CbfBrowserServiceTest.*'
```

## 9. Required Chromium switches injected by `start_chromium`

`start_chromium` is the default runtime path and injects required Chromium flags
for CBF bootstrap automatically.

The following switches are required and are always set by `start_chromium`:

- `--enable-features=Cbf`
  - Enables CBF-specific Chromium feature wiring.
  - Without this, CBF bridge integration paths are not activated.

- `--cbf-ipc-handle=<platform-specific-value>`
  - Carries the inherited IPC endpoint information generated by
    `IpcClient::prepare_channel()`.
  - Chromium uses this value to recover the parent-provided endpoint and complete
    the bridge-side IPC bootstrap.

- `--cbf-session-token=<hex-token>`
  - Provides per-process-session authentication material.
  - The Rust and Chromium sides must agree on this token during initial
    handshake/authentication.

Common optional flags controlled by `StartChromiumOptions` include:

- `--user-data-dir=...`
  - When `user_data_dir` is set, `start_chromium` also injects
    `--breakpad-dump-location=<user_data_dir>/Crashpad` so Crashpad data stays
    under the same application data root on macOS.
- `--enable-logging=...`
- `--log-file=...`
- `--v=...` / `--vmodule=...`
- `--no-startup-window` (default unless explicitly overridden)

Manual Chromium launch is primarily for low-level local debugging. If you bypass
`start_chromium`, you must provide equivalent bootstrap inputs yourself.
