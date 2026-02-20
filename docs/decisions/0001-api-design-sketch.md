# ADR 0001 API Design Sketch

This document provides a concrete API sketch aligned with:

- `docs/decisions/0001-layered-api-for-generic-and-chromium.md`

It is intentionally close to the current `NEW_ARCH_STUB.rs` idea, but updated to use the ADR naming (`RawCommand` / `RawEvent`) and the `OpaqueEvent` flow.

## Scope

- This is an implementation sketch, not a strict final signature set.
- The normative source of truth is ADR 0001.

## Layering

```text
cbf            (generic safe API)
cbf-chrome     (chromium-specific safe API + Backend impl)
cbf-chrome-sys (unsafe FFI/wire to cbf_bridge)
```

Dependency direction:

```text
cbf (no dependency on chrome layers)
cbf-chrome -> cbf + cbf-chrome-sys
cbf-chrome-sys -> cbf_bridge
```

## Generic API Sketch (`cbf`)

```rust
pub enum BrowserCommand {
    // browser-generic commands
}

pub enum BrowserEvent {
    // browser-generic events
}

pub trait Backend {
    type RawCommand;
    type RawEvent;
    type RawDelegate;

    fn to_raw_command(command: BrowserCommand) -> Self::RawCommand;
    fn to_generic_event(raw: &Self::RawEvent) -> Option<BrowserEvent>;

    fn connect<D: BackendDelegate>(
        self,
        delegate: D,
        raw_delegate: Option<Self::RawDelegate>,
    ) -> Result<(CommandSender<Self>, EventStream<Self>), Error>
    where
        Self: Sized;
}

pub struct CommandSender<B: Backend> {
    // private tx of B::RawCommand
}

impl<B: Backend> CommandSender<B> {
    pub fn send(&self, command: BrowserCommand) -> Result<(), Error> {
        // generic -> raw via B::to_raw_command
        todo!()
    }
}

pub trait RawCommandSenderExt<B: Backend> {
    // Explicit escape hatch.
    fn send_raw(&self, raw: B::RawCommand) -> Result<(), Error>;
}

impl<B: Backend> RawCommandSenderExt<B> for CommandSender<B> {
    fn send_raw(&self, raw: B::RawCommand) -> Result<(), Error> {
        todo!()
    }
}

pub struct OpaqueEvent<B: Backend> {
    raw: B::RawEvent,
    generic: Option<BrowserEvent>,
}

impl<B: Backend> OpaqueEvent<B> {
    pub fn as_generic(&self) -> Option<&BrowserEvent> {
        self.generic.as_ref()
    }
}

pub trait RawOpaqueEventExt<B: Backend> {
    fn as_raw(&self) -> &B::RawEvent;
}

impl<B: Backend> RawOpaqueEventExt<B> for OpaqueEvent<B> {
    fn as_raw(&self) -> &B::RawEvent {
        &self.raw
    }
}

pub struct EventStream<B: Backend> {
    // private rx of B::RawEvent
}

impl<B: Backend> EventStream<B> {
    pub async fn recv(&self) -> Result<OpaqueEvent<B>, Error> {
        // single receive path:
        // raw receive -> try to map to generic once -> store both in OpaqueEvent
        todo!()
    }

    pub fn recv_blocking(&self) -> Result<OpaqueEvent<B>, Error> {
        todo!()
    }
}
```

## Chromium Implementation Sketch (`cbf-chrome`)

```rust
pub enum ChromeCommand {
    // chromium-specific commands
}

pub enum ChromeEvent {
    // chromium-specific events
}

pub struct ChromiumBackend;

impl Backend for ChromiumBackend {
    type RawCommand = ChromeCommand;
    type RawEvent = ChromeEvent;
    type RawDelegate = ChromeDelegate;

    fn to_raw_command(command: BrowserCommand) -> Self::RawCommand {
        // generic -> chromium mapping
        todo!()
    }

    fn to_generic_event(raw: &Self::RawEvent) -> Option<BrowserEvent> {
        // chromium -> generic mapping (None if no generic equivalent)
        todo!()
    }

    fn connect<D: BackendDelegate>(
        self,
        delegate: D,
        raw_delegate: Option<Self::RawDelegate>,
    ) -> Result<(CommandSender<Self>, EventStream<Self>), Error> {
        // build transport via cbf-chrome-sys
        let _ = (delegate, raw_delegate);
        todo!()
    }
}
```

Current staged implementation note:

- `ChromeCommand` is now a Chromium-named transport enum (e.g. `CreateWebContents`, `SetWebContentsSize`).
- The direction is one-way for semantic mapping: `ChromeCommand -> BrowserCommand`.
  - `Backend::to_raw_command` maps generic commands into Chromium command vocabulary.
- `ChromeEvent` is raw-only:
  - `Ipc(IpcEvent)` for bridge-native events
  - Chromium-owned non-IPC raw variants (`BackendReady`, `BackendError`, etc.) for lifecycle/error events
- `to_generic_event` maps `IpcEvent` to `BrowserEvent` where possible.
- `BackendDelegate` is policy-only:
  - command: `Forward | Drop | Stop`
  - event: `Forward | Stop`
  - payload transform and delegate-emitted events are intentionally unsupported.
- Delegate dispatching is decision-first:
  - `DelegateDispatcher::dispatch_command` returns `CommandDecision`.
  - `DelegateDispatcher::dispatch_event` returns `EventDecision`.
  - Forward execution is immediate at call sites (no deferred emit callback path).
  - `flush` only drains queued generic commands (`Vec<BrowserCommand>`); execution and emit are backend-owned.
  - `stop` runs teardown and returns queued commands for backend-side drain before final stop emission.
- Backend implementations process transport events as raw-first data:
  - raw event -> optional generic projection for delegate policy -> raw emit.
  - avoid `Raw -> Generic -> Raw` re-encoding loops in transport paths.

## Implementation Notes

- Keep `cbf` free from Chromium nouns in public types and docs.
- Keep raw access explicit in API names (`send_raw`, `as_raw`) and extension traits (`RawCommandSenderExt`, `RawOpaqueEventExt`).
- Preserve single event consumption path (`EventStream::recv`) to avoid dual-loop ambiguity.
- Current staging decision: keep a single `connect` entry point and pass `raw_delegate` as `Option`.
  - Re-evaluate and migrate to split connect APIs or `ConnectOptions` when optional-argument overhead becomes significant.
- If desired, place raw extension traits under a dedicated module/feature (for example `cbf::raw` + `raw-api` feature).
- `cbf-chrome-sys` owns ABI safety, memory/lifetime rules, and conversion to/from `cbf_bridge`.
- Initial rollout may use `RawCommand = BrowserCommand` / `RawEvent = BrowserEvent` in existing backends for compatibility, then migrate to richer backend-native raw types incrementally.
