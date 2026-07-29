---
baseline_commit: a414f922
depends_on: 13-3b-provenance-crosses-the-wall (citer-auth pin), 13-6a-authenticated-team-identity (DONE)
sequence_after: 13-6b-production-cross-team-crossing-initiators — NO code dependency; both rewrite `xtask/src/check_multi_tenant_loom.rs`, so they are serialized on the gate file, not on each other's mechanisms
blocks: 13-6-reza-cortex-journey-closer-nfr-scale-5
kernel_grant: NONE — ZERO maos-kernel-core Δ, pin stays 23401. `maos_kernel_core::iac` is a Story-6.5 re-export shim (D-3); adding a public item to `maos-iac` does not move `count_rs_lines` over `maos-kernel-core/src`. **Verify with the gate; do not assume.**
splits_from: 13-6b-production-cross-team-crossing-initiators
---

# Story 13.6d — A cross-wall traceback is a question production can actually ask

Status: **ready-for-dev**

**Kernel-core Δ: ZERO — and that is not the same sentence as "zero delta."** Pin stays **23401** (measured equal at `a414f922`); `xtask/fkcs-baseline.toml` byte-untouched at `23081`. Work lands in `maos-domain`, `maos-iac`, `maos-bin`, and possibly `spirits/`. **No new gate** — legs go on `check-multi-tenant-loom` (13.5c **K5(b)**).

