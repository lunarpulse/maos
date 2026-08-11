# Story 13.6 evidence publication

## Current status

**`NOT_PROVEN` — review-corrected, unbound development-lane observation.**

This file is the current evidence index. Current observation:

- [`review-observation-check-multi-tenant-loom.json`](review-observation-check-multi-tenant-loom.json) — `publication_status: CURRENT_REVIEW_OBSERVATION_UNBOUND`; the required `reza-three-team-three-region-journey` is `ABSENT` and `product_claim` is `NOT_PROVEN`.

The gate ran without an operator key or live PostgreSQL substrate. The
observation deliberately publishes no raw dirty-worktree ledger, artifact
references, workstation paths, or key paths: local binding hashes include
untracked bytes before report generation, so copying or annotating that report
would invalidate the digest it carried. A bound publication requires an
immutable clean commit.

## Why the original proof was rejected

The pre-review journey emitted a signed `PASSED` record even though the `collective-erase` and `traceback` child processes failed before reaching their production dispatches. Review removed that attestation. The required journey must remain `ABSENT` until all six specified processes execute through their production entries.

## Superseded operator history

These sanitized artifacts are retained only for audit history. Each carries
`publication_status: SUPERSEDED_PRE_REVIEW`, and every filename includes
`.pre-review` so `load_published_ledgers` rejects it by the gate/filename
invariant:

- [`evidence-ledger-check-multi-tenant-loom.pre-review.json`](evidence-ledger-check-multi-tenant-loom.pre-review.json)
- [`evidence-ledger-check-cross-region-consensus.pre-review.json`](evidence-ledger-check-cross-region-consensus.pre-review.json)
- [`evidence-ledger-check-multi-region-slo.pre-review.json`](evidence-ledger-check-multi-region-slo.pre-review.json)
- [`evidence-ledger-check-reza-production-path.pre-review.json`](evidence-ledger-check-reza-production-path.pre-review.json)

Their `PROVEN` claims bind their recorded rejected pre-review snapshots. They
must not be aggregated or treated as evidence for the corrected Story 13.6
state. Operator-local paths were sanitized outside signed transcript payloads.

## Current verification

- `cargo run -q -p xtask -- check-multi-tenant-loom --json` — local ignored report, exit 0; required journey `ABSENT`; `product_claim: NOT_PROVEN`. The sanitized observation above records these semantics without claiming a dirty-worktree binding.
- `cargo run -q -p xtask -- check-dev-record-completeness --json` — pass; 31 owner assertions, 16 owned-but-deferred rows, zero violations.
- The mandatory-leg omission vulnerability in published-ledger validation is filed against reopened Story 13.6e; Story 13.6's out-of-scope verifier patch was reverted.
