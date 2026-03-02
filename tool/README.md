# CBF Development Tooling CLI

This directory contains the Python CLI used for Chromium patch/build workflows.

## Installation

This repository exposes the CLI as `tool` via `pyproject.toml`.

Examples in this document use:

```bash
uv run tool ...
```

## Commands

Use the `patch` command group:

```bash
tool patch --help
```

Available commands:

- `tool patch apply`
- `tool patch export`
- `tool patch clean`
- `tool patch commit`
- `tool patch git`
- `tool patch verify`
- `tool patch build`

`tool patch apply` uses the current `chromium/src` checkout by default. Pass
`--branch <name>` only when you want the tool to reset/create a work branch at
the selected base commit before applying patches.

`tool patch build` supports `--out-dir` to override `series.toml` output dir:

```bash
tool patch build -t chrome --out-dir out/Release
```

## Deprecated Alias

`tool chromium ...` is still available as a compatibility alias, but it is deprecated.
Use `tool patch ...` for new scripts.

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
tool patch verify --depot-tools ~/dev/depot_tools
```

## Commit Workflow

`commit` runs `git commit` inside `chromium/src`.

Examples:

```bash
tool patch commit -m "cbf: fix bridge callback race"
tool patch commit -a -m "cbf: stage tracked changes and commit"
tool patch commit --amend -m "cbf: revise patch message"
```

## Git Passthrough

`tool patch git` is an alias for:

```bash
cd chromium/src && git <args...>
```

Examples:

```bash
tool patch git status
tool patch git log --oneline -n 10
```
