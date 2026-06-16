# RFC-NNN: [Title]

| Field | Value |
|-------|-------|
| **Status** | Draft |
| **Author** | [Your name / GitHub handle] |
| **Date** | YYYY-MM-DD |
| **Tracking** | [Link to issue or discussion] |

## Summary

One-paragraph description of the proposal.

## Motivation

Why this change is needed. What problem does it solve? What use cases does
it enable? Link to any prior art, issues, or user feedback.

## Detailed Design

The technical or procedural design. Include:

- Data structures, APIs, or protocol changes
- Migration path for existing users
- Impact on invariants (reference `docs/invariants/` if applicable)
- Impact on ABI stability (reference `STABILITY.md` and `BREAKING.md`)

## Alternatives Considered

Other approaches evaluated and why they were rejected.

## Unresolved Questions

Open issues to be resolved during the RFC review period.

---

## Process

1. Copy this template to a new file: `rfcs/RFC-NNN-short-title.md`
2. Fill in all sections; set Status to **Draft**
3. Open a pull request titled `RFC-NNN: Short Title`
4. Collect feedback; iterate on the design
5. Once consensus is reached, update Status to **Accepted**
6. If the RFC is superseded later, update Status to **Superseded by RFC-MMM**

### RFCs vs ADRs

- **ADRs** (`docs/adr/`) record architecture decisions that constrain the
  system design. They follow the format in `docs/adr/index.md` and are
  binding once accepted.
- **RFCs** are broader proposals (features, process changes, community
  policies) that benefit from structured review before implementation.

An RFC may result in one or more ADRs if it involves architectural decisions.
