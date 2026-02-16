# Graphic Sharing in Chromium (Windows/Linux)

## 1. Scope

This note summarizes how Chromium shares rendered results for desktop platforms,
focusing on Windows and Linux, based on code in `chromium/src`.

The target question is:

- How rendered output is passed from renderer-side compositing into browser-side display.
- Which OS-specific memory sharing primitives back that flow on Windows vs Linux.

## 2. Common Cross-Platform Flow

At a high level, Chromium does not pass raw pixel blobs from renderer to browser.
It passes frame metadata plus transferable GPU resources:

1. Renderer submits a `viz::CompositorFrame` through `CompositorFrameSink`.
2. The frame contains `viz::TransferableResource` entries.
3. Each transferable resource points to a `ClientSharedImage` (`Mailbox` + `SyncToken`).
4. Browser/viz side imports and composes surfaces by `FrameSinkId`/`SurfaceId`.

Key code:

- `third_party/blink/renderer/platform/widget/widget_base.cc`
  - Renderer requests/creates frame sink pipes and calls `widget_host_->CreateFrameSink(...)`.
- `cc/mojo_embedder/async_layer_tree_frame_sink.cc`
  - `SubmitCompositorFrame(...)` sends frames over mojo.
  - `DidReceiveCompositorFrameAck(...)` and `ReclaimResources(...)` return resource ownership.
- `components/viz/common/resources/transferable_resource.h`
  - `TransferableResource` wraps `ClientSharedImage` and exposes mailbox/sync token semantics.
- `content/browser/renderer_host/render_widget_host_impl.cc`
  - `CreateFrameSink(...)` forwards frame sink creation to viz.
- `content/browser/renderer_host/render_widget_host_view_aura.cc`
  - Browser view embeds surfaces via `DelegatedFrameHost::EmbedSurface(...)`.
- `content/browser/renderer_host/delegated_frame_host.cc`
  - Surface embedding and deadline behavior for visible/hidden/resizing cases.

## 3. Windows Path

Windows uses DXGI/D3D-backed SharedImage backings.

### 3.1 Native buffer/share type

- Native `GpuMemoryBufferType` for Windows is `DXGI_SHARED_HANDLE`.
- See `gpu/command_buffer/service/shared_image/shared_image_factory.cc`.

### 3.2 SharedImage backing factories

- `D3DImageBackingFactory` is used when D3D shared images are supported.
- `DCompImageBackingFactory` is also used when DirectComposition is available.
- Selection wiring is in `shared_image_factory.cc`.

### 3.3 Handle creation/import

- `D3DImageBackingFactory::CreateGpuMemoryBufferHandle(...)` creates D3D11 textures
  with shared handle flags and exports a DXGI shared handle.
- `D3DImageBackingFactory::CreateSharedImage(..., gfx::GpuMemoryBufferHandle handle)`
  validates `handle.type == gfx::DXGI_SHARED_HANDLE` and opens/imports the texture.
- See `gpu/command_buffer/service/shared_image/d3d_image_backing_factory.cc`.

### 3.4 Synchronization

- Windows has explicit DXGI fence registration/update path:
  - `SharedImageInterfaceProxy::UpdateSharedImage(..., D3DSharedFence, ...)`
    enqueues `RegisterDxgiFence` / `UpdateDxgiFence`.
  - GPU service side handles these in `SharedImageStub`.
- See:
  - `gpu/ipc/client/shared_image_interface_proxy.cc`
  - `gpu/ipc/service/shared_image_stub.cc`

## 4. Linux Path (Ozone)

Linux desktop path is based on Ozone + `NativePixmap` (typically dma-buf-backed).

### 4.1 Native buffer/share type

- Native `GpuMemoryBufferType` for Linux is `NATIVE_PIXMAP`.
- See `gpu/command_buffer/service/shared_image/shared_image_factory.cc`.

### 4.2 SharedImage backing factory

- `OzoneImageBackingFactory` creates `OzoneImageBacking` backed by `gfx::NativePixmap`.
- Import path accepts `gfx::NATIVE_PIXMAP` handles.
- See:
  - `gpu/command_buffer/service/shared_image/ozone_image_backing_factory.cc`
  - `gpu/command_buffer/service/shared_image/ozone_image_backing.cc`

