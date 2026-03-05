# ADR 0001: Layered API for Browser-Generic and Chromium-Specific Surfaces

- Status: Accepted
- Date: 2026-02-19

## Context

CBF currently exposes some Chromium-shaped vocabulary in the high-level crate (`cbf`), especially around input and drag-and-drop data modeling.
Examples can be found in:

- `crates/cbf/src/data/key.rs`
- `crates/cbf/src/data/mouse.rs`
- `crates/cbf/src/data/ime.rs`
- `docs/knowledge/drag-data-field-mapping.md`

This creates an ambiguous API boundary: users are told `cbf` is browser-generic, but parts of the surface are effectively chrome-specific.
As drag-related features expand, this ambiguity is expected to grow and increase cognitive load for API consumers.

At the same time, we still need an explicit escape hatch for chrome-specific commands/events, without forcing users to maintain separate event loops for generic vs native paths.

## Decision

We split responsibilities into three layers:

- `cbf`: browser-generic safe API (no Chromium vocabulary in public data model)
- `cbf-chrome`: chrome-specific safe API and backend implementation
- `cbf-chrome-sys`: unsafe FFI/wire boundary to `cbf_bridge` (dylib)

Dependency direction is:

- `cbf` (no dependency on `cbf-chrome` or `cbf-chrome-sys`)
- `cbf-chrome -> cbf + cbf-chrome-sys`
- `cbf-chrome-sys -> cbf_bridge`

For command/event handling, we adopt a single-stream model in `cbf` with explicit raw opt-in:

- `CommandSender::send(...)` remains the default generic path
- `RawCommandSenderExt::send_raw(...)` is provided as explicit raw API
- `EventStream::recv(...)` returns `OpaqueEvent<B>`
- `OpaqueEvent::as_generic()` is the default interpretation path
- `RawOpaqueEventExt::as_raw()` is extension API for explicit raw access

Design constraints:

- Raw APIs are explicitly marked/isolated (for example by extension traits and/or module/feature gating) so generic-first discovery remains the default.
- Raw types are backend-native contract types (`RawCommand` / `RawEvent`) and must not grow into additional unbounded raw type families without a new ADR.
- The `Backend` trait uses `type RawCommand;` / `type RawEvent;` and conversion methods are named `to_raw_command` and `to_generic_event`.
- The `Backend` trait also carries `type RawDelegate;`, and `connect` accepts `raw_delegate: Option<Self::RawDelegate>` during the current migration stage to keep backend setup shape consistent across implementations.
- Delegate hooks are policy-only (forward/drop/stop); they must not rewrite command/event payloads.
- Delegate dispatch is decision-first: dispatcher methods return decisions, and forward paths are executed immediately by the caller.
- `flush` is queue-drain only (`BrowserCommand` extraction), and backend implementations own transport execution/emit ordering.
- Raw stream contracts stay raw-only (for example `ChromeEvent` does not carry a `Generic` wrapper variant).
- `cbf` public API must not introduce chrome-specific nouns or Chromium internal concepts.
- Prompt UI vocabulary boundary is explicit:
  - `cbf` public surface keeps browser-generic `AuxiliaryWindow*` terms.
  - `cbf-chrome` raw/internal event and command vocabularies use chrome-specific `PromptUi*` terms.
  - `cbf-chrome-sys` FFI/wire contracts use `PromptUi*` terms only.
  - Vocabulary translation is owned by `cbf-chrome` conversion boundaries (`to_generic_event` and command mapping).

## Consequences

### Positive

- Clarifies layer boundaries: browser-generic in `cbf`, chrome-specific in `cbf-chrome`, unsafe wire in `cbf-chrome-sys`.
- Reduces user confusion about where chrome-specific behavior should be implemented.
- Keeps one receive flow (`OpaqueEvent`) while still allowing explicit raw handling.
- Improves long-term extensibility for additional backends (`cbf-firefox`, `cbf-webkit`, etc.) without changing `cbf`’s generic contract.

### Negative / Trade-offs

- `cbf` still exposes raw entry points (`send_raw`, `as_raw` via extension traits), so neutrality relies on disciplined API placement and documentation.
- Additional crate split increases maintenance overhead (versioning, CI matrix, docs synchronization).
- Conversion and mapping logic becomes more explicit and may require more boilerplate in `cbf-chrome`.

## Alternatives Considered

### A. Keep current mixed surface in `cbf`

- Rejected because chrome-specific vocabulary leakage would continue to increase and make API semantics less predictable.

### B. Strict separation with no raw access in `cbf` (raw only in `cbf-chrome`)

- Not selected for now because it complicates consumer event handling with dual paths and can require extra async coordination.

### C. Make `cbf` depend on `cbf-chrome`

- Rejected because it breaks `cbf` neutrality and couples browser-generic API evolution to chrome-specific concerns.

## Notes

- This ADR defines architecture and API boundary policy; it does not define the full migration plan for each existing type.
- Legacy `cbf-sys` responsibilities are now owned by `cbf-chrome-sys`.
- Any future expansion of raw contracts beyond `RawCommand` / `RawEvent` requires explicit reassessment.
- The `connect(..., raw_delegate: Option<...>)` form is a staged decision for experimentation in this non-public phase. Re-evaluate when:
  - optional-argument noise becomes a repeated ergonomics issue, or
  - additional connection parameters are introduced.
  At that point, prefer either `connect_with_raw_delegate(...)` split APIs or a `ConnectOptions` object.
- Concrete API sketch for implementation is documented in:
  - `docs/decisions/0001-api-design-sketch.md`
  - The sketch is based on the direction discussed from `NEW_ARCH_STUB.rs`, with ADR-aligned naming and `OpaqueEvent` flow.

## Follow-ups

- Introduce new crates: `cbf-chrome` and `cbf-chrome-sys`.
- Implement the API sketch in `docs/decisions/0001-api-design-sketch.md` incrementally, starting from command/event transport boundaries.
- Define and document `OpaqueEvent`, `as_generic`, `RawCommandSenderExt::send_raw`, and `RawOpaqueEventExt::as_raw` APIs.
- Move chrome-specific data vocabulary out of `cbf` models where applicable (starting with key/mouse/IME/drag areas).
- Add CI/docs guardrails to prevent chrome-specific terms from leaking into `cbf` public API.
- Update setup/architecture documentation to reflect new dependency graph and crate responsibilities.
