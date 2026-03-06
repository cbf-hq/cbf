# Chromium Fork Workflow

This chapter covers Chromium-fork specific workflow for CBF contributors.

## 1. Scope

CBF requires a Chromium fork with CBF integration.
Do not assume stock Chromium has equivalent behavior.

Relevant locations:

- CBF Chromium code: `chromium/src/chrome/browser/cbf/`
- Bridge implementation: `chromium/src/chrome/browser/cbf/bridge`
- Exported patch queue: `chromium/patches/cbf`

## 2. Build and test targets

You should use `autoninja` to build Chromium targets from `chromium/src`:

```bash
autoninja -C out/Default chrome
autoninja -C out/Default cbf_bridge
```

From the repository root, you can also execute the following command:  
This command will automatically add `./depot_tools` to `PATH`.

```bash
uv run tool build -t chrome -t cbf_bridge

# If you activated venv with uv, you can also use:
python3 tool build -t chrome -t cbf_bridge
```

These targets are relevant for CBF development:

- `chrome`: Chromium-fork build target with CBF integration.
- `cbf_bridge`: CBF bridge library target. It is used from `cbf-chrome-sys`.
- `cbf_tests`, `browser_tests`, `unit_tests`: Chromium-side test targets covering different test scopes and types. See their respective BUILD.gn files for details.

## 3. Patch queue policy

Changes to Chromium are exported as patches using Git's format-patch and stored in the cbf repository.
You should maintain a clean with the following principles:

- Keep CBF-specific changes traceable in `chromium/patches/cbf`.
- Curate `chromium/src` commit stack, then export patches.
- Fold refinements with `fixup`/`squash` rather than appending noisy fix patches.

Patch-splitting principles:

- Keep one primary responsibility per patch.
- Keep dependency direction explicit in patch ordering.
- Prefer mechanical moves/refactors before behavior changes.
- Keep each patch buildable in the target Chromium output directory.
- Use short imperative English subjects for exported patches; do not use Conventional Commits in patch titles.
- When refining an existing patch, prefer `fixup` / `squash` into that patch instead of adding a follow-up fix patch.

Common tooling flow:

```bash
uv run tool apply
uv run tool export
uv run tool git <...args>
uv run tool commit -m "<message>"
```

Patch files live in `chromium/patches/cbf/`.

## 4. Runtime constraints

- CBF runtime behavior depends on `--enable-features=Cbf`.
- IPC bootstrap uses inherited endpoint + session token injected by `start_chromium`.
- `start_chromium` should be the default runtime path; manual launch is primarily for debugging.
