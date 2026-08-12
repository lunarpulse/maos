---
baseline_commit: a414f922 (defects measured here) — **branch point is `05e7e967`** (13.6b committed 2026-07-29). Gate state re-verified at `05e7e967`: `check-kernel-baseline` **PASSED 23401==23401**, `check-multi-tenant-loom` **PASSED** (hermetic legs bind; 12 live legs advisory-skipped, no Postgres substrate), `kloc-check` **RED — see Residual 6**, `check-service-boundary` / `check-dev-model-tier` **RED, pre-existing and not this story's** (see Residual 7).
depends_on: 13-3b-provenance-crosses-the-wall (citer-auth pin), 13-6a-authenticated-team-identity (DONE), 13-5e-tenant-audit-isolation (per-team TL artifact + `tenant_binding` — the read substrate, D-9)
sequence_after: 13-6b-production-cross-team-crossing-initiators — NO code dependency on its mechanism; both rewrite `xtask/src/check_multi_tenant_loom.rs` and both consume `maos-bin` kloc reserve, so they are serialized on the gate file AND on the budget
blocks: 13-6-reza-cortex-journey-closer-nfr-scale-5
kernel_grant: NONE — ZERO maos-kernel-core Δ, pin stays 23401. `maos_kernel_core::iac` is a Story-6.5 re-export shim (D-3); adding a public item to `maos-iac`/`maos-domain` does not move `count_rs_lines` over `maos-kernel-core/src`. **Verify with the gate; do not assume.**
splits_from: 13-6b-production-cross-team-crossing-initiators
---

# Story 13.6d — A cross-wall traceback is a question production can actually ask, and an answer that came from the other side of the wall

Status: **ready-for-dev**

**Kernel-core Δ: ZERO — and that is not the same sentence as "zero delta."** Pin stays **23401** (measured equal at `a414f922`); `xtask/fkcs-baseline.toml` byte-untouched at `23081`. Work lands in `maos-domain`, `maos-iac`, `maos-bin`, `maos-audit`. **No new gate** — legs go on `check-multi-tenant-loom` (13.5c **K5(b)**).

> **Why this is its own story.** Split from 13.6b at the 2026-07-29 preflight (operator-ratified — the fifth split of Epic 13). The write side and the read side share a gate file, a budget, and nothing else.

---

## ⚠ HEADLINE REVERSAL — read D-8 before anything else

**The 2026-07-29 post-13.6b preflight inverted this story's founding premise.** The first draft said the read side was unreachable because *"the request cannot express the question."* That is true and it is the **second** problem. The first problem is **D-8: `recall_cross_wall` does not read across the wall.** Its body ends in `self.recall(spirit_pid, filter)` — the local, emitter-scoped read. The `team` argument reaches `consent.decide(team, "log:recall")` and **nothing else**. There is no remote read anywhere in the adapter, and a **green Blocking leg asserts the local delegation as correct**.

So the shape is not "wire an initiator to a governed method." It is: **build the read, prove it is not the caller's own rows, and only then wire the initiator** — in that commit order, because every commit that wires a production traceback before the read exists ships a surface that lies to an operator.

Operator-ratified at the 2026-07-29 preflight: **absorb, do not split a sixth time.** One story, AC2 grows, Epic 13 stays at 20 stories.

---

## Story

**As** a team lead in Reza's single-org Cortex investigating an incident that crossed a team boundary,
**I want** a traceback against a *named remote team* to be a question a running MAOS process can ask, **answered with that team's rows**, and a refusal to be an answer I can see,
**so that** `recall_cross_wall` governs a real disclosure instead of consenting to a local read under a remote label.

---

## The defect, in code — measured at `a414f922`, budget re-measured 2026-07-29

### D-8 — ⚠⚠ THE HEADLINE: the capability's `team` parameter is decorative

`crates/maos-iac/src/adapter/log_recall.rs:346-376`. After proving consent, the method's last line is:

```rust
        self.recall(spirit_pid, filter)
```

`recall` is the **emitter-scoped local** read (`:270`, test `recall_emitter_scope_only_returns_own_frames`). `LogRecallAdapter` holds exactly one `Arc<TransparencyLog>` — the local one. **`team` is consumed only by `consent.decide`; it never reaches a query.**

And this is **specified, not overlooked.** `log_recall.rs:733-745` — inside `cross_wall_recall_has_five_distinguishable_outcomes`, the test behind the **`BindingClass::Blocking`** leg `cross-wall-recall-refusal-distinguishable`:

```rust
// Granted AND non-empty: the method must forward spirit_pid/filter to
// `recall` and return the frames …
assert_eq!(page.entries.len(), 2, "a granted cross-wall recall returns the target pid's frames");
```

**"the target pid's frames."** Pid — not team. Two rows out of the same in-memory log the caller already owns; `remote_team = "team-a"` is a string with no data behind it.

⇒ **Consequence if this story had shipped as drafted:** an operator investigating a cross-team incident issues a traceback against team-b, receives **their own team's rows**, correctly consented, journaled as `log.recall`, exit 0 — and concludes team-b is clean. New failure shape for the epic's catalogue, distinct from *constructed-but-unwired* and from *a control standing next to the decision it doesn't govern*: ***a capability whose parameter is decorative*** — the argument that names the operation participates in the authorization and never in the operation.

⇒ AC2's original clause *"an allowed recall returns the remote page"* was therefore (i) unbuildable by wiring and (ii) **in direct contradiction with a green Blocking leg**.

### D-9 — the read substrate DOES exist, locally, and 13.5e ships a control that refuses it

Not a blocker — a design constraint, and a good one.

- **Path derivation:** `maos_audit::transparency_log_path_for_tenant_mode(postgres_present: bool, home_team: Option<&str>)` — called at `crates/maos-bin/src/main.rs:88-98` (`resolved_transparency_log_path`), followed by `maos_audit::validate_transparency_log_path`. **The same function derives the artifact path for *any* team name.** 13.5e sharded the TL on the team axis; the shards sit on one host, one per team (`tenant_mode_path_shards_only_when_both_inputs_are_present`, `maos-audit/src/lib.rs:2069`).
- **Identity:** the shard carries an in-artifact `tenant_binding` row (`team_id`, `datname`, `bound_at_ns`) — `maos-audit/src/lib.rs:1009-1200`.
- **And the wall is already enforced there:** 13.5e ships `TenantBindingError` — *"artifact tenant_binding is bound to {bound}, but env team is {env}"* (`:1251`) — a control that **refuses to open a foreign-team artifact**.

⇒ The consented cross-wall entry point must be the **sole permitted foreign open**, and the ordinary boot path must keep refusing. That pairing is a control, and AC2 asserts both halves.

⇒ ⚠ **Crate-boundary fact that determines where the adapter lives:** `maos-iac/Cargo.toml` depends on `maos-domain`, `maos-spirit-abi`, `maos-capability`, `maos-attrs` — **not `maos-audit`**. `transparency_log_path_for_tenant_mode` is unreachable from `maos-iac`, and ADR-058 forbids `maos-iac → maos-cohort` / `maos-loom-lite`. `maos-bin` has all three. ⇒ **dependency-inverted port in `maos-domain`, adapter in `maos-bin`** — the 12.3 P5r / 13.6b D-8 pattern, third consecutive story. **Do not add a `maos-iac → maos-audit` edge** (`check-service-boundary`, `check-empty-kernel --path crates/maos-iac`).

### D-1 — the request is also unable to express the question (the *second* problem)

`LogRecallPort::recall_cross_wall(spirit_pid, team: &TeamId, filter)` — trait `crates/maos-domain/src/ports/log_recall.rs:35`, impl `crates/maos-iac/src/adapter/log_recall.rs:347`. Default impl returns `ECrossWallRecallDenied{ NoConsentProvider }`.