> **Why this is its own story.** Split from 13.6b at the 2026-07-29 preflight (operator-ratified — the fifth split of Epic 13, and the fourth where grounding disproved a story's own scope). The write side and the read side share a gate file and nothing else: different crates, different dead-wire leg, different failure mode. And **D-1 below is not "a caller is missing" — it is "the request cannot express the question,"** which is a different kind of work from wiring an initiator.

---

## Story

**As** a team lead in Reza's single-org Cortex investigating an incident that crossed a team boundary,
**I want** a traceback against a *named remote team* to be a question a running MAOS process can ask — and a refusal to be an answer I can see,
**so that** `recall_cross_wall` and the consent adapter already wired into the composition root govern real queries instead of guarding a trait method no production request can reach.

---

## The defect, in code — measured at `a414f922`

### D-1 — ⚠ the read side is unreachable for a STRUCTURAL reason, not a missing call

`LogRecallPort::recall_cross_wall(spirit_pid, team: &TeamId, filter)` — `crates/maos-domain/src/ports/log_recall.rs:35`, implemented at `crates/maos-iac/src/adapter/log_recall.rs:347`. It is a **default-implemented** trait method whose default returns:

```rust
Err(LogRecallError::ECrossWallRecallDenied {
    team: team.clone(),
    reason: CrossWallRecallRefusal::NoConsentProvider,
})
```

Production dispatch calls plain `.recall(`:
- `spirits/researcher/src/lib.rs:1030` — `port.recall(spirit_pid, filter)`
- `crates/maos-bin/src/main.rs:5881` — inside a one-shot smoke path

**No production surface supplies the `&TeamId`.** `LogRecallFilter` (`crates/maos-domain/src/log_recall.rs:14`) carries `kind` / `since_ns` / `until_ns` / `limit` / … and **no team field**.

⇒ The gap is not "add a caller." It is *"the request cannot express the question."* Anyone scoping this as a wiring job will write a caller with nothing to pass it.

⚠ **`LogRecallFilter` is not a free-for-all struct.** Its own doc says: *"Construct via `LogRecallFilter::new` to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."* Whatever carries the team must go through the validating constructor or a sibling entry point — a new public field that struct literals can set is a regression in that discipline, not a feature.

### D-2 — the consent apparatus is already built and already dead-ended

`main.rs:2958` — `LogRecallAdapter::with_cross_wall_consent(CrossWallRecallConsentAdapter::new(…))`, real signed-manifest state, feeding a method nothing calls. **The "constructed-but-unwired controls fail" shape 13.6·AC1 exists to catch** — the closer cannot judge it, because it would be judging itself.

`xtask/src/check_multi_tenant_loom.rs:525` runs `cross_wall_recall_has_no_production_caller` as `BindingClass::Blocking`. **Green at `a414f922`.**

### D-3 — this is NOT kernel-core

`main.rs:2950` says `maos_kernel_core::iac::log_recall::LogRecallAdapter`, which *reads* like kernel-core. It is not: `crates/maos-kernel-core/src/iac.rs:13` is `pub use maos_iac::*;` — a Story-6.5 backward-compat shim. The adapter lives in `maos-iac`. **Verify with `check-kernel-baseline`, do not assume.**

### D-4 — the emitter-scope pin is load-bearing and must NOT widen as a side effect

13.3b·AC2 ratified the citer-auth control (`distillate.rs:330-359`, originally Story 8.10 AC2b) precisely so a digest-of-digest cannot launder a cross-principal raw frame. Cross-wall reach is granted **only** through the team-named path with its own consent proof. **A mutation that lets plain `recall` cross the wall must red.**

Related and non-negotiable: **`ranged_recall` is forbidden here** — path-addressed, capability-free, and compile-pinned out by `spirits/researcher` (13.3b·AC3). Use the consented `recall_cross_wall` path.

### D-5 — refusal must be an outcome, not an empty page

`ECrossWallRecallDenied` carries a typed `CrossWallRecallRefusal`. ADR-049 §7's provenance-presence discipline says an unconsented or stale-lease recall must surface as an **observable operator outcome**, never folded into an empty-page success. The gate already carries a sibling leg — `cross-wall-recall-refusals-not-journaled` (`check_multi_tenant_loom.rs`, immediately after `:525`) — so the refusal-journaling clause is live and must stay live once the path is reachable.

### D-6 — ⚠ success-side disclosure is NOT journaled, and this story makes it reachable

ADR-058 Decision 2 scoped success-side cross-wall disclosure out (deferred-work 2026-07-21). A **successful** `recall_cross_wall` journals as a plain local `log.recall` — the audit record does not say that data crossed a team boundary. The recommendation on file is to extend the refusal-journaling clause to success-side disclosure.

**While the path was dead this was a paper gap. This story is the introduction.** It is in scope (AC4): a cross-wall *disclosure* that is indistinguishable from a local read in the audit trail is exactly the shape the epic keeps cataloguing — a control that exists for refusals and evaporates for successes.

### D-7 — ⚠ a control's scope condition is invisible while the path is dead

The standing lesson from 13.6a, applied here: **re-read every claim the read path's controls make before wiring it.** `recall_cross_wall`'s default-deny, the citer-auth pin, the refusal-journaling leg and ADR-049 §7 were all written while nothing could call the method. Any qualifier nobody wrote down is the one about to become false. Budget a probe pass for it (13.5j's *probe-harness-before-design*).

---

## Acceptance Criteria (5)

### AC1 — The request can express the question

**Given** D-1: no production request can name a remote team, so the gap is structural,
**When** this story lands,
**Then** the recall request path can carry a target team — either a validated field on `LogRecallFilter` or a sibling entry point in **`maos-domain`** (⚠ **not** kernel-core, D-3),
**And** it is constructed through the **validating** constructor path, never a bare public field a struct literal can set (D-1's own doc discipline),
**And** the shape is chosen deliberately and the reason recorded: a filter field makes every existing call site able to ask a cross-wall question; a sibling entry point keeps the two surfaces separable. **Say which, and why, in the design note before writing.**

### AC2 — A production read-side initiator exists, and consent is proven on the live path

**Given** `CrossWallRecallConsentAdapter` is constructed at `main.rs:2958` and feeds nothing (D-2),
**When** a production surface issues a traceback against a named remote team,
**Then** `LogRecallPort::recall_cross_wall` is reached **in production** with a real `&TeamId`, consent is proven fresh through the already-wired adapter, and an allowed recall returns the remote page,
**And** `cross-wall-recall-no-production-caller` (`check_multi_tenant_loom.rs:525`) is **inverted in the same commit**, with this story named as its owner in the replacement — never left half-deleted (11.3 D2 / 10.4c atomic cutover),
**And** the **13.5g composition-root test applies**: delete the new production initiator and confirm the positive leg **reds**. If it stays green it is testing the library, not the wiring.

### AC3 — Refusal is first-class, and the emitter-scope pin does not widen

**Given** ADR-049 §7 and the citer-auth pin (D-4, D-5),
**When** an unconsented or stale-lease cross-wall recall is issued,
**Then** it surfaces `ECrossWallRecallDenied` with its typed `CrossWallRecallRefusal` as an **observable operator outcome**, never folded into an empty-page success,
**And** unconsented, stale-lease, and no-consent-provider remain **distinguishable from one another** — one generic denial is not three controls,
**And** ⚠ **a mutation that lets plain `recall` cross the wall must red** (D-4). `ranged_recall` stays compile-pinned out (13.3b·AC3),
**And** each limb is **proven-red separately**: a mutation neutering one refusal reds its own leg and leaves the siblings green.

### AC4 — A disclosure that crossed the wall says so in the audit trail

**Given** D-6: a successful `recall_cross_wall` journals as a plain local `log.recall`, so a cross-team disclosure is indistinguishable from a local read,
**When** an allowed cross-wall recall returns a remote page,
**Then** the audit record names it as a **cross-wall disclosure** — the remote team, the consent grant relied on, and the fact that data left the team boundary,
**And** the refusal-journaling clause (`cross-wall-recall-refusals-not-journaled`) stays live and is **not weakened** by the addition,
**And** ⚠ if implementation shows this cannot be done without a kernel-core edit or an ADR-058 amendment, **stop and record it** — do not smuggle it in, and do not silently drop it. Closing ADR-058 Decision 2 is the intent; a measured "this costs a kernel Δ" is an acceptable outcome, a quiet omission is not.

### AC5 — Gate, ADR and budget

**Given** ADR-055 owns the tenant wall and `check-multi-tenant-loom` is its gate,
**When** this story's legs are registered,
**Then** they go on **`check-multi-tenant-loom`** — **no new gate** (13.5c K5(b), re-ratified at 13.5e),
**And** every hermetic leg is `Blocking` with **one `#[test]` per `--exact` leg** (the gate's only anti-vacuity oracle is `"running 1 test"` + `"1 passed"`, structurally blind to a null assertion),
**And** live legs are `AdvisorySubstrate` and **`.expect()` their own env var rather than silently skipping** (13.5g — skipped ≠ passed),
**And** `check-kernel-baseline` proves **23401 unchanged**; if it disagrees, **stop for FLAG-Winston, do not route around the owner**,
**And** ADR-058 is amended (or its Decision 2 closed) with what AC4 landed, and `ABSENT_SUCCESSORS` updated to reflect what this story closes and what it does not,
**And** `xtask/kloc.toml` ceilings for **`maos-domain` (8590)** and **`maos-iac` (6606)** are checked **before** writing, so a breach is a design input rather than a "done"-time re-base — that has happened four consecutive stories and is a live retro item.

---

## Traps

1. **This is not a wiring job (D-1).** The request cannot express the question. Design the request shape first.
2. **`LogRecallFilter` has a validating constructor for a reason.** A new bare public field re-opens the struct-literal bypass its own doc warns about.
3. **Do not widen plain `recall` (D-4).** The citer-auth pin is 13.3b·AC2 ratified and must red on mutation.
4. **`ranged_recall` is forbidden** — path-addressed, capability-free, compile-pinned out.
5. **Not kernel-core (D-3)** — `maos_kernel_core::iac` is a re-export shim. Verify with the gate anyway.
6. **Do not "fix" `main.rs:2402` / `SpiritMemoryView`.** It forwards only to the trait path, a permanent `CapabilityDenied` in production **by design** (`memory/mod.rs:721-739`). 13.5d recorded this trap; 13.6b re-recorded it.
7. **Atomic cutover.** Invert the dead-wire leg and land the positive in the **same commit**.
8. **One `#[test]` per `--exact` leg.** Two tests behind one leg name defeats the oracle.
9. **Proven-red per limb, byte-identical restore** (`diff -q`). Serialize the mutations; do not batch.
10. **Measure the baseline in an isolated `git worktree`**, not a dirty tree (the 13.5i error, fixed at 13.5j). *"Code first, then the pin."*
11. **`cargo run -q -p xtask -- <cmd>`.** There is no `cargo xtask` alias.
12. **Gate-file serialization.** 13.6b also rewrites `check_multi_tenant_loom.rs`. Land after it, or agree a merge plan first.

---

## Tasks

- [ ] **T1 (AC1)** — Probe-harness pass (13.5j): drive `recall_cross_wall` through the public surface until every probe is red; delete before commit. Then decide the request shape — filter field vs sibling entry point — and **record the reason**.
- [ ] **T2 (AC1)** — Implement the team dimension in `maos-domain` behind the validating constructor.
- [ ] **T3 (AC2)** — Add the production surface that supplies the `&TeamId` and calls `recall_cross_wall`; confirm the already-wired `CrossWallRecallConsentAdapter` (`main.rs:2958`) is on the live path.
- [ ] **T4 (AC3)** — Refusal legs: unconsented, stale-lease, no-consent-provider, each observable and each distinguishable. Confirm plain `recall`'s emitter pin is untouched and reds on mutation.
- [ ] **T5 (AC4)** — Cross-wall disclosure journaling; verify the refusal-journaling leg is not weakened. If a kernel Δ is required, **stop and record** rather than proceeding either way.
- [ ] **T6 (AC2/AC5)** — Invert `cross-wall-recall-no-production-caller` (`check_multi_tenant_loom.rs:525`); add the positive leg naming the production caller and the consent proof; one `#[test]` per `--exact` leg; each clause separately named with its own inverter. Run the composition-root deletion test.
- [ ] **T7 (AC3/AC5)** — Serialized proven-red pass: one mutation per limb, own-leg-only red, restore byte-identical. Record each mutation and its observed red leg in the Dev Agent Record.
- [ ] **T8 (AC5)** — Amend ADR-058 / close Decision 2; update `ABSENT_SUCCESSORS`; update `tests/coverage-matrix.yaml` if traceability changes.
- [ ] **T9 (AC5)** — Gates: `check-kernel-baseline` (**23401**), `kloc-check` (`maos-domain` 8590, `maos-iac` 6606 — **check before writing**), `check-multi-tenant-loom`, `check-reza-production-path`, `cargo fmt --all -- --check`, `cargo test --workspace`.
- [ ] **T10** — Record the dev model in the Dev Agent Record (`check-dev-model-tier` / `check-dev-model-used-populated` are live gates; frontier-class allowlist; §A6 full-layer review net **non-degradable** per `epic-13:13`).

---

## Dev Notes

### Existing surfaces to reuse — do not reinvent

| Need | Use | Path |
|---|---|---|
| Cross-wall read (default-deny today) | `LogRecallPort::recall_cross_wall` | trait `maos-domain/src/ports/log_recall.rs:35`; impl `maos-iac/src/adapter/log_recall.rs:347` |
| Request shape to extend | `LogRecallFilter` + `LogRecallFilter::new` | `maos-domain/src/log_recall.rs:14` |
| Cross-wall read consent (already constructed) | `CrossWallRecallConsentAdapter` | `maos-bin/src/cross_team_consent.rs:50`; wired `main.rs:2958` |
| Typed refusal | `LogRecallError::ECrossWallRecallDenied` + `CrossWallRecallRefusal` | `maos-domain/src/ports/log_recall.rs` |
| The dead-wire leg to invert | `cross-wall-recall-no-production-caller` | `xtask/src/check_multi_tenant_loom.rs:525` |
| Sibling leg that must stay live | `cross-wall-recall-refusals-not-journaled` | `check_multi_tenant_loom.rs`, immediately after `:525` |
| Emitter-scope pin that must not widen | citer-auth control | `distillate.rs:330-359` (13.3b·AC2 ← 8.10 AC2b) |

### House standards

- Test idiom: `Command::new(env!("CARGO_BIN_EXE_maos"))`. No `assert_cmd`/`escargot`/`predicates`.
- `cargo fmt --check` blocking since E12-B4; rustfmt `max_width = 100`.
- `cargo-deny` is live — a new dependency needs justification. Prefer none.
- `#[ignore]` + `.expect()` on live legs; the gate controls execution. **Skipped ≠ passed.**

### Budget

- **kernel-core:** ZERO @ **23401**. If the measurement disagrees, stop for FLAG-Winston. *"kernel-core ZERO" is not the same sentence as "zero delta"* — state both.
- **kloc:** `maos-domain` ceiling **8590**, `maos-iac` ceiling **6606**. Ceiling policy is `measured + max(100, ceil(0.02 × measured))`; slack is operating capacity, **not** authorization; a ceiling must never block a correctness repair — surface it rather than routing around it.
- **fkcs:** `xtask/fkcs-baseline.toml` stays byte-untouched at `23081`.

### Previous-story intelligence

- **13.6a (`a414f922`).** Its **honest finding** — an Accept-seam rule that is defense-in-depth behind another control — is the model for reporting a layered control without over-claiming it.
- **13.6b's D-15 and D-16** are worth reading before designing AC4: the kernel discards every collective cause (`memory/mod.rs:204`, `Transport(_)`), and a stale seam comment steered three stories' design. Check what the read path's comments claim before trusting them (D-7).
- **13.5g.** Legs green while connecting to nothing; no leg exercised the composition root. AC2's deletion test is the direct descendant.
- **13.5j.** *Probe-harness-before-design* — T1 is that method.

### References

- [Source: `epic-13-reza-cortex-v2-2.md#52`] — *"13.6 is last and only judges."*
- [Source: `epic-13-reza-cortex-v2-2.md#175`] — separately-named clauses, each with its own inverter.
- [Source: `docs/adr/ADR-058-*.md`] — Decision 2, success-side cross-wall disclosure scoped out.
- [Source: ADR-049 §7] — provenance-presence discipline: a refusal is an outcome, not an empty page.
- [Source: `_bmad-output/implementation-artifacts/deferred-work.md`] — 2026-07-21 success-side disclosure entry.

---

## Residuals (carry forward; do not silently close)

1. **`(f-ii)` was closed by 13.5c; `(f-i)` is 13.6b's; the read-side leg is this story's.** After 13.6b and this story, no Epic-13 dead-wire clause should remain unassigned — **verify that mechanically before asserting it.**
2. **The kernel erases every collective cause** — `kernel-core/src/memory/mod.rs:204`, `CollectivePortError::Transport(_)`. Not this story's path, but the same class of erasure AC3 is guarding against on the recall side. **Owner: 13.6.**
3. **`v25-signed-shard`** — unchanged, not this story's.
4. **`maos-cohort` is absent from `xtask/kloc.toml`** — retro item, recorded at 13.6b.

---

## Dev Agent Record

### Agent Model Used

_(record the frontier-class model; `check-dev-model-used-populated` is a live gate)_

### Debug Log References

### Completion Notes List

### File List

---

## Change Log

| Date | Change |
|---|---|
| 2026-07-29 | **Created — read side split out of 13.6b** (operator-ratified at the post-13.6a preflight; the fifth split of Epic 13). Fault line: the write side and the read side share `check_multi_tenant_loom.rs` and nothing else — different crates (`maos-domain`/`maos-iac` vs `maos-a2a-core`/`maos-loom-lite`), different dead-wire leg (`cross-wall-recall-no-production-caller` vs `replication-crossing-has-no-production-initiator`), different failure mode. D-1 carries the reason it deserved its own story: the read side is unreachable **structurally** — `LogRecallFilter` has no team field, so *the request cannot express the question* — which is a different kind of work from wiring an initiator. Scope also absorbs **ADR-058 Decision 2** (13.6b Residual 3): success-side cross-wall disclosure journals today as a plain local `log.recall`, so a cross-team disclosure is indistinguishable from a local read — a paper gap while the path was dead, live the moment this story lands, and therefore AC4 rather than a residual. 5 ACs, ZERO kernel-core Δ @23401, no new gate. |
