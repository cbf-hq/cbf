# simpleapp

`simpleapp` is the compositor-based CBF example application.

It uses `cbf-compositor` to compose multiple browser-managed surfaces in a
single [winit](https://github.com/rust-windowing/winit) window:

- a toolbar UI browsing context loaded from `app://simpleapp/ui.html`
- a normal page browsing context rendered below the toolbar

Secondary host-managed windows are also created for:

- popup windows backed by transient browsing contexts
- dedicated DevTools windows
- additional host-managed browsing context windows opened with a local mock toolbar

This sample remains macOS-only.

The toolbar and overlay UI are served from embedded assets through
`app://simpleapp/...` using the Chrome-specific custom scheme responder in
`cbf-chrome`. This keeps the sample on the same URL model in development and in
bundled builds without resolving `file://` paths from the Cargo manifest tree.

When `--test-overlay-surface` is enabled, `overlay.html` loads
`overlay-hit-test.js` and pushes hit-test region snapshots for elements marked
with `data-cbf-hit-test-region`. The sample transport uses
`window.cbf.invoke("simpleapp.overlay.hit_test.update", ...)` and chooses the
snapshot mode from the overlay URL. With `--test-overlay-surface`, the centered
overlay label consumes clicks while transparent regions pass input through to
the page. Adding `--passthrough-only-overlay-region` flips that interpretation
so only the centered label passes input through and the rest of the overlay
consumes it.

## License

This project is licensed under the BSD Zero Clause License.
You may use, copy, modify, and distribute this example without restriction.
It is provided as-is, without warranty or liability.
See the [LICENSE](./LICENSE) file for details.
