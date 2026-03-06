# AGENTS.md

## Project

CBF (Chromium Browser Framework) — reusable browser backend framework for Rust.
Framework development only; not product-specific. Currently private; breaking changes are acceptable.

**Language:** conversations follow first-used language; code comments, docs, and commits in English.

**Scope:** owns `cbf` (browser-generic API), `cbf-chrome` (Chrome backend), `cbf-chrome-sys` (FFI),
Chromium bridge/patches, async IPC/lifecycle reliability. Does not own product domain logic or app UI/UX.

## Architecture (Must Keep)

```
Application -> cbf + cbf-chrome
cbf-chrome  -> cbf
cbf-chrome  -> cbf-chrome-sys <-- IPC --> Chromium process
```

Hard rules:

1. Public `cbf` API must remain browser-generic (no Chromium/Mojo internals).
2. C ABI/FFI contracts must stay in `cbf-chrome-sys`.
3. WebContents/Tab ownership stays in the Chromium process.
4. Rust side uses stable logical IDs (`WebPageId`, `BrowsingContextId`) across boundaries.
5. `BrowsingContextId <-> TabId` mapping is resolved at boundary conversion points only.

## API Design Rules

- `BrowserCommand` for upstream requests; `BrowserEvent` for backend facts.
- Prefer additive changes; avoid unnecessary `pub` surface growth.
- Keep failure/lifecycle explicit; keep `cbf` vocabulary browser-generic.

## IPC and Async Safety (Bridge / Chromium-facing changes)

1. Never carry raw pointer or owning `this` across async boundaries.
2. Capture stable IDs; resolve platform objects at task execution time.
3. Guard callbacks with `WeakPtr`; safe no-op if re-resolution fails.
4. `DCHECK` (not `CHECK`) on expected lifecycle races.
5. Shutdown/close races must tolerate duplicate or late operations.
6. Bind each `mojo::Remote` on its intended sequence.

`base::Unretained` is acceptable only as temporary technical debt with justification + follow-up plan.

See [`docs/developer-guide/chromium-integration-rules.md`](docs/developer-guide/chromium-integration-rules.md)
and [`docs/developer-guide/chromium-implementation-guide.md`](docs/developer-guide/chromium-implementation-guide.md).

## Failure Model

Normal outcomes (not exceptional): backend not running, disconnects/timeouts, protocol mismatch, crashes.
Expose all failures as events/errors so upstream can choose recovery.

## Build and Test Quick Commands

Rust: `cargo check/test -p <crate>` for `cbf`, `cbf-chrome`, `cbf-chrome-sys`.

Chromium-side — use exactly one of:
- `uv run tool build -t <target>` (from repo root; auto-adds `./depot_tools` to PATH)
- `source depot_tools.sh` → `autoninja -C out/Default <target>`

Do not use `ninja` directly, as it will break the cached build state.

Common targets: `chrome`, `cbf_bridge`, `cbf_tests`, `browser_tests`, `unit_tests`.

Tooling helpers (from repo root):
- `uv run tool build -t chrome -t cbf_bridge`
- `uv run tool apply` / `export` / `git <args>` / `commit -m "<msg>"`

## Chromium Runtime and Linking

- Set `CBF_BRIDGE_LIB_DIR` (env or `.cargo/config.toml`) to the directory containing the bridge library.
- `executable_path` must point to CBF Chromium-fork binaries, not stock Chromium.
- Use `start_chromium` as the default runtime path; manual launch is for debugging only.
- `--enable-features=Cbf`, `--cbf-ipc-handle`, `--cbf-session-token` are injected automatically by `start_chromium`.
- `depot_tools` lives at `./depot_tools`; do not add as a git submodule.
- CBF Chromium code: `chromium/src/chrome/browser/cbf/`; patch queue: `chromium/patches/cbf/`.
- Patch principles: one responsibility per patch, keep each buildable, fold fixes with `fixup`/`squash`,
  use short imperative English subjects (no Conventional Commits in patch titles).
- Debug flags: `--enable-logging=stderr`, `--log-file=/tmp/chromium_debug.log`.

## Licensing

- CBF-authored code: BSD 3-Clause. Chromium/third-party: each upstream license applies.
- Redistribution must include required notices. See [`docs/developer-guide/licensing.md`](docs/developer-guide/licensing.md).

## Docs Routing

| Topic | Path |
|---|---|
| User setup (prebuilt) | `docs/getting-started/user-setup.md` |
| Contributor setup | `docs/developer-guide/contributor-setup.md` |
| Fork workflow & patch queue | `docs/developer-guide/chromium-fork-workflow.md` |
| Boundary invariants & policy | `docs/developer-guide/chromium-integration-rules.md` |
| Bridge/FFI implementation | `docs/developer-guide/chromium-implementation-guide.md` |
| Licensing | `docs/developer-guide/licensing.md` |
| Contribution rules & commit convention | `CONTRIBUTING.md` |

## Agent Workflow

1. Read relevant docs before editing.
2. Keep changes minimal and layer-correct (respect dependency direction).
3. Update docs when behavior, API, or build flow changes.
4. Run focused checks for touched areas.
5. Report known risks, regressions, and follow-up tasks explicitly.

## Commit Convention

`<type>(<scope>): <subject>` — core scopes: `cbf`, `chrome`, `chrome-sys`, `bridge`, `chromium`;
additional: `docs`, `ci`, `build`, `release`.

## PR / Commit Checklist

- [ ] No product domain terms in public `cbf` API
- [ ] No FFI concern above `cbf-chrome-sys`
- [ ] Async lifetime safety rules respected
- [ ] Failure paths (disconnect/crash/timeout) validated
- [ ] Documentation updated; licensing/notice impact considered

1. Read relevant docs before editing (`README`, architecture/setup/licensing docs).
2. Keep changes minimal and layer-correct.
3. Update docs when behavior, API, or build flow changes.
4. Run focused checks for touched areas.
5. Report known risks, regressions, and follow-up tasks explicitly.

