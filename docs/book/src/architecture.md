# Architecture

CBF is organized as a strict layered stack:

- `cbf`
- `cbf-chrome`
- `cbf-chrome-sys`
- Chromium fork and `cbf_bridge`

Dependency direction must remain:

`Application -> cbf -> cbf-chrome -> cbf-chrome-sys -> Chromium process`

The public `cbf` API stays browser-generic.
Chromium and Mojo implementation details terminate at the boundary layers.

This chapter should explain the design in a reader-friendly order before diving into contributor-only implementation rules.
