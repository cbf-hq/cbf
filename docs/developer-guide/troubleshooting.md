# Troubleshooting

This chapter collects common failure modes seen in setup and runtime flows.

## 1. Bridge library cannot be linked

Symptoms:

- linker/runtime loader cannot find `cbf_bridge`
- startup fails before backend connection

Checks:

- `CBF_BRIDGE_LIB_DIR` points to the directory that actually contains the bridge library.
- The selected binary architecture matches your Rust target.

## 2. Chromium process fails to spawn

Symptoms:

- `start_chromium` returns spawn errors

Checks:

- `ChromiumProcessOptions.executable_path` exists and is executable.
- Path points to the CBF Chromium-fork binary rather than stock Chromium.

## 3. IPC connection/authentication fails

Symptoms:

- launch appears successful, but connection/auth fails

Checks:

- child command line includes `--cbf-ipc-handle=<endpoint>`
- child command line includes `--cbf-session-token=<token>`
- inherited fd close-on-exec handling is correct
- parent closes its remote fd copy after spawn

## 4. Chromium build drift

Symptoms:

- `cbf_bridge` or Chromium targets break after upstream roll/update

Checks:

- Re-apply/export patch queue and confirm patch order is still valid.
- Reconcile bridge contract changes with current Chromium revision.

## 5. Shutdown/close race regressions

Symptoms:

- intermittent crashes on close, shutdown, or late callbacks

Checks:

- Remove raw pointer capture across async boundaries.
- Use ID re-resolution and weak ownership guards.
- Replace race-path hard `CHECK` with `DCHECK` + safe early return where object absence is expected.
