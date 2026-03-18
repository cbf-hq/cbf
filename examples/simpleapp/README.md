# simpleapp

`simpleapp` is the compositor-based CBF example application.

It uses `cbf-compositor` to compose multiple browser-managed surfaces in a
single [winit](https://github.com/rust-windowing/winit) window:

- a toolbar UI browsing context loaded from `src/ui.html`
- a normal page browsing context rendered below the toolbar

Secondary host-managed windows are also created for:

- popup windows backed by transient browsing contexts
- dedicated DevTools windows
- additional host-managed browsing context windows requested by the backend

This sample remains macOS-only.

The toolbar UI is currently static HTML loaded from `src/ui.html` through a
`file://` URL. Back/forward/reload/address-bar controls are visual placeholders
until IPC-backed host integration is added to CBF.
