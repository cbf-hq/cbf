# ADR Guide (CBF)

This directory, `docs/decisions/`, stores **Architecture Decision Records (ADRs)** for CBF (Chromium Browser Framework).
The goal of an ADR is to preserve *why* a decision was made so future maintainers, AI agents, and contributors can trace it.
An ADR is not an implementation design document.

## Current ADRs

- [ADR 0001: Layered API for Browser-Generic and Chromium-Specific Surfaces](./0001-layered-api-for-generic-and-chromium.md)
- [ADR 0001: API Design Sketch](./0001-api-design-sketch.md)
- [ADR 0002: DevTools Integration Without Chrome Browser Dependency](./0002-devtools-integration-without-chrome-browser-dependency.md)
- [ADR 0003: Chrome Runtime Default and Embedded Scope Boundary](./0003-chrome-runtime-default-and-cbf-scope.md)
- [ADR 0004: Chrome Feature Wiring on WebContents Path](./0004-chrome-feature-wiring-on-webcontents-path.md)
- [ADR 0005: Host-Mediated Browsing Context Open and Disposition Mapping](./0005-host-mediated-browsing-context-open-and-disposition-mapping.md)

## 1. Naming Convention

- Filename: `NNNN-kebab-case-title.md`
  - `NNNN` is a 4-digit sequential number (for example, `0001-...`)
  - As a rule, numbering order should match decision order
- Title line: `# ADR NNNN: ...`

## 2. Required Sections (in this order)

Every ADR must include the following sections:

- `Status`
- `Date`
- `Context`
- `Decision`
- `Consequences`
- `Alternatives Considered`
- `Notes`
- `Follow-ups`

### 2.1 Status

Recommended values (you may add more if needed):

- `Proposed`: under proposal (not yet decided)
- `Accepted`: adopted (decision made)
- `Rejected`: rejected
- `Deprecated`: previously valid but now deprecated
- `Superseded`: replaced by another ADR (explicitly include the replacing ADR number)

### 2.2 Date

- Format: `YYYY-MM-DD`
- Use the date when the ADR became an actual decision (typically when set to `Accepted`)

## 3. Writing Guidelines

### Context

- Describe the problem to solve, constraints, assumptions, current state, and why a decision is needed now
- If references exist, include links or repository paths

### Decision

- State in definitive form what will be done, at which boundary, and in which dependency direction
- When possible, make responsibilities and dependencies explicit (avoid cycles)

### Consequences

- Separate `Positive` (expected benefits) and `Negative / Trade-offs` (costs, drawbacks)
- Consider impact areas such as developer experience, changeability, performance, operations, and testing

### Alternatives Considered

- List alternatives and briefly explain why each was not chosen

### Notes

- Add important supplemental information that does not fit naturally in Context/Decision/Consequences
- You may explicitly mark what is out of scope for this ADR

### Follow-ups

- List concrete next actions as bullets (entry points to implementation tasks)
- If possible, include ordering or the first step

## 4. Template

Create new ADRs by copying the template below.

```markdown
# ADR NNNN: <Short Title>

- Status: <Proposed | Accepted | ...>
- Date: YYYY-MM-DD

## Context

<Background, problem, constraints, reference links>

## Decision

<Definitive statement of this decision>

## Consequences

### Positive

- <Benefit>

### Negative / Trade-offs

- <Cost or drawback>

## Alternatives Considered

### A. <Alternative>

- <Reason it was not selected>

## Notes

- <Supplemental notes>

## Follow-ups

- <Next action>
```
