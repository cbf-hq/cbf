# Contributor Setup

This chapter is for contributors who modify CBF crates, bridge code, or Chromium-fork patches.

## 1. Prerequisites

You need the following tools installed and configured:

- Stable Rust toolchain
- Git
- Python 3
- `depot_tools`
- Chromium build dependencies for your OS
- Chromium source code

For downloading Chromium source code and `depot_tools` and setting up the Chromium build environment, follow the official guide: <https://chromium.googlesource.com/chromium/src/+/HEAD/docs/get_the_code.md>

You should place Chromium source tree at `chromium/src` relative to the repository root. This allows build scripts to locate bridge artifacts without additional configuration.
It is also recommended to place `depot_tools` at `./depot_tools` relative to the repository root so helper commands can discover it consistently.

## 2. Build Chromium targets

Before building, generate GN build files (this creates `chromium/src/out/Default/args.gn`):

```bash
cd chromium/src
gn gen out/Default
```

Optionally, you can tune `out/Default/args.gn` for CBF development. Example:

```gn
# Build release-like binaries for faster runtime and smaller output.
is_debug = false

# Keep DCHECK-enabled behavior even in non-debug builds.
# This is important for CBF development since CBF and Chromium often rely
# on DCHECKs to validate invariants and catch issues early.
dcheck_always_on = true

# Use component build for faster incremental link/build cycles.
is_component_build = true

# Enable ccache via wrapper to speed up repeated C/C++ compilation.
cc_wrapper = "env CCACHE_SLOPPINESS=time_macros ccache"

# Reduce overall symbol volume.
symbol_level = 1

# CBF usually does not modify Blink/V8 internals, so strip more symbols there.
blink_symbol_level = 0
v8_symbol_level = 0
```

Then build relevant targets with `autoninja` from `chromium/src`:

```bash
autoninja -C out/Default chrome
autoninja -C out/Default cbf_bridge
```

This builds the main Chromium target and the CBF bridge library, which is required for `cbf-chrome-sys` to link successfully.

Release packaging uses a separate `chromium/src/out/Release` directory and a
different `args.gn` policy. See [Release Process](./release-process.md) for the
release-specific flow.

> [!WARNING]
> Chromium builds can take several hours or longer, and CPU usage may stay high during the entire build, which can keep sustained load on your machine.
> Because builds are long-running, you can mitigate sleep-related build interruption by using tools such as `caffeinate` of macOS.

## 3. Configure bridge linkage

`cbf-chrome-sys` and `cbf-chrome` needs bridge artifacts from `CBF_BRIDGE_LIB_DIR`:

```bash
export CBF_BRIDGE_LIB_DIR="/path/to/chromium/src/out/Default"
```

Then run focused checks:

```bash
cargo check -p cbf
cargo check -p cbf-chrome
cargo check -p cbf-chrome-sys
cargo test -p cbf
cargo test -p cbf-chrome
cargo test -p cbf-chrome-sys
```

## 4. Contributor validation checklist

- Rust crates compile and tests pass for touched areas.
- Rust code changes are formatted with `cargo fmt`.
- Rust code follows lint rules with `cargo clippy`.
- `cbf_bridge` builds against current Chromium revision.
- Run Chromium-side tests as needed based on the impact scope of your changes (see [Chromium Fork Workflow](./chromium-fork-workflow.md) for how to run them).

## 5. Where to continue

- Integration invariants: [Chromium Integration Rules](./chromium-integration-rules.md)
- Implementation details: [Chromium Implementation Guide](./chromium-implementation-guide.md)
- Fork operations and patch queue: [Chromium Fork Workflow](./chromium-fork-workflow.md)
- Local release packaging: [Release Process](./release-process.md)
- Decision history: [Decisions](./decisions.md)
