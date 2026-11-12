# 20-14 closure note — 2026-11-12 (record-only; no code changed by this closure)

Final 20-14 commit: **9996fe09** (`20-14 L2 correction round 2`). Landings: L1 f68c7de6 (+ corrections 2ae11192, 553336f6, fc7373f0;
r3 reviews persisted 870117ee); L2 **edeedd25**; round 1 **456793ca**; round 2 **9996fe09**.

## Hosted proof (observed 2026-11-12 via `gh run view 43140995903 --repo lunarpulse/maos`)
Workflow `discipline`, event `push`, headSha `9996fe09256dc6dc9249a3fd72e093ac6c66218a`, conclusion `success`, 199 jobs, 0 non-success; the
`abi-diff` job is `success`. Caveat: the proof is that run on 9996fe09; descendants of 9996fe09 are not separately proven for 20-14, no
`prepush` ran on this closure, the Apple and Windows legs were not run, and I did not read the job logs (so which hosted job exercises the
isolated-HOME alice/bob bin test is unobserved).

## Reviews (persisted verbatim)
| Round | Reviewer | Result | File |
|---|---|---|---|
| L2 r1 | GPT gpt-6.1-sol | 3 P2 (fixed in 456793ca) | `landing-2/review/gpt-review.md` |
| L2 r1 | GLM glm-5.3 security | S1 Low, S2 Low, S3 Info | `landing-2/review/security.md` |
| L2 r2 | GPT gpt-6.1-sol | 1 P1, 1 P2 (fixed in 9996fe09) | `landing-2/review/gpt-review-r2.md` |
| L2 r3 | GPT gpt-6.1-sol on 9996fe09 | 0 P0/P1/P3, ONE P2 (KLOC approval citation) | `landing-2/review-r3/gpt-review.md` |
| L2 r3 | **GPT reviewer agent (not GLM)**; security | 0 findings | `landing-2/review-r3/security-gpt-reviewer.md` |

The round-3 security pass ran on a GPT reviewer agent because GLM was unavailable (weekly quota exhausted). The round-1 security review was
GLM glm-5.3; the round-2/3 security coverage of 9996fe09 is therefore by a different model family than the standing policy, and is recorded as such.

## The round-3 P2 (KLOC approval citation) — resolved honestly
`closure/kloc-adjudication-2026-11-12.md` records who adjudicated (Main/orchestrator, disclosed to the operator in the 2026-11-12 progress
reports, operator did not object), on which standing rule (operator-ratified recovery-lane ceiling rule 2026-09-12, `xtask/kloc.toml` header),
the per-crate numbers at landing and now, and what is NOT claimed (no operator numeric ruling exists; kernel unchanged at its own ledger).
`landing-2/review-r2/evidence.md:31` and the `kloc.toml` HISTORY words "Main-adjudicated" / "PENDING Main re-adjudication" are read through that note;
persisted evidence was not rewritten. No code, kloc or kernel pin changed by this closure.

## AC table
| AC | Status | Evidence |
|---|---|---|
| AC4 report axes | LANDED (f68c7de6 + 2ae11192/553336f6/fc7373f0) | `landing-1/evidence.md`, `landing-1/review/correction-round{2,3}.md` |
| AC5 docs / ADR-046 | LANDED (L1; re-touched in L2) | `crates/maos-cli/tests/cost_docs_20_14.rs` |
| AC2 per-task kernel scope | LANDED (edeedd25, 456793ca, 9996fe09) | kernel `cost_attribution_20_14.rs` (6), capability `cost_scope_20_14.rs`, bin `cost_attribution_20_14.rs` (11) |
| AC3 session principal | LANDED (456793ca, 9996fe09) | `MAOS_SESSION_PRINCIPAL`, caps, codec, shared replay predicate; plants under `landing-2/{plants,review/plants,review-r2/plants}` |
| AC1 command | LANDED; hosted run green on 9996fe09 | `crates/maos-bin/tests/cost_attribution_20_14.rs:105` (real `maos` + `maosctl`, isolated HOME alice/bob) |
| AC6.1 coverage rows | LANDED, wording updated at closure | `tests/coverage-matrix.yaml` FR64 / NFR-Cost-1 |
| AC6.2 ledger closures | LANDED at closure | `deferred-work.md` FR64 CLOSED, NFR-Cost-1 re-scoped, round-2 rows dispositioned |
| AC6.3 landing records | LANDED | `xtask/kloc.toml`, `kernel-core-baseline.toml`, `20-14-flag-winston-decision.md` |
| AC2.7 / AC3.7 | N/A (D-04 A) | |
| NFR-Cost-1 >= 98 % invoice reconciliation | CLAIM-BOUNDARY: not claimed | ownerless, operational, D-11 A; rule 10 |
| Worker CLI egress spend | CLAIM-BOUNDARY: not claimed | D-20-14-06 A, deferred row stays open |

## Honest caveats
- No `prepush`; Apple and Windows legs not run; no clean-tree local `abi-diff` (hosted `abi-diff` is the evidence).
- Hosted proof is run 43140995903 on 9996fe09; later commits are not separately proven for 20-14.
- Two million-row pending cases remain the known `20-7` timing family (`ops-stable-perf-runner`).
- Round-2/3 security review is by a GPT agent (GLM quota); round 1 was GLM.
- Residual Low/Info items stay open as rule-10 rows in `deferred-work.md` (do not claim complete Cf refusal, matcher normalization, Spirit-name columns, etc.).
