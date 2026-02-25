# simpleapp

`simpleapp` is a minimal CBF example that implements only the essential features.

It keeps one window and one surface, and continuously renders a single browsing context.

## Current Platform Status

- macOS: supported
- Windows: planned (not implemented yet)

## Features

- Single `winit` window
- Full-window browsing context rendering via `BrowserViewMac`
- Keyboard input
- IME support
- Mouse input
- Cursor icon sync from web content
- Window title sync from browsing context title
- Native drag and drop handling

## Prerequisites

- `CBF_BRIDGE_LIB_DIR` must point to the directory containing `cbf_bridge` shared library artifacts.
- Chromium executable must be provided by either:
  - `--chromium-executable <PATH>`, or
  - `CBF_CHROMIUM_EXECUTABLE=<PATH>`

## Run

```bash
cargo run -p cbf --example simpleapp --features chromium-backend -- \
  --chromium-executable /path/to/Chromium.app/Contents/MacOS/Chromium
```

Using environment variable instead of `--chromium-executable`:

```bash
CBF_CHROMIUM_EXECUTABLE=/path/to/Chromium.app/Contents/MacOS/Chromium \
cargo run -p cbf --example simpleapp --features chromium-backend
```

## CLI Options

- `--url <URL>`
  - Initial URL to open.
  - Default: `https://www.google.com`
- `--chromium-executable <PATH>`
  - Path to Chromium fork executable.
  - If omitted, `CBF_CHROMIUM_EXECUTABLE` is used.
- `--user-data-dir <PATH>`
  - User data directory.
  - If omitted, defaults to:
    - `dirs::data_local_dir()/CBF SimpleApp`
- `--channel-name <NAME>`
  - CBF IPC channel name.
  - Default: `cbf-simpleapp`
- `--enable-logging-stderr`
  - Enable Chromium logging to stderr.
- `--log-file <PATH>`
  - Path to Chromium log file.
- `--chromium-arg <ARG>`
  - Extra Chromium argument.
  - Repeat to pass multiple arguments.

Example to hide Chromium startup window:

```bash
cargo run -p cbf --example simpleapp --features chromium-backend -- \
  --chromium-executable /path/to/Chromium.app/Contents/MacOS/Chromium \
  --chromium-arg=--no-startup-window
```

## Notes

- `simpleapp` is intentionally small and focuses on runtime integration (window, surface, events, and process startup/shutdown).
- For framework-level setup details, see the repository root `README.md`.
