# CBF Licensing Guide

**WARNING**: This document is a work in progress and may be updated as the project evolves. It is intended to provide practical guidance on licensing policy for the CBF project, but should not be considered legal advice. For specific legal questions, consult a qualified attorney.

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
- If the repository includes Chromium patch files or other Chromium-derived
  source diffs, include the Chromium license text alongside the repository
  (for example as `LICENSE.chromium`) and note that Chromium-derived portions
  remain subject to Chromium's license terms.

For binary/distribution bundles containing Chromium artifacts:

- Include third-party notices in release artifacts.
- Preserve required copyright and attribution texts.

For example apps (Tauri/GPUI etc.):

- Track app-specific dependency licenses separately from core CBF notices.

## 5. Release Bundle Files

For the current MVP pre-built macOS distribution, include:

- `Chromium.app`
- `libcbf_bridge.dylib`
- `CBF_LICENSE.txt`
- `THIRD_PARTY_LICENSES.txt`
- `SOURCE_INFO.txt`

Optional supplemental artifact:

- `CHROMIUM_THIRD_PARTY_NOTICES.html`

`CBF_LICENSE.txt` is copied from the repository `LICENSE`.

`THIRD_PARTY_LICENSES.txt` should be generated from Chromium with:

```bash
python tools/licenses.py license_file --format txt
```

`CHROMIUM_THIRD_PARTY_NOTICES.html` is human-readable supplemental attribution.
It does not replace the required bundled license text.

## 6. Release Process Policy

The current release flow is local and manually invoked.

Recommended process:

1. Regenerate `THIRD_PARTY_LICENSES.txt` from the exact Chromium revision used for the build.
2. Generate `SOURCE_INFO.txt` from the exact tagged source state used for the bundle.
3. Verify the release archive includes all required files.
4. Manually upload the final archive after inspection.

See [Release Process](./release-process.md) for the concrete MVP workflow.

## 7. Contributor/Consumer Clarity

README should state clearly:

- CBF code is BSD 3-Clause.
- Chromium/third-party components remain under their own licenses.
- Redistributors must include relevant notices.

## 8. Compliance Checklist

Before release:

- [ ] `CBF_LICENSE.txt` is present and matches the repository `LICENSE`
- [ ] `THIRD_PARTY_LICENSES.txt` is generated for the exact Chromium revision being distributed
- [ ] `SOURCE_INFO.txt` is generated for the exact tagged source state being distributed
- [ ] Release artifacts include the required bundled license files
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
