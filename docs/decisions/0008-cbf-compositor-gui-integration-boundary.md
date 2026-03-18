# ADR 0008: cbf-compositor as the GUI Composition Integration Layer

- Status: Accepted
- Date: 2026-03-03

## Context

Issue #46 proposes a host-managed embedded browsing-context model so host
applications can compose browser UI and page content inside Chromium-managed
content instead of depending on native WebView overlays and platform-specific
focus / hit-testing workarounds.

That work needs a crate that helps host applications integrate CBF into native
GUI windows while preserving CBF's existing event-driven model and public API
boundaries.

The desired developer experience is:

- host applications can attach CBF-managed frames to a native GUI window
- users do not need to manage platform-specific surface embedding and focus
  details directly
- multiple pages can be composed in one window without imposing browser UI such
  as tabs or an address bar

At the same time, this integration layer must not turn into a Tauri-like
application runtime:

- it must not provide general JS-side host capabilities such as database access
- it must not hide `cbf` behind a replacement `WebView`-style abstraction
- it must not own browser connection setup, command sending, or event loop
  policy

There is also a naming concern. `cbf-kit` could later be used for a broader
high-level package, so the GUI composition layer should use a narrower name now.

## Decision

CBF will introduce a new crate named `cbf-compositor`.

`cbf-compositor` is the GUI composition integration layer for native desktop
applications that use `cbf` together with `cbf-chrome`.

Its responsibilities are:

- accept host-created native windows through a `WindowHost` abstraction built
  on `raw-window-handle`
- require a narrow host trait instead of a crate-specific window enum so
  adapters can be provided for multiple GUI runtimes
- provide a `winit` feature that implements `WindowHost` for
  `winit::window::Window`
- manage frame composition for a window, including frame creation requests,
  placement, visibility, movement, resize, and surface attachment state
- absorb platform-specific surface embedding and focus-routing details required
  to present CBF content in native GUI windows
- emit `BrowserCommand` requests that the host application sends through its own
  `cbf` command path
- consume `BrowserEvent` and Chrome-specific surface events that the host
  application forwards into the compositor

`cbf-compositor` will not:

- own or hide the host application's `cbf` connection lifecycle
- own `EventStream`, `CommandSender`, or browser bootstrap policy
- replace `BrowsingContextId` with a new public "webview" abstraction
- provide browser chrome UI such as tabs, URL bars, or application widgets
- provide general JS-side convenience capabilities

`BrowsingContextId` may remain visible in the public API where needed, but the
crate's primary abstraction is composition state, not browser state.

`cbf-compositor` may generate `RequestId` values for emitted create requests,
but `RequestId` generation must be injectable so applications can preserve a
single request-correlation strategy across compositor-managed and non-compositor
operations, including headless flows.

The initial attachment API is top-level window attachment through
`attach_window(...)`.
Future child/subview embedding should be added as a separate API such as
`attach_window_as_child(...)`, not folded into the initial top-level attach
path.

Development ergonomics such as running a Vite dev server, file watching, and
automatic Rust restarts are out of scope for `cbf-compositor`. Those concerns
belong in tooling, such as `cbf-cli`.

## Consequences

### Positive

- CBF gains a dedicated integration layer for native GUI composition without
  widening the scope of `cbf` itself.
- The architecture stays aligned with the existing dependency direction:
  `Application -> cbf-compositor -> cbf -> cbf-chrome -> cbf-chrome-sys`.
- Host applications retain direct control over command sending, event handling,
  and mixed GUI / headless workflows.
- The crate name describes the narrow responsibility more accurately than
  `cbf-kit`.
- `cbf-kit` remains available as a future name for a broader high-level package
  if the project later needs one.

### Negative / Trade-offs

- Applications must explicitly forward events into the compositor and send
  emitted commands themselves, which is less "magic" than a fully managed
  runtime.
- Public APIs must document precisely which browser and backend events the
  compositor depends on, or integration failures will be hard to diagnose.
- Supporting multiple GUI runtime adapters over time will require a careful
  feature and compatibility story.
- The crate adds a new public integration surface that must remain small and
  well-bounded to avoid drifting into a second browser abstraction layer.
- Child/subview embedding will likely need a separate host abstraction later,
  so the top-level attachment API should stay narrowly scoped.

## Alternatives Considered

### A. Use `cbf-kit` as the crate name now

- Rejected because it suggests a broader batteries-included layer than the
  current scope and makes it harder to reserve that name for a future higher-
  level package.

### B. Put GUI composition directly into `cbf`

- Rejected because native window integration, platform surface handling, and
  composition policy are above the browser-generic API layer owned by `cbf`.

### C. Build a fully managed runtime that owns `cbf` connection and event flow

- Rejected because it would reduce host flexibility, hide existing `cbf`
  primitives unnecessarily, and blur the boundary with a Tauri-like runtime.

### D. Restrict the crate to raw surface attachment only

- Rejected because the intended value includes declarative frame composition and
  platform-level focus / placement handling, not just surface handle storage.

## Notes

- The initial host abstraction is `WindowHost`, built on
  `raw-window-handle::HasWindowHandle` and
  `raw-window-handle::HasDisplayHandle`.
- The initial convenience implementation is behind a `winit` feature and
  targets `winit::window::Window`.
