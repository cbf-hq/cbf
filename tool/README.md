# CBF Development Tooling CLI

This directory contains the Python CLI used for Chromium patch/build workflows.

## Installation

This repository exposes the CLI as `tool` via `pyproject.toml`.

Examples in this document use:

```bash
uv run tool ...
```

## Commands

```bash
uv run tool --help
```

Available commands:

- `uv run tool apply`
- `uv run tool export`
- `uv run tool clean`
- `uv run tool commit`
- `uv run tool git`
- `uv run tool release check`
- `uv run tool release licenses`
- `uv run tool release source-info`
- `uv run tool release package`
- `uv run tool verify`
- `uv run tool build`
- `uv run tool run`

`uv run tool apply` uses the current `chromium/src` checkout by default. Pass
`--branch <name>` only when you want the tool to reset/create a work branch at
the selected base commit before applying patches.

`uv run tool build` supports `--out-dir` to override `series.toml` output dir:

```bash
uv run tool build -t chrome --out-dir out/Release
```

## Release Packaging

The release helper commands support the local MVP release pipeline:

```bash
uv run tool release check
uv run tool release licenses
uv run tool release source-info
uv run tool release package
```

These commands are normally orchestrated from the repository root via
`Taskfile.yml`.

## Common Options

`--series` is supported by patch workflow commands and defaults to `cbf`.

## depot_tools Resolution

`verify` and `build` prepend `depot_tools` to `PATH` automatically.

Priority:

1. `--depot-tools <path>`
2. `CBF_DEPOT_TOOLS_PATH`
3. `./depot_tools` (repository root)

Example:

```bash
uv run tool verify --depot-tools ~/dev/depot_tools
```

## Commit Workflow

`commit` runs `git commit` inside `chromium/src`.

Examples:

```bash
uv run tool commit -m "cbf: fix bridge callback race"
uv run tool commit -a -m "cbf: stage tracked changes and commit"
uv run tool commit --amend -m "cbf: revise patch message"
```

## Git Passthrough

`uv run tool git` is an alias for:

```bash
cd chromium/src && git <args...>
```

Examples:

```bash
uv run tool git status
uv run tool git log --oneline -n 10
```

## Running Chromium

`uv run tool run` launches Chromium with CBF-specific flags:

```bash
uv run tool run --enable-features=Cbf --enable-logging=stderr
```
