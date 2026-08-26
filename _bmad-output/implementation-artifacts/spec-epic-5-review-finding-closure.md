---
title: 'Close all reopened Epic 5 review findings'
type: 'bugfix'
created: '2026-08-13'
status: 'in-progress'
review_loop_iteration: 0
baseline_commit: '2688c6d09ebe6abab8042ec23e745d737c1afc48'
context:
  - '_bmad-output/implementation-artifacts/epic-5-context.md'
  - '_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md'
  - '_bmad-output/planning-artifacts/architecture-maos-minimal-opus/7-inter-agent-communication.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Current-HEAD audit reopened Stories 5.1, 5.2, 5.4, and 5.5a because their Review Findings still expose runtime panics, false tests/smokes, hot-swap identity loss, revocation bypasses, and fabricated T3 observability.

**Approach:** Close every status-begins-open row with production behavior and adversarial proof, repair the concurrently reproducible revocation-idempotency regression, then synchronize all four stories and Epic 5 to `done` only after the closure gates pass.

## Boundaries & Constraints

**Always:** Preserve ABI discriminants; add typed errors; retain one SCB `Arc`; keep kernel envelope/Spirit CBOR ownership; use one compact recursively key-sorted CRL byte form; fail closed; route budget events to the invoking Spirit via MPSC-32; query live SCBs through authenticated operator HTTP; preserve unrelated worktree changes.

**Ask First:** Destructive persistence, incompatible wire changes, or weakened acceptance floors.

**Never:** Defer a reproducible row; ship placeholders, fake output, global trust state, check-then-act races, reused predecessor objects, test-only proof, or `Co-authored-by`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|---|---|---|---|
| Lifecycle bridge | no Tokio runtime or current-thread runtime | no unwind; verb resolves or returns typed failure | `LifecycleError`, never panic |
| Budget event | hook reaches 80% or exceeds cap | targeted MPSC-32 frame plus Transparency Log row | route failure is surfaced/audited |
| Hot-swap | snapshot failure, adjacent major, rollback | distinct error; migrator runs; same SCB `Arc` and counters survive | restore runtime snapshot on same SCB |
| Revocation | malformed/bad-signature CRL or concurrent apply/load | reject invalid input; one atomic application; matching future load denied | typed fail-closed result |
| Upgrade | hot/cold/migrator policy | factory creates a fresh successor object | predecessor remains/restores on failure |
| T3 inspect | authorized/unauthorized live HTTP query | real SCB report / no disclosure | 401/404 typed CLI failure |
| T3 image | signed lock and repository digest | verified lock and matching `RepoDigests` admission | missing trust, placeholder, malformed, mismatch stay distinct |
| Runtime discovery | Podman and Docker both fail | both diagnostics retained | aggregate unavailable error |

</frozen-after-approval>

## Code Map

- `crates/maos-{spirit-abi,domain,iac}/src/**` -- budget routing and typed contracts.
- `crates/maos-kernel-core/src/{scheduler,hot_swap,revocation,lifecycle,security}/**` -- production lifecycle, swap, revocation, upgrade, and T3 paths.
- `crates/maos-{bin,cli,control,manifest,eval}/**`, kernel tests/benches -- operator surface, corpora, smokes, and NFR proof.
- `.github/workflows/discipline.yml`, `xtask/src/**`, four story files, `sprint-status.yaml` -- closure gates and tracking.

## Tasks & Acceptance

**Execution:**
- [ ] Story 5.1 files -- validate resolver identity, remove sync-runtime panics, add ABI budget frames/payload/MPSC-32 delivery, deduct DRR quantum, fix subsecond warning timing, and make five-verb/budget tests observe hooks and payloads.
- [ ] Story 5.2 files -- close the already-fixed adjacent-major row; add snapshot/expected-version/migrator typed errors; remove in-process envelope roundtrip; preserve one SCB `Arc` with swappable runtime snapshots and stale-clone-safe dispatch.
- [ ] Story 5.4 revocation files -- canonicalize and fully validate signed CRLs; atomically reserve/store validated rules; enforce them during scheduler admission; inject trust anchors; fail closed on unknown actions; widen drain math; own poller shutdown and cadence precedence.
- [ ] Story 5.4 upgrade/evidence files -- inject a fresh-Spirit factory, replace fake smoke/structural tests, materialize and execute all 50 referenced assets, enforce real 10^4-validation p99 ≤5s and upgrade paths, and resolve the all-features timeout without weakening coverage.
- [ ] Story 5.5a files -- correct misleading names/parameters, aggregate runtime probe errors, verify signed non-placeholder locks, compare registry manifest digests, and serve real live sandbox reports through bearer-authenticated operator HTTP with constant-time token comparison.
- [ ] Story/tracker files -- cite paths and commands for every row, close only proven rows, set four stories and Epic 5 to `done`, run closure/coherence gates, and create one no-coauthor commit.

