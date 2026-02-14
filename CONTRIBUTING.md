# Contributing to CBF

Thanks for contributing.

## 1. Before You Start

Read these first:

- `docs/setup-guide.md`
- `docs/user-setup-guide.md`
- `docs/developer-setup-guide.md`
- `docs/chromium-fork-guide.md`
- `docs/implementation-guide.md`
- `docs/licensing.md`

## 2. Scope of Contributions

Main areas:

- `cbf` (high-level Rust API)
- `cbf-sys` (FFI boundary)
- Chromium bridge/fork (`cbf_bridge`, Chromium-side integration)
- Documentation and tests

## 3. Required Technical Rules

When changing bridge/fork/boundary behavior, follow `docs/implementation-guide.md`:

- No raw pointer ownership across async boundaries.
- Prefer `WebPageId` re-resolution and weak ownership guards.
- Keep Chromium/Mojo details out of public `cbf` API.
- Treat disconnect/crash paths as normal outcomes.

## 4. Commit Message Convention

Use Conventional Commits:

`<type>(<scope>): <subject>`

Examples:

- `feat(cbf): add browser event for ...`
- `fix(sys): handle missing bridge library path`
- `refactor(bridge): move callback ownership to WeakPtr`
- `chore(chrome): repin fork patch for Mxx`

### Allowed scopes

Core scopes:

- `cbf`
- `sys`
- `bridge`
- `chrome`

Additional scopes for non-runtime changes:

- `docs`
- `ci`
- `build`
- `release`

## 5. Pull Request Expectations

- Keep changes focused and small.
- Explain behavior changes and risk in the PR description.
- Add or update tests for behavior changes when feasible.
- Update affected docs in the same PR.
- If Chromium fork behavior changes, mention patch/revision impact clearly.

## 6. Licensing and Notices

By contributing, you agree your changes are licensed under this repository's license policy.
If your change affects redistribution or third-party components, update notice artifacts/policy docs as needed.