Production dispatch calls plain `.recall(`:
- `spirits/researcher/src/lib.rs:1030` — `port.recall(spirit_pid, filter)`
- `crates/maos-bin/src/main.rs:5897` — inside the one-shot smoke path

`LogRecallFilter` (`crates/maos-domain/src/log_recall.rs:14`) carries `kind` / `since_ns` / `until_ns` / `limit` / `cursor` / `intent_filter` and **no team field**.

⚠ `LogRecallFilter` is not a free-for-all struct. Its own doc: *"Construct via `LogRecallFilter::new` to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."* A new bare public field re-opens that bypass. And see **D-11** — the shape choice is not open.

### D-2 — the consent apparatus is built, dead-ended, and **conditionally injected**

`main.rs:2965-2980` — `LogRecallAdapter::with_cross_wall_consent(CrossWallRecallConsentAdapter::new(…))`, real signed-manifest state, feeding a method nothing calls. **The "constructed-but-unwired controls fail" shape 13.6·AC1 exists to catch** — the closer cannot judge it, because it would be judging itself.

⚠ **CORRECTED (D-12):** the injection is **not unconditional**. `main.rs:2968`:

```rust
if let (Some(bootstrap), Ok(home_team)) =
    (cohort_daemon.as_ref(), std::env::var("MAOS_LOOM_HOME_TEAM"))
```

ADR-058 documents this in its limits section: *"The manifest-backed consent adapter is injected only when verified cohort state and `MAOS_LOOM_HOME_TEAM` are both available; otherwise the builder remains absent and the method fails closed."* Decision 2's word "unconditional" describes the **builder** (not `cfg`-gated), not the call.

⇒ In any other boot, every cross-wall call returns `NoConsentProvider` and a developer can watch a "denial" that proves nothing. **13.5c's line: BOOTS ≠ SERVES.** AC3's deletion test must run in a boot where the provider is actually attached, and the two preconditions must be named in the test.

The dead-wire leg: `cross-wall-recall-no-production-caller`, `BindingClass::Blocking` — **`xtask/src/check_multi_tenant_loom.rs:872` at the branch point `05e7e967`** (`:525` at `a414f922`). Verified green at `05e7e967`.

⚠ **All four cross-wall legs moved when 13.6b landed. Use the branch-point column, not the `a414f922` column:**

| Leg | @`a414f922` | **@`05e7e967` (branch point)** |
|---|---|---|
| `cross-wall-recall-refusal-distinguishable` | `:469` | **`:816`** |
| `cross-wall-recall-manifest-direction` | `:482` | **`:829`** |
| `cross-wall-recall-no-production-caller` | `:525` | **`:872`** |
| `cross-wall-recall-refusals-not-journaled` | `:541` | **`:888`** |

### D-3 — this is NOT kernel-core

`main.rs:2967` says `maos_kernel_core::iac::log_recall::LogRecallAdapter`, which *reads* like kernel-core. It is not: `crates/maos-kernel-core/src/iac.rs:13` is `pub use maos_iac::*;` — a Story-6.5 backward-compat shim. **Verify with `check-kernel-baseline`, do not assume.**

### D-4 — the emitter-scope pin is load-bearing and must NOT widen as a side effect

13.3b·AC2 ratified the citer-auth control (`distillate.rs:330-359`, originally Story 8.10 AC2b) precisely so a digest-of-digest cannot launder a cross-principal raw frame. Cross-wall reach is granted **only** through the team-named path with its own consent proof. **A mutation that lets plain `recall` cross the wall must red.**

Related and non-negotiable: **`ranged_recall` is forbidden here** — path-addressed, capability-free, and compile-pinned out by `spirits/researcher` (13.3b·AC3).

### D-5 — ⚠ CORRECTED: the refusal-journaling leg asserts the **ABSENCE** of journaling, and its owner is DONE

The first draft read this leg backwards. `crates/maos-bin/tests/cross_team_consent_13_3.rs:262-276`:

```rust
fn cross_wall_recall_refusals_not_journaled() {
    let method = adapter.split("fn recall_cross_wall").nth(1)
        .and_then(|tail| tail.split("fn fetch").next()).expect(…);
    assert!(!method.contains("insert_frame_event"),
        "cross-wall-recall-refusals-not-journaled inverted: Story 13.5e must \
         replace this dead-wire assertion with per-team refusal-audit coverage");
}
```

Three facts:
1. It is a **dead-wire marker forbidding journaling**, not a control requiring it. Any `insert_frame_event` in the `fn recall_cross_wall` → `fn fetch` source span **reds it**.
2. Its named replacement owner is **Story 13.5e — which is DONE and did not replace it.** An orphaned dead-wire leg with a dead owner, listed in the first draft's own Dev Notes as *"sibling leg that must stay live."*
3. ADR-058 explicitly forbids the first draft's reading: *"Per-team TL isolation and refusal journaling remain outside this decision. **They must not be claimed by a gate or operational document until implemented.**"*

⇒ 13.6b's **D-16 shape one story later** — a stale marker steering the design. Last time a code comment; this time a **test failure message**, which is worse, because a failure message reads like a specification. Resolved by AC5 (re-own + atomic cutover), operator-ratified.

ADR-049 §7's provenance-presence discipline still holds: an unconsented or stale-lease recall must surface as an **observable operator outcome**, never folded into an empty-page success.

### D-6 — ⚠ CORRECTED premise: today's audit row is *accurate*, and AC5 is what makes it a lie

The first draft said *"a successful `recall_cross_wall` journals as a plain local `log.recall` — the audit record does not say that data crossed a team boundary."* Both halves are literally true, **and the reason matters**: the row is emitted by the *local* read the method delegates to (`log_recall.rs:275-287` — `CapabilityInvocation` + `LOG_RECALL_INTENT = "log.recall"`, FR4 "audit BEFORE data-movement"). Given D-8, **no data crossed, so the audit trail is currently correct.**

⇒ Stamping *"crossed the team boundary"* on today's behaviour would put a false claim on a truthful row. **AC5 becomes correct only once AC2 lands** — that is the ordering, and it is why AC5's clauses are written against the new read, not the old delegation. ADR-058 Decision 2 is closed by AC5, not asserted by it.

### D-7 — ⚠ a control's scope condition is invisible while the path is dead

13.6a's standing lesson, and D-5 / D-6 / D-8 are three instances of it in one file: `recall_cross_wall`'s default-deny, the citer-auth pin, the refusal-journaling leg and ADR-049 §7 were all written while nothing could call the method. **The qualifier nobody wrote down is the one about to become false.** T1 is the probe pass (13.5j *probe-harness-before-design*).

### D-10 — AC's refusal taxonomy under-counted a shipped five-way control

`CrossWallRecallRefusal` (`crates/maos-domain/src/log_recall.rs:247-261`) has **five** variants, all live at `log_recall.rs:353-368`: `NoConsentProvider`, `NoGrant`, **`WrongDirection`**, `ConsentStateStale(String)`, `ConsentStateUnavailable(String)`. The enum is **not** `#[non_exhaustive]`, so a sixth variant forces every match site to be updated — fail-closed by construction, and cheap.

The first draft named three and said *"one generic denial is not three controls."* It is five. The missing one that matters is **`WrongDirection`** — the reverse-only grant, the **only refusal that encodes the asymmetry the entire tenant wall exists for** (ADR-058: *"A reverse-only grant is not symmetric and is reported separately"*).

⚠ **Two shipped Blocking legs already cover this ground and the first draft listed neither:**

