# Story 13.6 evidence publication

## Current status

**PROVEN — operator-lane, clean-commit, four-gate ledger set.**

This file is the current evidence index. Current artifacts, all binding commit
`e38185eb` (clean worktree, no `+worktree:` digest):

- [`evidence-ledger-check-cross-region-consensus.json`](evidence-ledger-check-cross-region-consensus.json) — `product_claim: PROVEN`; all 4 live legs `PROVEN_LIVE_SIGNED`.
- [`evidence-ledger-check-multi-region-slo.json`](evidence-ledger-check-multi-region-slo.json) — `product_claim: PROVEN`.
- [`evidence-ledger-check-multi-tenant-loom.json`](evidence-ledger-check-multi-tenant-loom.json) — `product_claim: PROVEN`; the required `reza-three-team-three-region-journey` leg is `PROVEN_LIVE_SIGNED`: all six processes (3 daemons, `maos run`, `collective-erase`, `traceback`) executed through their production entries; the collective erase reconciled BOTH the destination crossed copy and the source origin row through the manifest-authorized `collective:erase` control; the traceback CLI reached the shipped `CrossWallLogReadAdapter` and returned the exact six-field minimum-disclosure DTO. The `cortex-fourteen-institution-isolation` leg is `PROVEN_LIVE_SIGNED`: fourteen independent institution authorities, signed manifests, physical datnames, typed cross-institution consent refusal, cross-authority clone rejection, and removal independence.
- [`evidence-ledger-check-reza-production-path.json`](evidence-ledger-check-reza-production-path.json) — `product_claim: PROVEN`.

All four ledgers carry the same commit, were produced on one operator run with
the operator audit key, and contain no workstation or key paths.

## Closure repairs since the pre-review rejection

1. **Six-process journey.** Tenant-mode bounded-refreshable classification now
   covers the `traceback` CLI and the cohort-backed `collective-erase`
   one-shot; both reach their production dispatches instead of failing at
   tenant-map construction.
2. **Two-sided erase reconciliation.** `CrossTeamCrossingControl::Erase`
   travels the existing authenticated intake path under its own wire intent
   (`collective:erase`, `IntentClass::Standard` — a share-only route cannot
   carry it), is authorized by the directional `collective:erase` consent
   check, and — closure-review repair — the origin side additionally requires
   its own journaled `collective.host.cross-team-share` provenance matching
   (spirit, requesting team, namespace, key) before any deletion, so a
   team-level grant cannot erase a row that was never shared to the requester.
   Reconciliation ACKs precede the local delete (remote tombstones are
   idempotent, so retries recover), and the local delete targets the exact
   crossed physical row. A typed refusal or transport failure fails the
   one-shot — never a silent one-sided erase. The planted one-sided oracle
   stays RED.
3. **Ledger omission closed (Story 13.6e).**
   `PublishedLedger::validate_against` now requires the exact gate-owned leg
   set, derived per gate from the same declarations the gate executes — no
   second hand-maintained list. Missing and unknown legs fail validation
   before any claim comparison.
4. **14-institution axis.** `cortex_fourteen_institution_isolation_live`
   provisions fourteen independent institutions (distinct authority keys,
   signed V4 manifests, host→team→physical datname bindings) and proves
   isolation, clone rejection, and removal independence on the live substrate.
5. **Chokepoint contract updated.** The team guard now covers seven guarded
   entry points (write, read, scan, crossed-row-origin lookup, generic erase,
   exact crossed-row erase, crossed-row annotation); the `team-guard-chokepoint` leg proves it.
6. **Publication hygiene.** Leg details render repo-relative source paths; no
   operator workstation paths appear outside signed transcript payloads.

## Superseded review and operator history

- [`review-observation-check-multi-tenant-loom.json`](review-observation-check-multi-tenant-loom.json) — the review-lane `CURRENT_REVIEW_OBSERVATION_UNBOUND` record from 2026-08-08, when the journey was still `ABSENT`. Retained for audit; superseded by the PROVEN ledgers above.
- `evidence-ledger-*.pre-review.json` — the rejected pre-review operator
  worktree artifacts, `publication_status: SUPERSEDED_PRE_REVIEW`. Their
  filenames keep them non-ingestible by `load_published_ledgers`. Retained
  only for audit history; they must not be aggregated or treated as evidence
  for the current Story 13.6 state.

## Current verification

- All four substrate gates: `product_claim: PROVEN` @ `e38185eb`, exit 0, every required leg proven.
- `cargo run -q -p xtask -- check-ship-gate-completeness` — PASS; published ledgers consumed without problems.
- `cargo run -q -p xtask -- check-loom-substrate-drift` — PASS.
- `cargo run -q -p xtask -- check-dev-record-completeness --json` — owner sweep gate.
- Full xtask suite — 450 passed, 1 ignored.
