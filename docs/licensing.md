# CBF Licensing Guide

## 1. Scope

This document defines practical licensing policy for the standalone CBF repository.
It is an engineering policy document and not legal advice.

## 2. Base License Choice

Recommended base license for CBF-authored code:

- `BSD 3-Clause`

Why:

- Permissive and widely acceptable for library reuse.
- Consistent style with Chromium/CEF ecosystem expectations.

## 3. Layered License Model

Use a layered interpretation, not a single-license simplification:

- CBF source authored in this project: BSD 3-Clause
- Chromium and bundled third-party components: each component's own license terms

Important implication:

- Repository base license does not remove third-party notice obligations.

## 4. Redistribution Responsibilities

For source-only distribution:

- Keep BSD 3-Clause license text for CBF code.
- Document that Chromium/tooling dependencies are governed by their own licenses.

For binary/distribution bundles containing Chromium artifacts:

- Include third-party notices in release artifacts.
- Preserve required copyright and attribution texts.

For example apps (Tauri/GPUI etc.):

- Track app-specific dependency licenses separately from core CBF notices.

## 5. Required Files in Repository

Recommended minimum:

- `LICENSE` (BSD 3-Clause for CBF-authored code)
- `NOTICE` (short distribution notice)
- `THIRD_PARTY_NOTICES/` (generated or curated notices)
- `docs/licensing.md` (this policy)

## 6. CI and Release Policy

Recommended process:

1. Maintain an automated or semi-automated notice generation flow.
2. Regenerate/update `THIRD_PARTY_NOTICES` when Chromium revision changes.
3. Gate release on license artifact checks.
4. Verify release package actually includes required notice files.

## 7. Contributor/Consumer Clarity

README should state clearly:

- CBF code is BSD 3-Clause.
- Chromium/third-party components remain under their own licenses.
- Redistributors must include relevant notices.

## 8. Compliance Checklist

Before release:

- [ ] `LICENSE` is present and current (BSD 3-Clause)
- [ ] `NOTICE` is present and current
- [ ] `THIRD_PARTY_NOTICES` is generated/updated
- [ ] Release artifacts include notice files
- [ ] README licensing section matches actual distribution behavior

## 9. References

- Chromium repository license page:
    - https://github.com/chromium/chromium
- Chromium license tooling:
    - https://chromium.googlesource.com/chromium/src.git/+/HEAD/tools/licenses/licenses.py
- Chromium OS licensing overview:
    - https://www.chromium.org/chromium-os/licensing/
- SPDX BSD-3-Clause:
    - https://spdx.org/licenses/BSD-3-Clause.html