**Acceptance Criteria:**
- Given the reopened row inventory, when closure verification runs, then zero reproducible open/decision rows, zero unsupported closed rows, and zero missing File List references remain.
- Given focused and workspace validation, when tests/gates finish, then behavior, corpus cardinality, security negatives, performance floors, status coherence, formatting, and diagnostics all pass.
- Given the final commit, when its message/trailers and worktree are inspected, then it contains the complete closure, no `Co-authored-by`, and no unrelated modifications.

## Spec Change Log

## Design Notes

Operator HTTP binds loopback by default, requires a bearer token on daemon and CLI, compares it in constant time, and has no fabricated fallback. Revocation admission/idempotency share one atomic rule store. Codec serialization remains at archive/subprocess boundaries; in-process swaps carry the logical envelope.

## Verification

**Commands:**
- `cargo test -p maos-spirit-abi --lib && cargo test -p maos-iac --lib && cargo test -p maos-domain --lib` -- contracts pass.
- `cargo test -p maos-kernel-core --all-features` -- lifecycle, swap, revocation, upgrade, sandbox, and NFR proof pass.
- `cargo test -p maos-manifest && cargo test -p maos-eval && cargo test -p maos-cli && cargo test -p maos-bin --all-features` -- corpora, HTTP/CLI, and smokes pass.
- `cargo test --workspace --all-features && cargo fmt --all --check` -- workspace completes without skipped coverage or formatting diff.
- `./target/debug/xtask check-review-findings-resolved --json && ./target/debug/xtask check-dev-record-completeness --json && ./target/debug/xtask check-epic-close-coherence --json && ./target/debug/xtask check-epic-close-green --json` -- every gate reports `passed: true`.
- `./target/debug/xtask kloc-check --json` -- **ADDED 2026-08-26 by Story `14-0` AC3.1; this is D13's own prescription and it had never been carried out** (`grep -in kloc` on this file returned ZERO hits at `9c5ae2db`). It is the hole that let the two kernel-core instruments disagree for thirteen days: this spec's `af788c3e` took the **FLAG-Winston PIN** grant (`xtask/kernel-core-baseline.toml:465`, anti-DRIFT, physical lines in every `.rs`) and never the paired **CEILING** grant (`xtask/kloc.toml`, anti-GROWTH, tokei CODE with tests/benches/examples/fuzz excluded). The pin read green the whole time while the ceiling sat **685 over**, and no command on this list could see it. D13(a) is now discharged by a founder grant (`kloc.toml:195`, `18248 -> 18933`, exact measured / zero headroom), but taking the grant does not add the gate that would have caught it — this line does.

> ### Provenance gap, recorded rather than fabricated (Story `14-0` AC3.1)
>
> This spec became a `development_status` key on 2026-08-26 so that D13(a)'s deadline
> — *"before `spec-epic-5-review-finding-closure` reaches `done`"* — could be a query
> instead of a judgement. A status no tracker records cannot transition, so that
> deadline was unsatisfiable from the day it was written.
>
> Being tracked also makes this file visible to the five converted story-file gates
> for the first time, and two of them are RIGHT to object: it carries **no
> `dev_model_used`** and **no §A6 review artifact**. That is a real gap and it is
> stated here rather than papered over — `14-0` will not write a `§A6` marker for a
> review that did not happen, and will not name a model it cannot evidence (the
> commit body of `af788c3e` records none, and this file was created by that commit).
>
> The status is therefore `blocked`, which is accurate on its own terms: every Task
> above is unchecked, and the command list now includes `kloc-check`, which is RED at
> HEAD on `maos-domain` (D14) and `_aggregate_hardfail` (D17) — neither of them this
> spec's debt. It cannot reach `done` until those clear. Whoever unblocks it owes the
> two missing records at that point.
