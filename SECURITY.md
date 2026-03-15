# Security Policy

This document defines the current vulnerability reporting policy for CBF.

CBF is still in an early pre-1.0 stage. Security handling aims to keep reports
private by default and to coordinate disclosure after a fix or mitigation is
available.

## Supported Versions

At this stage, only the latest published prerelease or release is considered
supported for security fixes.

Older tags may receive fixes at maintainer discretion, but reporters should
assume that the current latest version is the supported baseline.

## Reporting a Vulnerability

Preferred reporting path:

- Use GitHub's private vulnerability reporting / security advisory flow for this repository when available.
- If private reporting is not yet enabled or is temporarily unavailable, report the issue privately to the maintainer at `tasuren@icloud.com`.

Important:

- Do not open a public GitHub issue for an unpatched security vulnerability.
- Include a clear impact summary, affected version or commit, reproduction steps, and any required environment details.
- If a proof of concept is available, provide the smallest reproduction that demonstrates the issue safely.

## What to Include

Please include:

- affected crate, component, or patch area
- affected tag, commit SHA, or local revision
- platform and architecture
- reproduction steps
- expected impact and realistic attack scenario
- whether the issue depends on a downstream host application's behavior

## Scope

In scope:

- `cbf`
- `cbf-chrome`
- `cbf-chrome-sys`
- Chromium-side CBF code under `chromium/src/chrome/browser/cbf/`
- CBF bridge behavior and IPC/lifecycle handling
- CBF-maintained Chromium patch queue under `chromium/patches/cbf/`
- prebuilt artifacts published by this repository when applicable

Usually out of scope:

- vulnerabilities in downstream applications built on top of CBF
- product-specific app UI, business logic, or deployment configuration
- purely upstream Chromium issues that are not introduced or modified by CBF-specific integration
- reports that require unsafe local development setup without a plausible security impact

If a report spans both CBF and downstream code, the maintainer may still help
triage the boundary and redirect follow-up work.

## Response Targets

Current response targets:

- initial acknowledgement within 4 business days
- follow-up status update within 7 business days after acknowledgement when the issue remains open

These are best-effort targets, not a contractual SLA.

## Disclosure Policy

CBF prefers coordinated disclosure:

- keep reports private while impact and affected scope are being validated
- prepare a fix or practical mitigation before public disclosure when feasible
- publish the fix first, then share advisory details once users have a reasonable update path

If immediate public disclosure is being considered, please coordinate first so
affected users have a chance to update safely.

## Language

English is preferred for reports and follow-up discussion.
Japanese is also accepted.
