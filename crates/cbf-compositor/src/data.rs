pub type RequestId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CompositorWindowId(u64);

impl CompositorWindowId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameId(u64);

impl FrameId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

pub trait RequestIdAllocator {
    fn next_request_id(&mut self) -> RequestId;
}

#[derive(Debug, Clone)]
pub struct DefaultRequestIdAllocator {
    next: RequestId,
}

impl Default for DefaultRequestIdAllocator {
    fn default() -> Self {
        Self { next: 1 }
    }
}

impl RequestIdAllocator for DefaultRequestIdAllocator {
    fn next_request_id(&mut self) -> RequestId {
        let request_id = self.next;
        self.next = self.next.saturating_add(1);
        request_id
    }
}

#[derive(Debug, Clone, Default)]
pub struct AttachWindowOptions {
    pub transparent: bool,
}

#[derive(Debug, Clone, Default)]
pub struct FrameComposition {
    pub frames: Vec<FrameSpec>,
}

#[derive(Debug, Clone)]
pub struct FrameSpec {
    pub id: FrameId,
    pub kind: FrameKind,
    pub url: String,
    pub bounds: FrameBounds,
    pub ipc: IpcPolicy,
    pub transparency: TransparencyPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameKind {
    Ui,
    Page,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrameBounds {
    FullWindow,
    Rect(Rect),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcPolicy {
    Deny,
    Allow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransparencyPolicy {
    Opaque,
    Transparent,
}

#[derive(Debug, Clone)]
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