- Future adapters may be added, but the abstraction should remain narrow and
  focused on native window hosting, not on full UI frameworks.
- `RequestId` injection should use a small allocator interface so the default
  path is simple while advanced applications can preserve their own request
  strategy.
- The first stable attach API should describe top-level attachment semantics.
  Child/subview attachment should remain a separate future API because it
  changes hosting and layout responsibilities.
- The API sketch below is non-normative. It illustrates the intended boundary,
  not a final stabilized interface.

```rust
use cbf::{BrowserCommand, BrowserEvent, RequestId};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use cbf_chrome::ChromeEvent;

pub struct Compositor<A = DefaultRequestIdAllocator> {
    // internal state
    request_ids: A,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CompositorWindowId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameId(u64);

pub trait WindowHost: HasWindowHandle + HasDisplayHandle {
    fn inner_size(&self) -> (u32, u32);

    fn scale_factor(&self) -> f64 {
        1.0
    }
}

pub trait RequestIdAllocator {
    fn next_request_id(&mut self) -> RequestId;
}

pub struct AttachWindowOptions {
    pub transparent: bool,
}

pub struct FrameComposition {
    pub frames: Vec<FrameSpec>,
}

pub struct FrameSpec {
    pub id: FrameId,
    pub kind: FrameKind,
    pub url: String,
    pub bounds: FrameBounds,
    pub ipc: IpcPolicy,
    pub transparency: TransparencyPolicy,
}

pub enum FrameKind {
    Ui,
    Page,
}

pub enum FrameBounds {
    FullWindow,
    Rect(Rect),
}

pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub enum IpcPolicy {
    Deny,
    Allow,
}

pub enum TransparencyPolicy {
    Opaque,
    Transparent,
}

pub enum CompositionCommand {
    SetComposition {
        window_id: CompositorWindowId,
        composition: FrameComposition,
    },
    MoveFrame {
        window_id: CompositorWindowId,
        frame_id: FrameId,
        bounds: FrameBounds,
    },
    ShowFrame {
        window_id: CompositorWindowId,
        frame_id: FrameId,
    },
    HideFrame {
        window_id: CompositorWindowId,
        frame_id: FrameId,
    },
    RemoveFrame {
        window_id: CompositorWindowId,
        frame_id: FrameId,
    },
}

impl Compositor<DefaultRequestIdAllocator> {
    pub fn new() -> Self {
        todo!()
    }
}

impl<A> Compositor<A>
where
    A: RequestIdAllocator,
{
    pub fn with_request_id_allocator(request_ids: A) -> Self {
        todo!()
    }

    pub fn attach_window(
        &mut self,
        window: impl WindowHost + 'static,
        options: AttachWindowOptions,
        emit: impl FnMut(BrowserCommand),
    ) -> Result<CompositorWindowId, CompositorError> {
        todo!()
    }

    // Future expansion:
    // pub fn attach_window_as_child(...) -> Result<CompositorWindowId, CompositorError> {
    //     todo!()
    // }

    pub fn apply(
        &mut self,
        command: CompositionCommand,
        emit: impl FnMut(BrowserCommand),
    ) -> Result<(), CompositorError> {
        todo!()
    }

    pub fn update_browser_event(
        &mut self,
        event: &BrowserEvent,
        emit: impl FnMut(BrowserCommand),
    ) -> Result<(), CompositorError> {
        todo!()
    }

    pub fn update_chrome_event(
        &mut self,
        event: &ChromeEvent,
    ) -> Result<(), CompositorError> {
        todo!()
    }
}
```

## Follow-ups

- Define the minimal event contract: list which `BrowserEvent` and
  `ChromeEvent` variants must be forwarded into the compositor.
- Decide the exact browser command shapes emitted for scene-level focus,
  routing, and teardown.
- Decide feature-gating and dependency policy for `WindowHost` adapters beyond
  `winit`.
- Design the future child/subview embedding API (`attach_window_as_child(...)`
  or equivalent) separately from the top-level attach path.
- Add a companion design note or issue for `cbf-cli` development tooling
  (`vite`, watch, auto-restart), explicitly separate from `cbf-compositor`.

## Implementation Note

The original `FrameSpec`-based implementation has been archived as
`crates/cbf-compositor-old`. The active `cbf-compositor` crate is now being
rewritten around two separate trees:

- ownership tree: backend object relationships such as transient browsing
  contexts remaining owned by a parent browsing context
- composition tree: window assignment, z-order, bounds, visibility, and
  surface attachment

The current rewrite keeps the existing top-level `attach_window(...)` entry
point, but the public model is scene/item-based rather than frame-based.

- `SurfaceTarget` is limited to `BrowsingContext` and
  `TransientBrowsingContext`.
- transient ownership and presentation are intentionally decoupled: a transient
  browsing context may be shown in the parent's window or a different window,
  but still closes with its parent.
- Chrome surface handles remain below the generic core boundary and are
  consumed only by the `chrome` backend adapter plus the platform host.
- The macOS renderer now uses a compositor-owned container view with one child
  `BrowserViewMac` per scene item, so existing Chromium-backed key, wheel,
  edit-action, and IME behavior can be reused while the new scene model is
  introduced.
