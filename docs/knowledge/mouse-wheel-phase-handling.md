# Mouse Wheel Phase Handling in CBF (macOS)

## 1. Scope

This note records a CBF-specific issue around wheel scrolling on macOS and the
final fix direction.

Focus:

- Why wheel routing became unstable after scrollbar interactions.
- Why two Chromium DCHECKs were triggered in sequence.
- Which layer should own wheel phase synthesis for long-term maintenance.

## 2. Observed Failures

### 2.1 Wheel queue DCHECK

After some wheel paths, Chromium crashed at:

- `components/input/mouse_wheel_event_queue.cc`
- Condition:
  - `event.phase != kPhaseNone || event.momentum_phase != kPhaseNone`

Meaning: a wheel event with no phase information reached `MouseWheelEventQueue`,
which assumes at least one phase bit is set.

### 2.2 Inertial scroll DCHECK

A follow-up workaround then crashed at:

- `cc/input/input_handler.cc`
- Condition:
  - `DCHECK(!scroll_state.is_in_inertial_phase())`

Meaning: a non-inertial path was fed an inertial (`momentum`) update.

## 3. Root Cause

CBF wheel input was sent via `CbfProfileService::SendMouseWheelEvent` and
directly forwarded to `RenderWidgetHostImpl::ForwardWheelEventWithLatencyInfo`.

In this path, CBF bypasses Chromium's usual per-view wheel phase normalization
(for example, logic similar to `MouseWheelPhaseHandler` usage in standard RWHV
event entrypoints). Some mouse wheel events can arrive without explicit
`phase/momentum_phase`, so phase invariants were not consistently enforced.

## 4. Why the First Workaround Failed

A temporary bridge-side fix set:

- `momentum_phase = kPhaseEnded` when both phases were `kPhaseNone`.

This satisfied `MouseWheelEventQueue`'s non-none phase DCHECK, but it also made
the event look like momentum/inertial input, which later violated compositor
assumptions and triggered the inertial DCHECK.

Key lesson:

- Do not synthesize momentum semantics for plain wheel input just to satisfy a
  queue precondition.

## 5. Final Fix Implemented

### 5.1 Keep bridge conversion passive

`cbf_bridge_mac.mm` now forwards Chromium-converted NSEvent values without extra
phase mutation.

### 5.2 Synthesize non-momentum phases in Chromium-side CBF service

`CbfProfileService` now handles only the missing-phase case:

- If both `phase` and `momentum_phase` are none:
  - first event in sequence: `phase = kPhaseBegan`
  - subsequent events: `phase = kPhaseChanged`
  - always force `momentum_phase = kPhaseNone`
- A short timer emits a synthetic end event:
  - `phase = kPhaseEnded`
  - zero deltas/ticks
  - `momentum_phase = kPhaseNone`
- State is tracked per `WebPageId` and cleaned on page close.

This satisfies wheel queue invariants without turning the sequence into inertial
scrolling.

## 6. Layering Guidance

For CBF architecture:

- `cbf` (high-level Rust) should not implement Chromium wheel phase logic.
- Chromium-specific event semantics should stay in Chromium side or `cbf-sys`
  boundary code.
- If synthesis is unavoidable, keep it semantic-minimal and avoid inventing
  momentum/inertial state unless truly present.

## 7. Practical Validation Checklist

When changing wheel handling, validate at least:

1. Mouse wheel continues to work after scrollbar drag/click interactions.
2. No `MouseWheelEventQueue` phase DCHECK.
3. No inertial-phase DCHECK in compositor/input handler paths.
4. Trackpad momentum behavior remains natural (no premature termination).