### 4.3 Synchronization model

- `OzoneImageBacking` tracks begin/end access streams and can exchange `GpuFenceHandle`
  for overlay/GL/Vulkan/WebGPU access paths.
- Ozone factory checks whether required fence interop is supported depending on usage.
- See `ozone_image_backing.cc` and `ozone_image_backing_factory.cc`.

## 5. Linux Platform Details: Wayland vs X11

Both sit under Ozone, but import/presentation details differ.

### 5.1 Wayland

- GPU side creates dmabuf/shm buffers and commits overlays through
  `WaylandBufferManagerGpu`.
- Host side receives buffer creation (`CreateDmabufBasedBuffer`) in
  `WaylandBufferManagerHost`.
- See:
  - `ui/ozone/platform/wayland/gpu/wayland_buffer_manager_gpu.cc`
  - `ui/ozone/platform/wayland/host/wayland_buffer_manager_host.cc`

### 5.2 X11 (Ozone/X11)

- `X11SurfaceFactory` creates `NativePixmap` from GBM support (`NativePixmapDmaBuf`).
- Import for GL can be:
  - direct dma-buf import (`EGL_EXT_image_dma_buf_import`), or
  - DRI3 `PixmapFromBuffer` path (dma-buf -> X11 Pixmap -> EGL).
- See:
  - `ui/ozone/platform/x11/x11_surface_factory.cc`
  - `ui/ozone/platform/x11/native_pixmap_egl_x11_binding.cc`

## 6. Implications for `BrowserView` API Design

For a cross-platform high-level `BrowserView`, keep a common surface/frame contract,
and hide platform memory primitives behind per-platform backends:

- Common API layer should reason in terms of:
  - frame/surface lifecycle,
  - visibility/resizing sync points,
  - presentation feedback and failure handling.
- Platform layer should own:
  - Windows: DXGI shared handle + DXGI fence integration.
  - Linux: NativePixmap/dma-buf (+ Ozone platform specifics like Wayland/X11).

This aligns with Chromium’s own separation:

- common frame transport (`CompositorFrame` + `TransferableResource`),
- platform-specific SharedImage backing and synchronization.

## 7. Feasibility for High-Level `BrowserView` Embedding

The intended usage is:

- place `BrowserView` as a subview/subwindow inside an existing app window,
  coexisting with UI such as URL bar and tabs,
- switch input focus between app UI and `BrowserView`,
- coexist with host frameworks/toolkits such as Tauri, Wry, and GPUI.

Current `BrowserViewMac` behavior (separate subview under `contentView`, hit-test based
pointer routing, and first-responder based keyboard/IME routing) is a good baseline.
Equivalent behavior is generally feasible on Windows and Linux, with platform caveats.

### 7.1 Windows

- Embedding: feasible via child-window composition (`HWND` hierarchy).
- Focus switching: feasible with native focus APIs and IME routing.
- Toolkit coexistence (Tauri/Wry/GPUI): generally practical when native window handle
  integration is available.

### 7.2 Linux (X11)

- Embedding: generally feasible with child/reparented window patterns.
- Focus switching: feasible with X11 input focus control.
- Toolkit coexistence: typically practical, similar in spirit to Windows.

### 7.3 Linux (Wayland)

- Embedding: feasible mainly when done as same-client sub-surface style composition.
- Cross-toolkit embedding can be constrained by compositor/security model.
- Focus and input routing are more compositor-driven, so behavior is less uniform
  across environments than Windows/X11.

### 7.4 Practical API Direction

Design `BrowserView` as capability-driven API:

- `attach_to_native_parent(handle, rect)`
- `set_focus(bool)` / `has_focus()`
- input routing hooks for pointer/key/IME as needed by host integration
- `backend_capabilities()` to expose backend limits (for example, Wayland embedding
  restrictions)

This keeps public API consistent while allowing backend-specific behavior differences
without leaking Chromium internals.
