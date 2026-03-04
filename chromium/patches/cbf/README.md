# CBF Chromium Patch Series

This directory stores the CBF patch series applied to `chromium/src`.

## Order

Patches are applied in lexical order:

1. `0001-Add-CBF-bridge-baseline.patch`
2. `0002-Split-profile-service-into-focused-controllers.patch`
3. `0003-Add-DevTools-bridge-commands-and-event-mapping.patch`
4. `0004-Add-browser-and-unit-tests-for-profile-controllers.patch`
5. `0005-Wire-extension-runtime-and-install-prompt-flow.patch`
6. `0006-Wire-extension-runtime-and-auxiliary-window-flow.patch`
7. `0007-Stabilize-DevTools-frontend-rebinding.patch`
8. `0008-Add-auxiliary-window-and-popup-flow.patch`
9. `0009-Update-build-deps-for-CBF-DevTools-and-extension-hoo.patch`
10. `0010-Restore-browser-backed-CBF-tabs-after-roll.patch`
11. `0011-Preserve-embedded-rendering-for-browser-backed-tabs.patch`
12. `0012-Replace-named-channel-bootstrap-with-inherited-endpo.patch`
13. `0013-Add-per-session-token-handshake-for-IPC-authenticati.patch`

## Principles

- Keep one primary responsibility per patch.
- Keep dependency direction explicit in patch ordering.
- Prefer mechanical moves/refactors before behavior changes.
- Keep each patch buildable in the target Chromium output directory.
- Use short imperative English subjects for exported patches; do not use
  Conventional Commits in patch titles.
- When refining an existing patch, prefer `fixup` / `squash` into that patch
  instead of adding a follow-up fix patch.
