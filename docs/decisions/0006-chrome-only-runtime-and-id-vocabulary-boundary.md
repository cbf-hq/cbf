# ADR 0006: Browser-Backed Ownership Model and ID Boundary for Chrome-Only Runtime

- Status: Accepted
- Date: 2026-02-27

## Context

CBF must support Chrome-layer capabilities that assume Browser/Tab ownership
semantics (for example `chrome://settings` and `chrome://history` paths).

Issue planning established a chrome-only Browser-backed direction:

- Epic: #38 (`epic(chrome): adopt chrome-only runtime with Browser-backed integration`)
- Design issue: #39
- Related implementation issues: #40, #41, #42, #43, #44, #45
- Existing feature tasks in scope: #36, #37

At the same time, CBF keeps a strict layered API boundary:

- Public `cbf` API must remain browser-generic.
- Chromium/Mojo internals must not leak above `cbf-chrome-sys`.
- Dependency direction remains:
  `Application -> cbf -> cbf-chrome -> cbf-chrome-sys -> Chromium`

Historically, chrome-facing layers used legacy `WebPageId` naming.
This now conflicts with Browser/Tab runtime assumptions and with the explicit
vocabulary boundary required by #38.

ADR 0004 documented a WebContents-first ownership direction. The new #38/#39
direction requires an explicit update for ownership and lifecycle semantics.

## Decision

CBF adopts a Browser-backed ownership model for the chrome-only runtime track.

Concretely:

1. Ownership model
   - Chrome runtime ownership is `Browser -> Tab (WebContents-backed)`.
   - `cbf` does not own Chromium objects; it operates on stable logical IDs.
   - `WebContents` ownership remains in Chromium process space.

2. ID boundary and mapping
   - `cbf` public layer uses `BrowsingContextId`.
   - Chrome-facing layers (`cbf-chrome`, `cbf-chrome-sys`, bridge, Chromium CBF runtime) use `TabId`.
   - `BrowsingContextId <-> TabId` is treated as a stable 1:1 logical mapping.
   - Conversion responsibility is localized at chrome boundary conversion points,
     not spread across public `cbf` API code.
   - Legacy `WebPageId` naming is replaced by `TabId` in chrome-facing layers (#40).

3. Lifecycle model
   - `create`: allocate/resolve Browser-owned tab and emit created signal only
     after `TabId` is established.
   - `navigate`: execute against resolved `TabId`; if resolution fails, no-op safely.
   - `close`: duplicate and late close operations are valid and must no-op safely
     when target tab no longer exists.
   - `shutdown`: close/shutdown races are expected; operations must be idempotent
     and deterministic.

4. Async/failure safety model
   - Never carry raw `WebContents*` or owning `this` across async boundaries.
   - Use `ID + re-resolve` at execution time.
   - Guard async callbacks with weak ownership (`WeakPtr` pattern).
   - Use `DCHECK + guard return` on race-prone paths instead of `CHECK`.
   - Treat disconnect/crash/late callback as normal runtime outcomes.

5. Rollout ordering is fixed
   - #41 (ADR/doc alignment)
   - #40 (`WebPageId -> TabId` migration)
   - #42 (Browser-backed lifecycle scaffolding)
   - #43 (BrowserWindowInterface/TabInterface integration)
   - #44 (regression suite)
   - #45 (runtime selection gate: chrome-only default)

## Consequences

### Positive

- Removes ambiguity in Browser/Tab ownership responsibilities.
- Gives implementation issues a fixed lifecycle and race-handling contract.
- Preserves browser-generic public API while allowing chrome-specific internals.
- Improves traceability of ID mapping and failure behavior in tests and code reviews.

### Negative / Trade-offs

- Requires broad rename and contract updates across chrome-facing layers.
- Introduces temporary complexity while old/new assumptions coexist during migration.
- ADR 0004 ownership direction is no longer fully aligned and must be interpreted
  with this ADR's supersede note.

## Alternatives Considered

### A. Keep WebContents-first ownership and continue compatibility guards

- Not selected due to recurring Browser/Tab assumption gaps and crash-prone paths.

### B. Defer ownership decision and decide during implementation

- Not selected because it pushes high-impact decisions into implementation issues.

### C. Keep legacy `WebPageId` naming

- Not selected because it keeps vocabulary drift and boundary ambiguity.

## Notes

- This ADR supersedes ADR 0004 **partially** for ownership/lifecycle direction:
  Browser-backed ownership in this ADR replaces ADR 0004's WebContents-first
  ownership guidance.
- ADR 0004 remains valid as historical context for capability wiring themes that
  do not conflict with this ownership decision.
- This ADR does not change `cbf` public naming (`BrowsingContextId`) or expose
  Chromium internals above `cbf-chrome-sys`.
- Alloy runtime design remains explicitly out of scope for this ADR.
- Browser-backed ownership enables Chrome pages such as `chrome://settings`,
  but support level and user-facing recommendation remain capability-specific
  and are not implied by this ADR alone.

## Follow-ups

1. Update ADR references and wording in #41 to reflect this ownership model.
2. Complete `WebPageId -> TabId` migration in #40 across chrome-facing layers.
3. Implement lifecycle scaffolding with idempotent close/shutdown behavior in #42.
4. Add BrowserWindowInterface/TabInterface integration in #43.
5. Add regression suite for ID consistency and race/late-callback paths in #44.
6. Introduce runtime selection gate in #45 with chrome-only default.
