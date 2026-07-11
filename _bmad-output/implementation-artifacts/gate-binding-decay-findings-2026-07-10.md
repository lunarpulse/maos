# Findings note — the v2.0 gate suite is advisory at HEAD (gate-binding decay)

**Status:** findings note / decision input — for the Epic 12 retro (or a `correct-course` before it)
**Date:** 2026-07-10
**Found by:** Story 12.1 preflight, Round 3 (RR11) — party-mode + code-verification agents
**Severity:** systemic (process/CI), **not** a code-correctness bug — no runtime behavior is wrong; the *enforcement* is
**Kernel-Δ:** none implied by any option here except a phase advance, which is xtask/CI-only regardless

---

## TL;DR

7 of the project's hardest-won discipline gates — authored across Epics 8–11 to **block** — are currently **advisory at HEAD**. They are wired into the ship gate, they run their real legs, but a **real red returns `Ok()`** and prints a `WOULD-HAVE-BLOCKED` banner. They protect nothing on a normal push. This is the exact *"mechanical gates compound; promises decay"* pattern the retros keep flagging, now measured precisely.

The cause is benign and mechanical: `CURRENT_PHASE = "v1_5"` (the project's **ship-readiness** phase, held there by v1.5's two external GA items), while these gates were dispositioned to bind at `v2_0`. The phase never advanced, so they never started binding. **Dev-time enforcement was accidentally coupled to GA-ship phase.**

**Recommendation:** decouple the two axes. Introduce a **leg-level binding class** (always / capability-gated / substrate-gated) that governs dev-enforcement independent of the ship phase, so hermetic legs bind the moment they can run. This generalizes Story 12.1's one-off RR11 carve-out into the framework instead of copying it 7 times. **Do not** naively advance `CURRENT_PHASE` — it would hard-fail CI on the 3 substrate gates it can't provision.

---

## How it surfaced

Story 12.1 (cohort mesh) specified a new `check-cohort-mesh` gate that should "block per-commit." Round-2 preflight claimed it would "hard-block by construction, like the 11.4b seccomp leg." Round-3 verification (RR11) found that claim false against the code:

- A gate leg that **ran and failed** at `CURRENT_PHASE="v1_5"` flows to the advisory branch and returns **`Ok(())`** (`check_scale_churn.rs` `run()` verdict path; exit plumbing at `xtask/src/main.rs:1213` — only `Err` fails the job).
- The only phase-independent hard-fail is the **vacuous-green guard** (`check_scale_churn.rs:365-383`), which fires **only on 0-test legs**, never on a real red.
- The cited precedent (`check-escape-detector`, the 11.4b seccomp gate) is **itself advisory at v1_5** despite a docstring that calls it "a real per-commit tripwire."

Pulling that thread produced the inventory below.

---

## Evidence — gate inventory (`xtask/gate-registry.toml`, `.github/workflows/discipline.yml`)

**28 gates carry phase dispositions. 18 block now, 10 are advisory now.** `CURRENT_PHASE="v1_5"` across all 9 phased gate modules — **no drift**.

Effective behavior at `v1_5` is set by the `v1_5` disposition value (`advisory` ⇒ real red passes; `blocking`/`blocking-when-present` ⇒ real red fails CI).

### The 10 advisory-now gates, split by *why* they're advisory

| Gate | Disposition (v1_0/v1_5/v2_0) | Class | Verdict |
|------|------------------------------|-------|---------|
| check-wasm-form-equiv | adv / adv / blocking | hermetic (in-proc `maos-wasm-host` harness + fault-inject) | **DECAY — should block** |
| check-scale-churn | adv / adv / blocking | hermetic (in-proc `127.0.0.1` sockets, `--ignored` in CI) | **DECAY — should block** |
| check-enterprise-pdp | adv / adv / blocking | hermetic (Cedar in-process; module claims "every leg a real tripwire") | **DECAY — should block** |
| check-enterprise-identity | adv / adv / blocking | hermetic (in-proc SSO/OIDC, KMS seal, SIEM redaction) | **DECAY — should block** |
| check-fkcs | adv / adv / blocking | hermetic (frozen-tag, diff-oracle, proxy-cohort, kernel-abi) | **DECAY — should block** |
| check-trial-attestation | adv / adv / blocking | hermetic (reload/sbom/signing-chain/halt derive + blind negative control) | **DECAY — should block** |
| check-escape-detector | adv / adv / blocking | **capability**-gated (seccomp; present on CI `ubuntu-latest`, absent on some dev hosts) | **DECAY on CI — should block when capable** |
| check-cross-form-equiv | adv / adv / — (never graduates) | substrate (measurement engagement pending; no `CURRENT_PHASE` const at all) | correctly advisory |
| check-cross-region-consensus | adv / adv / blocking | substrate (live `MAOS_TEST_POSTGRES`) | correctly advisory |
| check-multi-region-slo | adv / adv / blocking | substrate (3× Postgres `MAOS_TEST_POSTGRES_{A,B,C}`, geo) | correctly advisory |

**Count: 10 advisory-now = 7 that should be binding (6 hermetic + escape-detector capability-gated) + 3 correctly-advisory (substrate/engagement).**

### What is NOT broken (so the note is fair)

