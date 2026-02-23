# CBF Chromium Patch Series

This directory stores the CBF patch series applied to `chromium/src`.

## Order

Patches are applied in lexical order:

1. `0001-CBF-bridge-series.patch`
2. `0002-refactor-chrome-split-profile-service-into-focused-c.patch`
3. `0003-refactor-bridge-add-devtools-bridge-commands-and-eve.patch`
4. `0004-chore-chrome-add-devtools-ui-bindings-diagnostics.patch`
5. `0005-test-chrome-add-browser-and-unit-tests-for-profile-c.patch`

## Principles

- Keep one primary responsibility per patch.
- Keep dependency direction explicit in patch ordering.
- Prefer mechanical moves/refactors before behavior changes.
- Keep each patch buildable in the target Chromium output directory.
