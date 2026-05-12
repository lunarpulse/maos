# Corpus Extension / Rebuild Runbook

This directory anchors the no-update-justification PR flow for extending
`valid_until` dates on committed corpora.

## When to extend

A corpus's `valid_until` date is approaching (within 30 days) or has expired.
The `cargo xtask corpus-staleness` gate will emit a warning at T-30 and fail
at T+0.

## Required artifacts

1. **Manifest update:** bump `valid_until` in `tests/corpora/MANIFEST.toml`
   (or `tests/coverage-matrix.yaml` for coverage rows).
2. **Assessor sign-off:** a PR description that includes:
   - The rationale for extension vs. rebuild.
   - A statement that the corpus content has been reviewed for drift.
   - Explicit request for two maintainer approvals.
3. **Justification file (optional at v0.1-alpha):** for complex extensions,
   create `docs/corpus-extensions/<corpus-id>.md` documenting the review
   findings. This becomes mandatory when Story 0.5 mechanizes audit-rubric
   tracking.

## Process

1. Open a PR titled `corpus-ext: <corpus-id> valid_until YYYY-MM-DD`.
2. Tag two maintainers for review.
3. Merge is blocked until both approvals are granted (operator-side ceremony,
   not enforced by xtask logic at v0.1-alpha).
