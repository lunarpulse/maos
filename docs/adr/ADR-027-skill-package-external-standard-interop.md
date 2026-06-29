# ADR-027: Skill-Package External-Standard Interop

## Status

Accepted — binding-v0.5.  Gate: Spirit-form adapter loads ≥1 third-party skill
format without kernel modification.

## Context

Skill ecosystems are converging across vendors.  The Anthropic Skills format
(markdown with YAML frontmatter), similar conventions in other agent frameworks,
and the MAOS `maos.skill.v1` format (markdown with TOML frontmatter) share a
structural similarity: a machine-readable metadata header followed by a
human-readable instruction body.

Two questions led to this ADR:

1. **Should MAOS adopt a third-party format wholesale?**  Adoption would
   relinquish control over admission semantics (FR39 operator-approval, FR57
   revision proposals, `deny_unknown_fields` strictness).

2. **Should MAOS define a wholly novel format?**  A novel format forces every
   skill author to re-learn conventions and prevents ecosystem portability.

## Decision

Skills are markdown with TOML frontmatter conforming to `maos.skill.v1`.  The
format is **intentionally close to** (but distinct from) the Anthropic Skills
format and similar emerging conventions.  A Spirit-form adapter can load at
least one third-party skill format **without kernel modification**.

The adapter lives in `maos-skill` (the skill ecosystem crate), NOT in
`maos-kernel-core`.  It bridges the third-party frontmatter (e.g. YAML) to the
`maos.skill.v1` `SkillManifest` struct and feeds the result through the existing
`SkillAdmissionQueue` — the kernel's admission flow is unchanged.

## Rationale

- The substrate supports ecosystem convergence by making `maos.skill.v1` close
  to the dominant external standards while retaining the kernel-mediated
  admission flow.
- The adapter pattern (bridge format → `SkillManifest` → admission queue) keeps
  the kernel's non-interpretability invariant (§4.0.7) intact: the kernel
  validates schema, never content.
- TOML frontmatter in `maos.skill.v1` gives the substrate `deny_unknown_fields`
  strictness that YAML's permissive defaults would not provide.

## Alternatives Considered

- **Adopt Anthropic Skills format wholesale** — rejected: gives up control over
  admission semantics and `deny_unknown_fields` correctness.
- **Define a wholly novel format** — rejected: forces every author to re-learn
  and prevents ecosystem portability.

## What Would Force a Revisit

A dominant skill format emerges that `maos.skill.v1` cannot interop with cleanly.

## Consequences

- `maos-skill` hosts the adapter; `maos-kernel-core` is unchanged.
- The adapter is tested via NFR-Test-10 conformance gate
  (`check-skill-conformance`).
- Third-party skill fixtures are committed under `tests/fixtures/` for
  reproducible CI validation.
