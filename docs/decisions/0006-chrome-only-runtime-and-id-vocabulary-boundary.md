# ADR 0006: Chrome-Only Runtime First and ID Vocabulary Boundary

- Status: Accepted
- Date: 2026-02-27

## Context

CBF needs to support Chrome-layer capabilities that often assume Browser/Tab ownership semantics (for example `chrome://settings` and `chrome://history` paths).

Recent issue planning established a new execution direction:

- Epic: #38 (`epic(chrome): adopt chrome-only runtime with Browser-backed integration`)
- Key child issues: #39, #40, #41, #42, #43, #44, #45
- Existing tasks re-parented for this direction: #36, #37

At the same time, CBF keeps a strict layered API boundary:

- Public `cbf` API must remain browser-generic.
- Chromium internals must not leak above `cbf-chrome-sys`.
- Dependency direction remains:
  `Application -> cbf -> cbf-chrome -> cbf-chrome-sys -> Chromium`

Historically, chrome-facing layers used legacy `WebPageId` terminology. This predated the current `BrowsingContextId` vocabulary in `cbf` and causes conceptual drift.

## Decision

CBF adopts a **chrome-only runtime-first** direction for current implementation work.

Concretely:

- Runtime adoption work will target Browser-backed chrome runtime assumptions first.
- Alloy runtime implementation is deferred and will be decided separately in the future.
- Identifier vocabulary is split by layer boundary:
  - `cbf` public layer: `BrowsingContextId`
  - chrome-facing layers (`cbf-chrome`, `cbf-chrome-sys`, `cbf_bridge`, Chromium CBF runtime): `TabId`
- Legacy `WebPageId` naming in chrome-facing layers will be migrated to `TabId` (tracked by #40).

## Consequences

### Positive

- Aligns implementation vocabulary with Chromium Browser/Tab mental model in chrome-facing layers.
- Reduces confusion between generic API IDs and Chromium runtime IDs.
- Improves consistency for Browser/Tab assumption fixes required by settings/history work.
- Keeps `cbf` public API browser-generic.

### Negative / Trade-offs

- Large, potentially breaking rename surface in bridge/FFI/runtime code.
- Short-term migration overhead for tests, tooling, and docs.
- Alloy runtime remains intentionally unimplemented for now.

## Alternatives Considered

### A. Keep WebContents-first approach and continue ad-hoc compatibility guards

- Not selected as primary direction due to recurring Browser/Tab assumption gaps.

### B. Implement chrome-only and Alloy runtimes in parallel now

- Not selected due to scope expansion and reduced delivery focus.

### C. Keep legacy `WebPageId` naming in chrome-facing layers

- Not selected because it preserves vocabulary drift and onboarding friction.

## Notes

- This ADR captures architecture and naming direction, not final class/file names.
- This ADR does not change `cbf` public naming (`BrowsingContextId`).
- Existing ADRs remain historical context; this ADR governs the current runtime-first execution track tied to #38.

## Follow-ups

- Execute #39 (ownership model) and #41 (ADR/doc alignment).
- Execute #40 (`WebPageId -> TabId` migration) before deeper runtime integration tasks.
- Execute #42 and #43 to satisfy Browser/Tab assumptions for target flows.
- Validate via #44 regression suite.
- Introduce runtime selection gate via #45 (chrome-only default, Alloy deferred).
