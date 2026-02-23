# ADR 0003: Chrome Runtime Default and Embedded Scope Boundary

- Status: Accepted
- Date: 2026-02-21

## Context

CBF must provide an embedded browser backend that can reliably leverage Chrome-layer capabilities such as extension support and PDF/printing behavior while keeping the public `cbf` API browser-generic.

Historically, the current Chromium fork side accumulated substantial logic in `chrome/browser/cbf/cbf_profile_service.cc`, creating high coupling and low maintainability.
The repository currently also carries a monolithic exported patch under `chromium/patches/cbf/`, which increases review and rebase cost.

At the same time, CBF product scope is explicitly embedded-first:

- CBF provides web page rendering, extension support, and popup support.
- Integrators provide application UI.
- Chrome Views shell UI (tab strip, omnibox/URL bar, toolbar, settings shell) is out of scope.

Reference:

- `CHROME_RUNTIME_SCOPE_MATRIX.md`
- `chromium/src/chrome/browser/cbf/cbf_profile_service.cc`
- `chromium/patches/cbf/`

## Decision

CBF adopts Chrome Runtime (chrome layer) as the default runtime direction for Chromium integration.

CBF scope is fixed as follows:

- In scope: embedded page surface, navigation primitives, popup lifecycle, extension runtime path, failure/lifecycle eventing.
- Out of scope: general Views abstraction and Chrome shell UI surfaces (tab strip, omnibox, toolbar/menu/settings shell).

Architecture and layering constraints remain unchanged:

- Dependency direction stays `Application -> cbf -> cbf-chrome -> cbf-chrome-sys -> Chromium`.
- Public `cbf` API remains browser-generic.
- Chromium/Mojo internals do not leak above `cbf-chrome-sys`.

Implementation direction for refactoring:

- Reduce `cbf_profile_service` to Mojo entrypoint and orchestration responsibilities.
- Move feature logic into focused components (navigation, popup, extension, input/IME/drag, context menu, devtools, conversion helpers).
- Keep content-layer dependent bridging minimal and explicit.

Patch management direction:

- Replace monolithic CBF Chromium patch with responsibility-based patch series.
- Enforce dependency-ordered, buildable patch steps with clear purpose per patch.

## Consequences

### Positive

- Better access to Chrome-layer capabilities with lower long-term feature gap risk.
- Clear embedded scope boundary between CBF core and integrator-owned UI.
- Reduced maintenance risk from smaller, responsibility-focused source files.
- Easier patch review, bisect, and rebase via split patch series.

### Negative / Trade-offs

- Migration requires temporary dual-path complexity during refactoring.
- Chrome Runtime-first approach increases reliance on Chrome-layer behavior changes.
- Initial engineering cost is non-trivial (file moves, API wiring, test updates, patch reorganization).

## Alternatives Considered

### A. Keep content-layer-first implementation and selectively backport chrome features

- Not selected due to recurring maintenance burden and slower access to advanced Chrome-layer capabilities.

### B. Provide CBF-owned Views abstraction including tab strip/omnibox shell

- Not selected because CBF is embedded-first and integrators should own app UI composition.

### C. Keep current monolithic `cbf_profile_service.cc` and single exported patch

- Not selected due to poor maintainability, high review/rebase friction, and weak fault isolation.

## Notes

- This ADR sets architecture direction and scope boundaries, not final class/file names.
- Chromium-native error page behavior remains default unless explicitly overridden by integrator policy.
- Async/lifecycle safety invariants remain mandatory (`ID + re-resolve`, weak ownership, safe no-op on stale targets).

## Follow-ups

- Refactor `cbf_profile_service` into orchestration + focused subcomponents with explicit boundaries.
- Introduce `mojo` conversion/helper files and remove broad converter logic from service monolith.
- Split `chromium/patches/cbf/0001-...patch` into dependency-ordered patch series.
- Add/adjust targeted tests for popup lifecycle, extension flows, failure paths, and shutdown races.
- Create implementation ADR(s) or design notes for concrete component boundaries and migration sequence if needed.
