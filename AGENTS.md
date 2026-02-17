# AGENTS.md

## Project

CBF (Chromium Browser Framework) is a reusable browser backend framework for Rust.
This repository is for framework development, not product-specific app development.

## Language Rules

- Conversation with maintainers: use the first language the maintainer used in the thread
- Code comments and docs: English preferred (unless requested otherwise)
- Commit messages: English

## Scope

This repository owns:

- High-level API crate (`cbf`)
- Low-level FFI crate (`cbf-sys`)
- Chromium bridge integration and related patches
- Reliability of async IPC, lifecycle, and crash behavior

This repository does not own:

- Product-specific domain logic (workspace, knowledge, pane, etc.)
- App-specific UI/UX concerns

## Architecture (Must Keep)

Dependency direction:

`Application -> cbf -> cbf-sys -> Chromium process`

Hard rules:

1. Public `cbf` API must remain browser-generic.
2. Chromium/Mojo internals must not leak into public API.
3. C ABI/FFI contracts must stay in `cbf-sys`.
4. WebContents ownership stays in Chromium process.
5. Rust side uses stable logical IDs (`WebPageId`) across boundaries.

## API Design Rules

- Model upstream requests as `BrowserCommand`.
- Model backend facts as `BrowserEvent`.
- Prefer additive changes over breaking changes.
- Avoid unnecessary `pub` surface growth.
- Keep failure and lifecycle explicit in API shape.

## IPC and Async Safety Rules

Required for bridge/Chromium-facing changes:

1. Never carry raw `WebContents*` or owning `this` across async boundaries.
2. Use `ID + re-resolve` at task execution time.
3. Guard async callbacks with weak ownership (`WeakPtr` pattern).
4. If re-resolution fails, no-op safely instead of crashing.
5. Use `DCHECK` instead of `CHECK` to prevent crash in production.
6. Shutdown/close race paths must tolerate duplicate or late operations.

## Failure Model

Treat as normal (not exceptional):

- backend not running
- disconnects/timeouts
- protocol drift/mismatch
- renderer/browser crashes

Expose failures as events/errors so upstream can choose recovery.

## Build and Test Quick Commands

Rust:

- `cargo check -p cbf`
- `cargo check -p cbf-sys`
- `cargo test -p cbf`
- `cargo test -p cbf-sys`

Chromium side (from `chromium/src`):

- `autoninja -C out/Default chrome`
- `autoninja -C out/Default cbf_bridge`

Tooling helpers:

- `uv run tool`: access development tools.
- `uv run tool patch`: access patch-related operations.
- `uv run tool patch build -t <target>`: build a specific target.
- `uv run tool patch build -t chrome -t cbf_bridge`: build both targets sequentially.

## Setup and Docs Routing

- Start from `docs/setup-guide.md` (overview), then choose:
  - `docs/user-setup-guide.md` for prebuilt artifact users
  - `docs/developer-setup-guide.md` for contributors
- Fork-specific layout/policy: `docs/chromium-fork-guide.md`
- Bridge/FFI invariants: `docs/implementation-guide.md`
- Contribution rules and commit convention: `CONTRIBUTING.md`

## Bridge Runtime and Linking Rules

- `cbf-sys` link path is configured by `CBF_BRIDGE_LIB_DIR`.
- Prefer project-local config in `.cargo/config.toml`:
  - `[env]`
  - `CBF_BRIDGE_LIB_DIR = "/path/to/cbf_bridge/libdir"`
- Default runtime path should use `start_chromium` rather than manual browser flags.
- `ChromiumOptions.executable_path` should point to CBF Chromium-fork binaries, not stock Chromium.
- Prefer setting `ChromiumOptions.user_data_dir` explicitly to avoid profile conflicts/corruption risk.

## `depot_tools` Policy

- Do not vendor `depot_tools` in this repository.
- Do not add `depot_tools` as a git submodule.
- Use external checkout and pin tested revision in CI/docs.

## Chromium Debug Launch Flags

Typical flags:

- `--enable-logging=stderr`
- `--log-file=/tmp/chromium_debug.log`

## Licensing Policy

- CBF-authored code: BSD 3-Clause
- Chromium/third-party artifacts: each upstream license applies
- Redistribution must include required notices

## Agent Workflow

1. Read relevant docs before editing (`README`, architecture/setup/licensing docs).
2. Keep changes minimal and layer-correct.
3. Update docs when behavior, API, or build flow changes.
4. Run focused checks for touched areas.
5. Report known risks, regressions, and follow-up tasks explicitly.

## Commit Convention

Use Conventional Commits: `<type>(<scope>): <subject>`.

Core scopes:

- `cbf`
- `sys`
- `bridge`
- `chrome`

## PR Checklist

- [ ] No product domain terms leaked into public `cbf` API
- [ ] No FFI concern leaked above `cbf-sys`
- [ ] Async lifetime safety rules respected
- [ ] Failure paths (disconnect/crash/timeout) validated
- [ ] Documentation updated when needed
- [ ] Licensing/notice impact considered for distribution changes
