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

- `cbf` (browser-generic high-level Rust API)
- `cbf-chrome` (chrome-specific safe backend API)
- `cbf-chrome-sys` (FFI boundary)
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
- `fix(chrome-sys): handle missing bridge library path`
- `refactor(bridge): move callback ownership to WeakPtr`
- `chore(chromium): repin fork patch for Mxx`

### Allowed scopes

Core scopes:

- `cbf`
- `chrome` (`cbf-chrome` crate changes)
- `chrome-sys`
- `bridge`
- `chromium` (`chromium/src` patch updates and fork changes)

Use `chromium` for exported patch queue updates from `chromium/src`, even when the tracked
change exists to support bridge behavior. Use `bridge` when the primary change is in
`cbf_bridge` code rather than the Chromium fork patch itself.

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

## 6. Issue Label Policy

CBF uses a small, explicit label taxonomy to keep triage consistent.

### Label groups

- `type/*`: what kind of work this is (`type/bug`, `type/feature`, `type/docs`, ...)
- `area/*`: which layer is affected (`area/cbf`, `area/chrome`, `area/chrome-sys`, `area/bridge`, ...)
- `priority/*`: urgency (`priority/p0` to `priority/p3`)
- `status/*`: current workflow state (`status/needs-triage`, `status/in-progress`, ...)
- OSS onboarding labels: `good first issue`, `help wanted`

### Usage rules

- Apply exactly one `type/*` label.
- Apply exactly one `priority/*` label.
- Apply exactly one `status/*` label.
- Apply one or more `area/*` labels as needed.
- Use `good first issue` only when scope, reproduction, and expected change are clear.
- Keep GitHub's default overlapping labels removed when they duplicate this taxonomy
  (for example `documentation`, `enhancement`, or `question`).
- Use GitHub Discussions for open-ended questions or support requests instead of issue labels.

## 7. Licensing and Notices

By contributing, you agree your changes are licensed under this repository's license policy.
If your change affects redistribution or third-party components, update notice artifacts/policy docs as needed.
