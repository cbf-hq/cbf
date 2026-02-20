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

    fn to_raw_command(command: BrowserCommand) -> Self::RawCommand;
    fn to_generic_event(raw: &Self::RawEvent) -> Option<BrowserEvent>;

    fn connect<D: BackendDelegate>(
        self,
        delegate: D,
    ) -> Result<(CommandSender<Self>, EventStream<Self>), Error>
    where
        Self: Sized;
}

pub struct CommandSender<B: Backend> {
    // private tx of B::RawCommand
}

impl<B: Backend> CommandSender<B> {
    pub async fn send(&self, command: BrowserCommand) -> Result<(), Error> {
        // generic -> raw via B::to_raw_command
        todo!()
    }

    // Explicit escape hatch.
    pub async fn send_raw(&self, raw: B::RawCommand) -> Result<(), Error> {
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
    ) -> Result<(CommandSender<Self>, EventStream<Self>), Error> {
        // build transport via cbf-chrome-sys
        let _ = delegate;
        todo!()
    }
}
```

## Implementation Notes

- Keep `cbf` free from Chromium nouns in public types and docs.
- Keep raw access explicit in API names (`send_raw`, `as_raw`).
- Preserve single event consumption path (`EventStream::recv`) to avoid dual-loop ambiguity.
- If desired, place raw extension traits under a dedicated module/feature (for example `cbf::raw` + `raw-api` feature).
- `cbf-chrome-sys` owns ABI safety, memory/lifetime rules, and conversion to/from `cbf_bridge`.