- The **vacuous-green guard works** — a leg that runs 0 tests hard-fails at *every* phase (a silent skip cannot pass green). Present across the advisory gates (e.g. `check_wasm_form_equiv.rs:239`, `check_cross_region_consensus.rs:337-347`).
- **18 gates do block now** — export-control, fuzz-targets/floor, ko-coverage, migration-merkle, rto-drill, live-bilateral-consent, rotation-real-timing, j4-latency, skill-conformance, windows-check, pentest/red-team/third-party-trial (blocking-when-present), etc.
- `kernel-abi-diff` legs hard-fail unconditionally in every gate.
- The 3 substrate gates are advisory **for the right reason** — CI cannot provision Postgres / 3-region geo / an external measurement engagement. They should stay advisory until CI gains that substrate.

So the decay is bounded and specific: **7 gates whose legs CI can already run, silently not enforcing.**

---

## Why it happened (root cause)

The phase ladder conflates **two orthogonal axes**:

1. **Ship/GA readiness** — "what release has cleared its external holds." Correctly at `v1_5` (held on real pen-test NFR-Sec-7 + export counsel NFR-Comp-1, the founder-decoupled GA-ledger items).
2. **Dev-time enforcement** — "which regressions must fail a push *right now*." Should be: every leg CI can actually run.

Binding was tied to axis (1). Gates authored during v2.0 (Epic 11) were dispositioned `v2_0=blocking`, correct for a *ship* posture — but because the project's ship phase legitimately stayed at `v1_5`, they never began enforcing at dev time. A hermetic regression is a regression whether you are shipping v1.5 or building v2.2; it should not wait on a GA phase it has nothing to do with.

---

## Options

**Option A — advance `CURRENT_PHASE` to `v2_0`.**
Flips all 10 advisory gates to blocking. ✗ **Breaks CI**: the 3 substrate gates (`cross-region-consensus`, `multi-region-slo`, `cross-form-equiv`) would hard-fail in a CI that can't provision Postgres/geo/engagement — forcing `continue-on-error` hacks (the silent-disable the retros ban). Also misrepresents GA readiness (v1.5 hasn't shipped). If chosen, it **must** be paired with re-dispositioning the 3 substrate gates to substrate-gated. Not recommended alone.

**Option B — per-gate RR11 carve-out ×7.**
Copy Story 12.1's `if !leg.green { return Err }` carve-out into the 7 should-bind gates' hermetic legs. ✓ Surgical, no phase change, restores enforcement where CI can run. ✗ Seven bespoke copies of the same mechanism; escape-detector's seccomp-capability case needs a variant; drift risk as gates are added.

**Option C — leg-level binding class in the shared framework (recommended).**
Add a `binding` marker to the leg abstraction in `xtask/src/gate_common.rs`:
- `AlwaysBinding` — pure hermetic legs: hard-fail on `!green` at every phase (the RR11 carve-out, promoted to a primitive).
- `CapabilityGated(cap)` — bind when the capability is present (seccomp on CI), advisory + marker when absent (generalizes what `check-escape-detector` already hand-rolls).
- `SubstrateGated(env)` — advisory until the substrate/env is present, then binding (the current Postgres/geo behavior, made explicit).

The **phase ladder then governs only the ship-gate disposition** (GA posture), while the **binding class governs dev-enforcement** — the two axes finally separated. Story 12.1's carve-out becomes the first consumer of `AlwaysBinding` instead of a one-off. ✓ One mechanism, self-documenting per leg, no phase change, correct treatment of all three classes. ✗ A few hours of framework work + re-marking existing legs.

---

## Recommendation

1. **Adopt Option C** — leg-level binding class in `gate_common.rs`. It is the durable fix and it retires the copy-paste risk in Options A/B.
2. **Re-mark the 7 decay gates' hermetic legs `AlwaysBinding`** (6 hermetic + `escape-detector` as `CapabilityGated(seccomp)`), restoring enforcement this cycle.
3. **Leave the 3 substrate gates `SubstrateGated`** — advisory until CI provisions Postgres/geo/engagement. Track "CI substrate for the live legs" as its own backlog item (it is the real blocker to those gates ever biting).
4. **Keep the ship-phase decision separate and deliberate.** Advance `v1_5 → v2_0 → v2_2` only in a GA/ship ceremony when v1.5 clears its external holds — and **centralize `CURRENT_PHASE`** (currently a per-module const duplicated ×9) into one place first, so a future advance can't drift 8-of-9 modules silently.
5. **Story 12.1 note:** its RR11 carve-out should be written *as* the `AlwaysBinding` primitive if Option C lands before 12.1 dev; otherwise 12.1 ships the one-off and it is refactored into the primitive when C lands. Either way `check-cohort-mesh` binds at HEAD — it should not be the *only* gate that does.

---

## Decision needed

- [ ] Ratify Option C (binding class) vs B (per-gate carve-out) vs A (phase advance + substrate re-disposition).
- [ ] Owner + cycle for re-marking the 7 decay gates.
- [ ] Backlog item: CI substrate (Postgres / 3-region / measurement engagement) for the 3 substrate gates.
- [ ] Backlog item: centralize `CURRENT_PHASE`; define the ship-phase advance ceremony and its trigger (v1.5 external holds cleared).

---

### Appendix — provenance
- RR11 (mechanism) and the inventory: verified against `xtask/gate-registry.toml`, `xtask/src/check_*.rs`, `.github/workflows/discipline.yml` at branch `epic-11` HEAD, 2026-07-10.
- Related retro carry-forwards: *mechanical gates compound; promises decay* (Epic 4 §A6/§A7); E11 retro §A2 (gate-split, hermetic-blocking / substrate-advisory); E8 retro (green-at-HEAD reached by disabling gates — the decay precedent).
- Related: Story 12.1 preflight Rounds 1–3 (`_bmad-output/implementation-artifacts/12-1-cohort-manifest-full-pairwise-mesh-foundation.md`), party memlog 2026-07-10.