| Leg (`check_multi_tenant_loom.rs` @`05e7e967`) | Runs |
|---|---|
| `cross-wall-recall-refusal-distinguishable` **`:816`** | `maos-iac` `adapter::log_recall::tests::cross_wall_recall_has_five_distinguishable_outcomes` |
| `cross-wall-recall-manifest-direction` **`:829`** | `cross_wall_recall_manifest_direction_and_staleness_are_typed` |

A developer implementing three limbs may **collapse or re-cut a five-way shipped control**. AC4 holds all five and re-cuts the first leg (D-8 forces it) **without relaxing a floor**.

### D-11 — AC1's fork was already closed by this room, 24 hours earlier

The first draft offered *"a filter field makes every existing call site able to ask a cross-wall question"* as a **benefit**. It is the hazard:

- `main.rs:4543-4544` — `.with_log_recall_port(Arc::clone(&log_recall_adapter) as Arc<dyn LogRecallPort>)`. **Story 8.14c injects this port into the researcher Spirit** (`--live`). An LLM builds those filters.
- 13.6b D-16, ratified: ***"a Spirit that can name a destination team is a Spirit that can be prompt-injected into naming one."***
- `spirits/researcher/src/lib.rs:1016-1040` — `recall_all` **rebuilds** the filter every page via `LogRecallFilter::new(…)` (six positional args) and calls `port.recall`. A team field is then either silently dropped — a filter that lies — or someone makes plain `recall` dispatch on it, which is **D-4's trap, sprung by AC1**.

⇒ **Not a fork. An omission wearing a fork's clothes.** AC1 is settled: **sibling entry point, no filter field.**

### D-12 — see D-2 (conditional consent injection; BOOTS ≠ SERVES)

### D-13 — the dead-wire leg is a source scanner, not a boolean

`cross_wall_recall_has_no_production_caller` (`crates/maos-bin/tests/cross_team_consent_13_3.rs:221-260`) walks `crates/` and `spirits/`, **splits each file at `#[cfg(test)]`** and searches the production half for the literal `.recall_cross_wall(`, skipping any directory named `tests` / `benches` / `examples` / `target`. Its leg runs `-p maos-bin --features network --test cross_team_consent_13_3` — **the feature flag and the test file were both unnamed in the first draft.**

⇒ Inverting it means **rewriting a scanner** and knowing where the positive caller is allowed to live (a non-test `.rs` under `crates/` or `spirits/`, above any `#[cfg(test)]`).

### D-14 — `ABSENT_SUCCESSORS` exists twice, with opposite states, and one copy is 13.6's

- `xtask/src/check_reza_production_path.rs:17` — populated.
- `xtask/src/check_multi_tenant_loom.rs:31` — **`&[]`**, empty. 13.6's own D-2 already files that emptiness as a defect (*"13.1 AC5's 'journey legs emit ABSENT, never disappear' has DISAPPEARED on that gate"*).

⇒ AC6 names **which one**. Writing the wrong one edits the closer's evidence ledger out from under it.

---

## Acceptance Criteria (6)

### AC1 — The request can express the question, through a sibling entry point

**Given** D-1 (no production request can name a remote team) and **D-11** (a filter field puts a team name inside an LLM-backed Spirit's reach, and `recall_all` would either drop it or spring D-4),
**When** this story lands,
**Then** the team dimension is carried by a **sibling entry point in `maos-domain`** — **not** a field on `LogRecallFilter`, and **not** kernel-core (D-3),
**And** it is constructed through a **validating** constructor, never a bare public field a struct literal can set,
**And** ⚠ `LogRecallFilter`'s six-field shape and `LogRecallFilter::new`'s arity are **unchanged**, so `recall_all` (`spirits/researcher/src/lib.rs:1016`) cannot acquire a team dimension by accident — with a **static leg** proving no team-typed field reached `LogRecallFilter`,
**And** the reason is recorded in the design note as a **security property**, citing 13.6b D-16 — not as a preference.

### AC2 — The cross-wall read actually reads across the wall

**Given** **D-8**: `recall_cross_wall` ends in `self.recall(spirit_pid, filter)`, so `team` selects a grant and never a source, and a green Blocking leg specifies that,
**When** a consented cross-wall recall is issued,
**Then** the rows come from the **named team's** Transparency Log artifact, via a **new dependency-inverted port in `maos-domain`** implemented by an adapter in **`maos-bin`** (D-9: `maos-iac` cannot reach `maos-audit`; no new `maos-iac` edge),
**And** the adapter derives the artifact through the **shipped** surfaces — `maos_audit::transparency_log_path_for_tenant_mode(…, Some(remote_team))` + `validate_transparency_log_path` + the in-artifact `tenant_binding` identity check — opened **read-only**,
**And** ⚠ **the control that does not exist today**: a negative proving the returned page is **NOT the caller's own rows** — two teams, two artifacts, disjoint frame sets, and an assertion that a cross-wall page contains the remote team's frames and **none** of the local team's,
**And** the ordinary boot path **still refuses** a foreign-team artifact (13.5e `TenantBindingError`, `maos-audit/src/lib.rs:1251`); the consented entry point is the **sole** permitted foreign open, proven by a negative on any other path,
**And** when the read port is **absent**, `recall_cross_wall` fails closed with a **typed** refusal — never a silent fall-back to `self.recall` (a sixth `CrossWallRecallRefusal` variant is authorized in `maos-domain`; the enum is not `#[non_exhaustive]`, so the compiler enforces every match site),
**And** the shipped leg `cross-wall-recall-refusal-distinguishable` is **re-cut, never relaxed**: its granted-path assertion moves from *"the target pid's frames"* to the remote team's frames, the change is named in the commit, and the five refusal outcomes it also covers stay intact. ⚠ **Do not re-can a fixture or lower a floor** (13.6c·AC3).

### AC3 — A production read-side initiator exists, consent is proven on the live path, and it lands LAST

**Given** `CrossWallRecallConsentAdapter` is constructed at `main.rs:2974` and feeds nothing (D-2),
**When** a production surface issues a traceback against a named remote team,
**Then** `recall_cross_wall` is reached **in production** with a real `&TeamId`, consent is proven fresh through the already-wired adapter, and an allowed recall returns the remote page,
**And** ⚠ **commit order is part of the design** (13.6a standing lesson): the initiator lands in the **last** commit, **after** AC2's read. No commit may contain a production traceback that returns local rows,
**And** the two **boot preconditions** are named in the test (D-2/D-12): verified cohort state **and** `MAOS_LOOM_HOME_TEAM`. A run without them yields `NoConsentProvider` and proves nothing — **BOOTS ≠ SERVES**,
**And** `cross-wall-recall-no-production-caller` (`check_multi_tenant_loom.rs:872` @ the `05e7e967` branch point) is **inverted in the same commit**, with this story named as its owner in the replacement — never left half-deleted (11.3 D2 / 10.4c atomic cutover). ⚠ It is a **source scanner** with `--features network` (D-13), not a boolean,
**And** the **13.5g composition-root test applies**: delete the new production initiator and confirm the positive leg **reds**. If it stays green it is testing the library, not the wiring.

### AC4 — Refusal is first-class across all FIVE outcomes, and the emitter-scope pin does not widen

**Given** ADR-049 §7, the citer-auth pin (D-4) and **D-10** (five shipped variants, two shipped legs),
**When** a cross-wall recall is refused,
**Then** it surfaces `ECrossWallRecallDenied` with its typed `CrossWallRecallRefusal` as an **observable operator outcome**, never folded into an empty-page success, and a legitimately empty page stays `Ok`,
**And** **all five** stay distinguishable from one another — `NoConsentProvider`, `NoGrant`, **`WrongDirection`**, `ConsentStateStale`, `ConsentStateUnavailable` — plus AC2's read-port-absent variant. One generic denial is not six controls,
**And** ⚠ `WrongDirection` is named explicitly: it is the only refusal that encodes the **asymmetry** the tenant wall exists for, and the existing `cross-wall-recall-manifest-direction` leg (`:482`) stays green,
**And** ⚠ **a mutation that lets plain `recall` cross the wall must red** (D-4). `ranged_recall` stays compile-pinned out (13.3b·AC3),
**And** each limb is **proven-red separately**: a mutation neutering one refusal reds its own leg and leaves the siblings green.

### AC5 — A disclosure that crossed the wall says so, and the orphaned journaling leg gets a live owner

**Given** **D-5** (the leg forbids `insert_frame_event` in the method span and names DONE 13.5e as its replacement owner) and **D-6** (today's `log.recall` row is *accurate*, because the read is local),
**When** an allowed cross-wall recall returns a remote page,
**Then** the audit record names it a **cross-wall disclosure** — the remote team, the consent grant relied on, and the fact that data left the team boundary — emitted **before data movement** (FR4, the `recall`/`fetch` idiom at `log_recall.rs:275`/`:385`),
**And** it stays **ZERO kernel-core Δ** by reusing `FrameKind::CapabilityInvocation` with a **new intent constant** in `maos-iac` (sibling to `LOG_RECALL_INTENT`/`LOG_FETCH_INTENT`). ⚠ **Do not add a `kernel-core` `Intent` / `Scope` / `FrameKind` variant** (`cap_policy/mod.rs:364`, `ports/policy_decision.rs:48`) — the capability check keeps the existing `log:recall` intent; only the **audit** intent is new,
**And** `cross-wall-recall-refusals-not-journaled` is **inverted and its replacement lands in the same commit** (atomic cutover), the replacement asserting positively that **both** a refusal and a disclosure journal, with this story named as owner,
**And** ADR-058's limits section is amended in **lockstep** — Decision 2 closed for success-side disclosure, and the *"must not be claimed by a gate or operational document until implemented"* sentence updated to match what shipped,
**And** ⚠ if implementation shows this cannot be done without a kernel-core edit, **stop and record it** — do not smuggle it in, do not silently drop it. A measured *"this costs a kernel Δ"* is an acceptable outcome; a quiet omission is not.

### AC6 — Gate, ADR and budget

**Given** ADR-055 owns the tenant wall and `check-multi-tenant-loom` is its gate,
**When** this story's legs are registered,
**Then** they go on **`check-multi-tenant-loom`** — **no new gate** (13.5c K5(b), re-ratified at 13.5e),
**And** every hermetic leg is `Blocking` with **one `#[test]` per `--exact` leg** (the gate's only anti-vacuity oracle is `"running 1 test"` + `"1 passed"`, structurally blind to a null assertion),
**And** live legs are `AdvisorySubstrate` and **`.expect()` their own env var rather than silently skipping** (13.5g — skipped ≠ passed), registered in `env_contract.rs`,
**And** `check-kernel-baseline` proves **23401 unchanged**; if it disagrees, **stop for FLAG-Winston, do not route around the owner**,
**And** ⚠ **`ABSENT_SUCCESSORS` is updated in `check_reza_production_path.rs:17` ONLY.** `check_multi_tenant_loom.rs:31` is `&[]` and its emptiness is **13.6's** filed defect (D-14) — do not touch it,
**And** the budget below is **re-measured in an isolated `git worktree` at the real branch point** before writing, and any breach is raised as **FLAG-Winston at design time**, never re-based at "done" — that has happened four consecutive stories and is a live retro item.

---

## Traps

1. **D-8 first.** The `team` parameter is decorative. Build the read before the initiator; no commit ships a traceback that returns local rows.
2. **AC1's fork is closed** (D-11). Sibling entry point. A filter field puts a team name in an LLM's reach and springs D-4 via `recall_all`.
3. **The journaling leg forbids journaling** (D-5). It is not a control requiring it, and its owner (13.5e) is DONE. Invert it; don't "keep it green" by moving the write somewhere weaker.
4. **Five refusals, not three** (D-10), and two shipped legs cover them. Re-cut, never relax.
5. **`maos-iac` cannot see `maos-audit`** (D-9). Port in `maos-domain`, adapter in `maos-bin`. No new `maos-iac` edge.
6. **The consent adapter is conditionally injected** (D-2/D-12). Without cohort state + `MAOS_LOOM_HOME_TEAM` every call returns `NoConsentProvider` — a denial that proves nothing.
7. **Do not widen plain `recall`** (D-4). The citer-auth pin is 13.3b·AC2 ratified and must red on mutation.
8. **`ranged_recall` is forbidden** — path-addressed, capability-free, compile-pinned out.
9. **Not kernel-core** (D-3) — `maos_kernel_core::iac` is a re-export shim. Verify with the gate anyway. And AC5 must not add a kernel `Intent`/`Scope`/`FrameKind` variant.
10. **Do not "fix" `main.rs:2402` / `SpiritMemoryView`.** It forwards only to the trait path, a permanent `CapabilityDenied` in production **by design** (`memory/mod.rs:721-739`). 13.5d recorded this trap; 13.6b re-recorded it.
11. **The dead-wire leg is a source scanner** with `--features network` (D-13). Inverting it rewrites a scanner.
12. **One `#[test]` per `--exact` leg.** Two tests behind one leg name defeats the oracle.
13. **Proven-red per limb, byte-identical restore** (`diff -q`). Serialize the mutations; do not batch.
14. **Measure the baseline in an isolated `git worktree`**, not a dirty tree (the 13.5i error, fixed at 13.5j). *"Code first, then the pin."*
15. **`cargo run -q -p xtask -- <cmd>`.** There is no `cargo xtask` alias.
16. **Gate-file AND budget serialization.** 13.6b rewrites `check_multi_tenant_loom.rs` and consumes `maos-bin`/`maos-audit` reserve. Land after it, on a re-measured budget.

---

## Tasks

- [x] **T1 (AC1/AC2)** — Probe-harness pass (13.5j): drive `recall_cross_wall` through the public surface until every probe is red, **including a probe that distinguishes remote rows from local rows** — the probe D-8 would have failed. Delete before commit.
- [x] **T2 (AC1)** — Sibling entry point in `maos-domain` behind a validating constructor; `LogRecallFilter` and `LogRecallFilter::new` **byte-unchanged in shape/arity**; static leg proving no team field landed on the filter.
- [x] **T3 (AC2)** — New `CrossWallLogReadPort` (name at dev's discretion) in `maos-domain`; `LogRecallAdapter` gains a `with_cross_wall_read` builder mirroring `with_cross_wall_consent`; `recall_cross_wall`'s tail becomes the port call, **fail-closed with a typed refusal when absent** — never `self.recall`.
- [x] **T4 (AC2)** — `maos-bin` adapter: `transparency_log_path_for_tenant_mode(…, Some(remote_team))` → `validate_transparency_log_path` → `tenant_binding` identity → read-only open → query. Add a `maos-audit` read-only foreign-open helper if the existing surfaces don't compose.
- [x] **T5 (AC2)** — The two negatives: (a) a cross-wall page contains the remote team's frames and **none** of the local team's; (b) every non-consented path still refuses a foreign artifact. Re-cut `cross_wall_recall_has_five_distinguishable_outcomes`' granted assertion and name the change in the commit.
- [x] **T6 (AC4)** — Refusal legs for **all five** shipped variants plus the read-port-absent variant, each observable and each distinguishable. Confirm plain `recall`'s emitter pin is untouched and reds on mutation; `cross-wall-recall-manifest-direction` stays green.
- [x] **T7 (AC5)** — Cross-wall disclosure journaling at the cross-wall site, before data movement, reusing `CapabilityInvocation` + a new intent const. **Invert `cross-wall-recall-refusals-not-journaled` and land its positive replacement in the same commit.** If a kernel Δ is required, **stop and record**.
- [x] **T8 (AC3)** — **LAST commit.** Production surface supplying the `&TeamId`; confirm `CrossWallRecallConsentAdapter` (`main.rs:2974`) is on the live path with both boot preconditions named. Invert `cross-wall-recall-no-production-caller` (source scanner, `--features network`) and add the positive leg naming the caller and the consent proof; one `#[test]` per `--exact` leg. Run the composition-root deletion test.
- [x] **T9 (AC4/AC6)** — Serialized proven-red pass: one mutation per limb, own-leg-only red, restore byte-identical. Record each mutation and its observed red leg in the Dev Agent Record.
- [x] **T10 (AC5/AC6)** — Amend ADR-058 (Decision 2 + limits section, in lockstep with what landed); update `ABSENT_SUCCESSORS` in **`check_reza_production_path.rs` only**; update `tests/coverage-matrix.yaml` if traceability changes.
- [x] **T11 (AC6)** — Gates: `check-kernel-baseline` (**23401**), `kloc-check` (**re-measure first — see Budget**), `check-multi-tenant-loom`, `check-reza-production-path`, `check-service-boundary`, `check-empty-kernel --path crates/maos-iac`, `cargo fmt --all -- --check`, `cargo test --workspace`.
- [x] **T12** — Record the dev model in the Dev Agent Record (`check-dev-model-tier` / `check-dev-model-used-populated` are live gates; frontier-class allowlist; §A6 full-layer review net **non-degradable** per `epic-13:13`).

---

## Dev Notes

### Existing surfaces to reuse — do not reinvent

| Need | Use | Path |
|---|---|---|
| Cross-wall read (local-delegating today — **D-8**) | `LogRecallPort::recall_cross_wall` | trait `maos-domain/src/ports/log_recall.rs:35`; impl `maos-iac/src/adapter/log_recall.rs:347`; **the defect is `:376`** |
| Local emitter-scoped read + the FR4 audit idiom to copy | `LogRecallAdapter::recall` / `fetch` | `maos-iac/src/adapter/log_recall.rs:270-287` / `:380-396` |
| **Remote artifact path derivation** | `transparency_log_path_for_tenant_mode` + `validate_transparency_log_path` | `maos-audit`; called `maos-bin/src/main.rs:88-98` |
| **Remote artifact identity** | `tenant_binding` row + `TenantBindingError` | `maos-audit/src/lib.rs:1009-1200`, refusal at `:1251` |
| Request shape (do **not** extend) | `LogRecallFilter` + `::new` | `maos-domain/src/log_recall.rs:14`, `:99` |
| Cross-wall read consent (already constructed, **conditionally**) | `CrossWallRecallConsentAdapter` | `maos-bin/src/cross_team_consent.rs:50`; wired `main.rs:2974` under the `:2968` guard |
| Typed refusal — **five** variants | `CrossWallRecallRefusal` (not `#[non_exhaustive]`) | `maos-domain/src/log_recall.rs:247-261` |
| Dependency-inversion precedent | `CrossTeamCrossingPort` (13.6b), 12.3 P5r | `maos-bin/src/cross_team_crossing.rs` |
| Dead-wire leg to invert (**source scanner**, `--features network`) | `cross-wall-recall-no-production-caller` | leg `check_multi_tenant_loom.rs:525`@`a414f922` / `:872` in tree; test `maos-bin/tests/cross_team_consent_13_3.rs:221` |
| Orphaned leg to **invert**, owner DONE | `cross-wall-recall-refusals-not-journaled` | leg `:541`@`a414f922`; test `cross_team_consent_13_3.rs:262` |
| Shipped leg to **re-cut, not relax** | `cross-wall-recall-refusal-distinguishable` | leg `:469`@`a414f922`; test `log_recall.rs:733` |
| Shipped leg that must stay green | `cross-wall-recall-manifest-direction` | leg `:482`@`a414f922` |
| Emitter-scope pin that must not widen | citer-auth control | `distillate.rs:330-359` (13.3b·AC2 ← 8.10 AC2b) |
| The Spirit that holds the port (**D-11**) | `with_log_recall_port` (8.14c) | `maos-bin/src/main.rs:4543`; walker `spirits/researcher/src/lib.rs:1016` |

### House standards

- Test idiom: `Command::new(env!("CARGO_BIN_EXE_maos"))`. No `assert_cmd`/`escargot`/`predicates`.
- `cargo fmt --check` blocking since E12-B4; rustfmt `max_width = 100`.
- `cargo-deny` is live — a new dependency needs justification. Prefer none.
- `#[ignore]` + `.expect()` on live legs; the gate controls execution. **Skipped ≠ passed.**

### Budget — ⚠ RAISED AT DESIGN TIME, and the numbers moved

**kernel-core:** ZERO @ **23401**. If the measurement disagrees, stop for FLAG-Winston. *"kernel-core ZERO" is not the same sentence as "zero delta"* — state both.
**fkcs:** `xtask/fkcs-baseline.toml` stays byte-untouched at `23081`.

**kloc — re-measured 2026-07-30 at the committed branch point `05e7e967` (13.6b landed; these are final, not in-flight):**

| Crate | Measured | Ceiling | Reserve | Note |
|---|---|---|---|---|
| `maos-domain` | 8421 | 8590 | **169** | port trait + sibling entry point + 6th refusal variant — fits |
| `maos-iac` | 6476 | 6606 | **130** | builder + tail swap + audit intent const — fits |
| `maos-bin` | **14807** | 14835 | **⚠ 28** | **the read adapter lands here.** 13.6b's grant, landed yesterday, already consumed. **28 lines is not enough — expect a named grant, FLAG-Winston, raised NOW.** |
| `maos-audit` | **6630** | 6665 | **⚠ 35** | if a foreign-open helper is needed it lands here |

⚠⚠ **`kloc-check` is RED AT THE BRANCH POINT and it is in this story's own T11 gate list**: `maos-a2a-core 4554 / 4450` OVER by **104** and `xtask 31879 / 31856` OVER by **23** — **both are 13.6b's, committed at `05e7e967`.** 13.6b shipped `kloc-check` red. The 13.6d developer will run T11 and hit a red gate they did not cause. **Resolve or carve out BEFORE starting** — see Residual 6. `maos-a2a-core`'s ceiling is already an unratified FLAG-Winston grant (third consecutive), so this is an owner decision, not a re-base. ~17 crates still carry ceiling `0` (Residual 4).

Ceiling policy is `measured + max(100, ceil(0.02 × measured))`; slack is operating capacity, **not** authorization; a ceiling must never block a correctness repair — **surface it rather than routing around it.**

### Previous-story intelligence

- **13.6a (`a414f922`).** Its **honest finding** — an Accept-seam rule that is defense-in-depth behind another control — is the model for reporting a layered control without over-claiming it. Its standing lesson (D-7) is why D-5, D-6 and D-8 exist.
- **13.6b.** D-16: a stale seam comment steered three stories' design. **D-5 is the same shape in a test failure message.** D-8's dependency inversion is 13.6b's D-8 pattern reused. Its budget grants are why `maos-bin` has 28 lines.
- **13.5e.** The read substrate (D-9) — and the control that refuses a foreign artifact, which must survive this story.
- **13.5g.** Legs green while connecting to nothing; no leg exercised the composition root. AC3's deletion test is the direct descendant.
- **13.5j.** *Probe-harness-before-design* — T1 is that method, extended by D-8 to probe *where the rows came from*.
- **13.6c.** *Never re-can a fixture or relax a floor* — AC2's re-cut of a shipped Blocking assertion is the one place this story comes near that line, so it is named in the commit.

### References

- [Source: `epic-13-reza-cortex-v2-2.md#52`] — *"13.6 is last and only judges."*
- [Source: `epic-13-reza-cortex-v2-2.md#175`] — separately-named clauses, each with its own inverter.
- [Source: `docs/adr/ADR-058-cross-wall-provenance-and-consented-recall.md`] — Decision 2 + **the limits section** (conditional injection; *"must not be claimed by a gate or operational document until implemented"*).
- [Source: ADR-049 §7] — provenance-presence discipline: a refusal is an outcome, not an empty page.
- [Source: `docs/adr/ADR-055-multi-tenant-loom.md:82`] — *"This does not establish per-team Transparency Log isolation or refusal journaling."* AC2 and AC5 change that; amend it.
- [Source: `_bmad-output/implementation-artifacts/deferred-work.md`] — 2026-07-21 success-side disclosure entry.

---

## Residuals (carry forward; do not silently close)

1. **`(f-ii)` was closed by 13.5c; `(f-i)` is 13.6b's; the read-side leg is this story's.** After 13.6b and this story, no Epic-13 dead-wire clause should remain unassigned — **verify that mechanically before asserting it.** ⚠ **D-5 is a live counterexample**: an orphaned leg whose named owner (13.5e) is DONE. The verification must look for *stale* owners, not just missing ones.
2. **The kernel erases every collective cause** — `kernel-core/src/memory/mod.rs:204`, `CollectivePortError::Transport(_)`. Not this story's path, but the same class of erasure AC4 guards against on the recall side. **Owner: 13.6.**
3. **`v25-signed-shard`** — unchanged, not this story's.
4. **~17 crates carry ceiling `0` in `xtask/kloc.toml`** (incl. `maos-cohort` 4800, `maos-registry` 3515, `maos-eval` 3596) — so their growth is unmeasured. Retro item, recorded at 13.6b, **re-measured and widened here**.
5. **`check_multi_tenant_loom.rs:31` `ABSENT_SUCCESSORS = &[]`** (D-14) — deliberately untouched by AC6. **Owner: 13.6.**
6. ⚠ **`kloc-check` RED at the branch point `05e7e967` — CONFIRMED, not suspected.** `maos-a2a-core 4554/4450` (+104), `xtask 31879/31856` (+23). **Owner: 13.6b**, which shipped it red. `kloc-check` is in this story's T11 list, so it must be green — or explicitly carved out with a named owner — **before 13.6d starts**, otherwise this story's budget claims and its "gates green" report are both meaningless. `maos-a2a-core`'s ceiling is a third-consecutive unratified FLAG-Winston grant ⇒ **owner decision, not a re-base.**
7. **`check-service-boundary` and `check-dev-model-tier` RED at the branch point — pre-existing, NOT this story's.** `check-service-boundary`: removed public kernel symbol `maos_kernel_core::memory::REGISTERED_ERASURE_BACKENDS` (NFR-Test-2 monotonic-additive violation) + `SecurityManagerAdapter`/`CapabilityRegistryAdapter` each constructed N=2 in `main.rs` (P1 single-owner). `check-dev-model-tier`: **6** frontier-era violations — `13-5g`, `13-5i`, `13-5j` (missing dev model; 13-5j also missing the §A6 marker) and `13-6` (not yet dev'd). 13.6a recorded these as byte-identically red at its own baseline. **Do not "fix" them inside 13.6d** — but do not report "gates green" while they are red either. `13-6b` is **not** flagged, so its §A6 net is closed per the binding control.
8. **Stale sentence in `sprint-status.yaml`:** 13.6b's entry still carries *"the SS-A6 full-layer review net has NOT run … NOT eligible for done yet"* from its pre-review pass, while its status is `done`, its story file carries the review markers, and `check-dev-model-tier` does not flag it. Harmless today, exactly the kind of stale claim this epic keeps getting bitten by. Correct it at the retro.

---

## Review Findings

Code review 2026-07-30 — 3-layer adversarial (Blind Hunter · Edge Case Hunter · Acceptance Auditor); dev_model `openai-codex/gpt-5.6-sol` (Codex family) → Test-Infra layer not armed. **7 patch applied (D1, P1–P6; P2 closed in-story via a single-connection foreign read after the team rejected the defer), 1 defer (W1 `consent_grant`), 8 dismissed.** All applied patches verified: `cargo test -p maos-iac` (lib 97 + cross-wall 10/10), `cargo test -p maos-bin --features network --test cross_team_consent_13_3` (8/8), `--test cross_wall_log_read_13_6d` (6/6), `--test cohort_daemon_smoke_13_5c` (9/9), `--test tenant_audit_phase_a_13_5g` (6/6), `cargo fmt --all --check` clean; gates `check-kernel-baseline` (23401), `kloc-check` (PASSED), `check-multi-tenant-loom` (PASSED) green; `check-empty-kernel --path crates/maos-iac` and `check-service-boundary` carry only their pre-existing reds (no 13.6d finding).

### Patch (applied)

- [x] [Review][Patch] **D1 (option A): make the cross-wall disclosure journal truthful** — `recall_cross_wall` now journals a pre-movement `disclosing` intent (FR4), then the truthful outcome after the read: `disclosed` on success, `failed` on a read error (never a false `disclosed`). Operational read failures are no longer rendered as consent refusals (`main.rs` CLI now emits `outcome:"error"` for non-denial errors). Verified by new `cross_wall_recall_journals_failure_when_remote_read_fails`. [`crates/maos-iac/src/adapter/log_recall.rs:416-432`, `crates/maos-bin/src/main.rs:3070-3088`]
- [x] [Review][Patch] P1 — the read adapter is now attached only when `MAOS_LOOM_POSTGRES` is present, so a cross-wall read fails closed (`ReadPortUnavailable`) instead of resolving the global artifact under a global-mode boot; verified by the composition-root Command test. [`crates/maos-bin/src/main.rs:3046-3050`]
- [x] [Review][Patch] P3 — production-caller gate now asserts the four preconditions appear in source order inside the one conditional branch AND that the composition-root dispatch site (`if let Some(request) = cross_wall_traceback`) calls `run_cross_wall_traceback`, so a dead-wired dispatch reds the leg. [`crates/maos-bin/tests/cross_team_consent_13_3.rs:368-401`]
- [x] [Review][Patch] P4 — added `cross_wall_traceback_refuses_without_cohort_preconditions`, a composition-root test that runs the `maos` binary's `traceback` dispatch and asserts a typed refusal (BOOTS ≠ SERVES); deleting the dispatch reds it. [`crates/maos-bin/tests/cross_team_consent_13_3.rs:482-513`]
- [x] [Review][Patch] P5 — `cross_wall_recall_refusals_and_disclosures_are_journaled` is now behavioral: drives a real refusal and disclosure through `LogRecallAdapter` and asserts the audit rows (`refused`; `disclosing`→`disclosed`), no longer a source-scan. [`crates/maos-bin/tests/cross_team_consent_13_3.rs:403-480`]
- [x] [Review][Patch] P6 — ADR-055 amended: the stale 13.3b "does not establish… isolation or refusal journaling" sentence now points to the 13.5e/13.5g + 13.6d closure, and a new `### 4d` section records the cross-wall read, truthful disclosure journaling, and the carried TOCTOU identity limit. [`docs/adr/ADR-055-multi-tenant-loom.md:82,139-145`]
- [x] [Review][Patch] **P2 (closed in-story; pulled back from defer)** — the foreign-read TOCTOU is closed: `read_remote` verifies the binding AND serves the rows on **one** read-only `NOFOLLOW` connection (`maos_audit::open_tenant_artifact_readonly` + `read_tenant_artifact_on`), and `maos-iac` gains `TransparencyLogAdapter::from_read_only_connection` to query that same handle — redaction is the stateless `CorpusBackedRedactionPolicy`, so a foreign shard is redacted identically. The defer's "needs a `maos-iac` zero-delta change" was a misread: the pin is zero **kernel-core** (23401), not zero `maos-iac`; `maos-iac` has budget and the only hard law is the no-`maos-audit`-edge (none added — the value crossing is a `rusqlite::Connection`). Verified by new `single_connection_foreign_read_is_immune_to_an_artifact_swap_between_binding_and_query` (a held connection serves the verified artifact's rows after a path swap). ADR-055 §4d amended to record the consistency closure; cryptographic artifact identity remains `v25-signed-shard`. [`crates/maos-bin/src/cross_wall_log_read.rs:39-68`, `crates/maos-audit/src/lib.rs:1069-1139`, `crates/maos-iac/src/adapter/transparency_log.rs:392-423`]

### Defer

- [x] [Review][Defer] `consent_grant` audit field records the compile-time intent constant, not an actual grant id/version/lease — `CrossWallRecallConsentDecision::Granted` is a unit variant carrying no metadata (`crates/maos-domain/src/ports/cross_wall_recall_consent.rs:17`), so enriching it widens the consent port (owned by 13.6a). The grant is reconstructable from (home_team, remote_team, intent). [`crates/maos-iac/src/adapter/log_recall.rs:107`] — deferred, consent-port enrichment owned by 13.6a

## Status

done

## Dev Agent Record

### Agent Model Used

Model: openai-codex/gpt-5.6-sol

### Debug Log References
- 2026-07-30 pre-write budget gate at branch point `05e7e967`: `cargo run -q -p xtask -- kloc-check` RED exactly as preflight warned. Pre-existing 13.6b overages remain `maos-a2a-core` 4554/4450 (+104) and `xtask` 31879/31856 (+23); zero-ceiling crates also keep the gate red. `maos-bin` is 14807/14835 (28 lines reserve), insufficient for the required read adapter. Implementation halted before code per AC6/Residuals 4 and 6 pending FLAG-Winston budget disposition.
- 2026-07-30 T1 probe pass: the remote-vs-local public-surface probe failed with the returned frame-id set equal to the local artifact and unequal to the named remote artifact; the production-caller probe failed with zero callers. Both temporary probes were deleted, and the two shipped baseline tests passed after byte-equivalent restoration.
- 2026-07-30 FLAG-Winston disposition: Lunarpulse authorized the founder-policy rebaseline. `xtask/kloc.toml` now governs every previously zero-ceiling source root, assigns the committed `maos-a2a-core`/`xtask` overages to 13.6b, grants `maos-bin` 13.6d design headroom, and sets the aggregate ceiling from 137158 measured LOC. `kloc-check` then PASSED at 137158.
- 2026-07-30 T6 emitter-scope mutation: changed `FrameFilter.spirit_pid` from `Some(spirit_pid)` to `None`; `cross-wall-recall-local-emitter-scope` red with 13 rows returned instead of 5. Restored SHA-256 `4f5b9c1b9dc44663d6da56c4fb54c23a64b1a35a837e23a5779b2cc49e66d30a` byte-identically; exact leg returned green.
- 2026-07-30 T7 kernel check: success/refusal journaling uses `FrameKind::CapabilityInvocation` and `LOG_CROSS_WALL_RECALL_INTENT`; `check-kernel-baseline` PASSED 23401==23401. No kernel `Intent`, `Scope`, or `FrameKind` change was required.
- 2026-07-30 T8 composition-root deletion mutation: replaced `run_cross_wall_traceback`'s port call with a typed local error; `cross-wall-recall-production-caller-live` red with zero production callers. Restored SHA-256 `5cc2431ebc0f92b77976114082eabf2704a196ad7067aa11b755ffdd39525ee5` byte-identically; the exact leg returned green.
- 2026-07-30 T8 smoke: `maos traceback --team team-b --spirit-pid 42 --limit 2` without verified cohort state and `MAOS_LOOM_HOME_TEAM` emitted observable JSON `outcome=refused`, typed `NoConsentProvider`, and exit 1—confirming BOOTS ≠ SERVES.
- 2026-07-30 T9 serialized refusal mutations, each with one sibling held green: `NoConsentProvider→NoGrant` red `cross-wall-recall-no-consent-provider`; `NoGrant→WrongDirection` red `cross-wall-recall-no-grant`; `WrongDirection→NoGrant` red `cross-wall-recall-wrong-direction`; `Stale→Unavailable` red `cross-wall-recall-stale-state`; `Unavailable→Stale` red `cross-wall-recall-unavailable-state`; `ReadPortUnavailable→NoGrant` red `cross-wall-recall-read-port-unavailable`. Every edit was restored before the next; final SHA-256 `f61e8a6ea3f1e321c8d9584aa5af8989a013fba1349bb8e8bffcba34ea1a83f1` matched the pre-mutation file and all nine cross-wall tests returned green.
- 2026-07-30 T10 traceability: `coverage-matrix` completed successfully after ADR-055 notes recorded the 13.6d read-side closure and narrowed absent successors to the 13.6 journey plus kernel cause erasure.
- 2026-07-30 T11 gates: PASS — kernel baseline 23401, KLOC 137948 within the authorized policy ceilings, multi-tenant Loom, Reza production path, coverage matrix, Rust formatting, and every Story 13.6d exact/hermetic leg. Reza initially found the new `maos-bin/src/cross_wall_log_read.rs` absent from the closed source-file scanner; the scanner was corrected to cover it, and the complete Reza gate then passed.
- 2026-07-30 T11 inherited-red accounting, reported without laundering: `check-service-boundary` reproduced exactly the three preflight findings (`REGISTERED_ERASURE_BACKENDS`, duplicate `SecurityManagerAdapter`, duplicate `CapabilityRegistryAdapter`); `check-empty-kernel --path crates/maos-iac` reported only the four pre-existing I9 findings in `DistillateWriter`, `LogRecallAdapter.transparency_log`, and `TransparencyLogAdapter.inner`, with no 13.6d field added. Three full `cargo test --workspace` attempts advanced through the Story 13.6d suites but hit unrelated environmental/pre-existing flakes (`maos-mcp` stdio subprocess broken pipe; one J0 PTY five-second timeout; missing sandbox fixture). The sandbox fixture was built and its exact test passed; MCP and J0 exact suites passed in isolation. No Story 13.6d regression remained.
- 2026-07-30 T12 provenance: recorded `openai-codex/gpt-5.6-sol`; both model gates accepted Story 13.6d's allowlisted `gpt-5.6` family, populated record, and §A6 marker. Repository-wide commands remain red only on the previously catalogued undeveloped 13.5i/13.5j/13.5g/13.6/13.6c story records; no 13.6d violation was emitted.

### Completion Notes List
- T1 complete: two deliberately failing probes reproduced both founding defects before design—local rows returned under a remote label and no production caller. Probe scaffolding was removed; shipped baseline controls remained green.
- T2 complete: added private-field `CrossWallRecallRequest` with canonical `TeamId` validation and zero-copy `into_parts`; left `LogRecallFilter`'s six fields and constructor arity unchanged. New Blocking static leg proves only the operator sibling request carries `TeamId`.
- T3 complete: introduced `CrossWallLogReadPort`, injected it through `with_cross_wall_read`, replaced the decorative local `self.recall` tail with `read_remote`, and added typed `ReadPortUnavailable` fail-closed behavior. Both granted routing and absent-port tests pass.
- T4 complete: `CrossWallLogReadAdapter` derives the named team shard with the shipped audit path function, validates symlink ancestry, verifies the in-artifact `tenant_binding`, opens SQLite read-only/NOFOLLOW without migrations, and reuses the emitter-scoped query mapper. Three Blocking legs cover remote rows, binding mismatch, and read-only open shape.
- T5 complete: added the two-artifact disjoint-frame negative and the ordinary Phase-A foreign-artifact refusal. Re-cut the shipped granted-path control under the explicit change name **“cross-wall recall granted path asserts remote frames, never target-pid local frames”** without lowering its refusal floor.
- T6 complete: all five shipped consent refusals plus `ReadPortUnavailable` have distinct exact Blocking legs and operator-visible messages. `WrongDirection` remains explicit, manifest direction stays green, plain recall's emitter scope proved red on mutation, and Researcher remains compile-pinned away from `ranged_recall`.
- T7 complete: atomically replaced the orphaned “not journaled” leg with positive refusal and disclosure controls. Audit payloads name the remote team, outcome, consent grant, refusal, and boundary-crossing fact; an observing read port proves the disclosure row exists before remote data movement.
- T8 complete, landed after the read and audit path: added validated `maos traceback` production dispatch, conditionally injected both production consent and remote-read adapters only when verified cohort state plus `MAOS_LOOM_HOME_TEAM` exist, inverted the caller scanner, exercised fresh signed-manifest consent against a real remote shard, and proved deletion red.
- T9 complete: all six refusal branches were mutated independently, each own leg red while a sibling stayed green, and the source was restored byte-identically before the next mutation.
- T10 complete: ADR-058 Decision 2 and limits now match the validated request, dependency-inverted remote shard read, six refusals, conditional production injection, and success/refusal journaling. Only `check_reza_production_path.rs`'s populated successor ledger was updated; `check_multi_tenant_loom.rs`'s empty ledger remained untouched.
- T11 complete with honest baseline accounting: all story-owned gates and exact tests pass; KLOC and kernel checks are green. The two known structural gates remain red exactly on inherited findings, and full-workspace failures were isolated to unrelated tests that pass independently—none are reported green.
- T12 complete: dev provenance records `openai-codex/gpt-5.6-sol`; Story 13.6d satisfies the frontier-family, populated-record, and non-degradable §A6 marker controls.

### File List
- `_bmad-output/implementation-artifacts/13-6d-cross-wall-recall-production-initiator.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `xtask/kloc.toml`
- `crates/maos-domain/src/log_recall.rs`
- `crates/maos-bin/tests/cross_team_consent_13_3.rs`
- `xtask/src/check_multi_tenant_loom.rs`
- `crates/maos-domain/src/ports/log_recall.rs`
- `crates/maos-domain/src/ports/mod.rs`
- `crates/maos-iac/src/adapter/log_recall.rs`
- `crates/maos-bin/src/cross_wall_log_read.rs`
- `crates/maos-bin/src/lib.rs`
- `crates/maos-iac/src/adapter/transparency_log.rs`
- `docs/adr/ADR-058-cross-wall-provenance-and-consented-recall.md`
- `xtask/src/check_reza_production_path.rs`
- `tests/coverage-matrix.yaml`
- `crates/maos-bin/tests/cross_wall_log_read_13_6d.rs`
- `crates/maos-bin/src/main.rs`
- `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs`

---

## Change Log

| Date | Change |
|---|---|
| 2026-07-29 | **Created — read side split out of 13.6b** (operator-ratified at the post-13.6a preflight; the fifth split of Epic 13). 5 ACs, ZERO kernel-core Δ @23401, no new gate. |
| 2026-07-29 | **Post-13.6b party-mode preflight — FOUNDING PREMISE INVERTED, scope absorbed, 5 ACs → 6.** **D-8 (headline):** `recall_cross_wall` ends in `self.recall(spirit_pid, filter)` (`log_recall.rs:376`) — the `team` argument reaches `consent.decide` and never a query, so the capability's parameter is **decorative**, and the shipped **Blocking** leg `cross-wall-recall-refusal-distinguishable` *specifies* the local delegation (*"a granted cross-wall recall returns the target pid's frames"*, `:735`). The first draft's AC2 (*"returns the remote page"*) was unbuildable by wiring **and** contradicted a green gate; wiring an initiator would have shipped a traceback that returns the operator's own rows under a remote label. New catalogue shape: ***a capability whose parameter is decorative***. **Operator ratified ABSORB over a sixth split**: one story, AC2 grows, Epic 13 stays at 20. **D-9:** the read substrate exists locally — `maos_audit::transparency_log_path_for_tenant_mode(…, Some(team))` + `validate_transparency_log_path` + in-artifact `tenant_binding` — and 13.5e already ships the control that refuses a foreign-team artifact, so the consented entry point must be the **sole** permitted foreign open; `maos-iac` has **no** `maos-audit` dependency, so port in `maos-domain` / adapter in `maos-bin` (12.3 P5r, third consecutive story). **D-5 CORRECTED:** `cross-wall-recall-refusals-not-journaled` asserts the **ABSENCE** of journaling (`!method.contains("insert_frame_event")`) and names **DONE** Story 13.5e as its replacement owner — an orphaned dead-wire leg the first draft listed as *"must stay live"*; 13.6b's D-16 shape, this time in a test failure message. Operator ratified **re-own + atomic cutover** (AC5), ZERO Δ via a new audit intent const on `CapabilityInvocation` — no kernel `Intent`/`Scope`/`FrameKind` variant. **D-6 CORRECTED:** today's `log.recall` row is *accurate* (the read is local), so AC5 becomes correct only once AC2 lands. **D-10:** five shipped refusal variants, not three — `WrongDirection` (the asymmetry the wall exists for) and `ConsentStateUnavailable` were unlisted, as were **two** shipped Blocking legs. **D-11:** AC1's fork was closed 24h earlier — `main.rs:4543` injects `LogRecallPort` into the researcher **Spirit** and `recall_all` rebuilds filters per page, so a filter field either lies or springs D-4; sibling entry point only (13.6b D-16: *"a Spirit that can name a destination team is a Spirit that can be prompt-injected into naming one"*). **D-12:** consent injection is **conditional** on cohort state + `MAOS_LOOM_HOME_TEAM` (`main.rs:2968`, documented in ADR-058's limits) — BOOTS ≠ SERVES, named in AC3's deletion test. **D-13:** the dead-wire leg is a **source scanner** with `--features network`; inverting it rewrites a scanner. **D-14:** `ABSENT_SUCCESSORS` exists twice with opposite states — AC6 touches `check_reza_production_path.rs` **only**; the empty one is 13.6's. **AC3 adds commit order as a design constraint** — initiator lands LAST. **BUDGET RAISED AT DESIGN TIME, and it moved:** `maos-bin` **14807/14835 = 28 lines** (13.6b's day-old grant already consumed) where the read adapter must land ⇒ expect a named grant + FLAG-Winston; `maos-audit` 6630/6665 = 35; `maos-domain` 169; `maos-iac` 130. **`kloc-check` is RED in the tree** (`xtask` +23, `maos-a2a-core` +104 — both 13.6b's, new Residual 6). 6 ACs, ZERO kernel-core Δ @23401, no new gate. |
| 2026-07-30 | **Implemented — production cross-wall traceback.** Added validated operator request, dependency-inverted remote read port, read-only bound-shard adapter, six typed refusals, refusal/disclosure journaling, `maos traceback` composition-root initiator, Blocking exact legs, mutation evidence, ADR/traceability updates, and authorized KLOC policy ceilings. |
