---
baseline_commit: a414f922
depends_on: 13-6a-authenticated-team-identity (DONE @a414f922), 13-6b-production-cross-team-crossing-initiators, 13-6c-three-team-three-region-substrate, 13-6d-cross-wall-recall-production-initiator
blocked_by: 13-6b-production-cross-team-crossing-initiators, 13-6c-three-team-three-region-substrate, 13-6d-cross-wall-recall-production-initiator
kernel_grant: NONE — ZERO maos-kernel-core Δ expected, pin stays 23401 (verify final baseline after 13.6b/13.6d)
inherited_residuals: (a) the kernel erases every collective cause — `maos-kernel-core/src/memory/mod.rs:204`, `CollectivePortError::Transport(_) => CollectiveErrorKind::Transport`, so no Spirit has ever distinguished `ConsentDenied` from `MapStale` on any path (13.6b D-15, Residual 6 — THIS STORY IS THE NAMED OWNER: judge whether "the operator can see why the wall refused" is a claim the epic may make on the Spirit path); (b) `CollectiveMemoryPort` has no `share` verb (13.6b Residual 7)
---

# Story 13.6 — The closer is a judge, and the courtroom has no instrument for recording a verdict

Status: **blocked** — on `13-6b`, `13-6c`, `13-6d` *(13.6a landed `a414f922`; the former `13-6a-production-cross-team-crossing-initiators` was renamed to `13-6b` at the 2026-07-28 split and the stale duplicate file was deleted 2026-07-29)*

**Kernel-Δ: ZERO expected.** Work lands in `xtask`, `.github/workflows`, `crates/maos-bench`, tests and docs. **No new gate** — legs go on `check-multi-tenant-loom` and `check-reza-production-path`.

> **Read this first — the grounding pass reframed this story twice.**
>
> **Reframe 1 (scope OUT).** 13.6 was filed to compose 13.1–13.5e including *"an allowed A→B asymmetric share"* (AC2) and *"reverse B→A share"* refusal (AC3). Measurement at `cb412348` found **the cross-team crossing has no production initiator** — the consent apparatus is constructed at four sites in the composition root and terminates in `apply_replication_bundle`, which a *Blocking* gate leg proves nothing in production calls. The epic forbids this story from inventing it (*"13.6 is last and only judges"*, `epic-13:52`). **Operator ratified 2026-07-28: split.** The mechanism is **Story 13.6a**. This story judges it.
>
> **Reframe 2 (scope IN — and this is the real story).** With the mechanism handed off, the question became: *what does a judge actually need that does not exist?* Three answers, all measured:
>
> 1. **The verdict vocabulary is prose.** The epic defines four evidence states — `PROVEN_BLOCKING`, `PROVEN_LIVE_SIGNED`, `ABSENT`, `INDETERMINATE` (`epic-13:180`) — and 13.6·AC5/AC6 are built on them. `grep` across all of `xtask/` and `crates/` returns **one hit, an unrelated doc comment** in `halt_receipt.rs:128`. No gate emits an evidence state. **The instrument does not exist.**
> 2. **`ABSENT` does not block.** 13.6·AC6 says *"Any ABSENT or INDETERMINATE required leg blocks the Reza/v2.2 product claim."* `check_reza_production_path.rs:1153` computes `"passed": blockers.is_empty()`, and a skipped `AdvisorySubstrate` leg is **never a blocker** — absent substrate prints `PASSED (live substrate advisory; 2 absent successors declared)` and exits 0. The rule 13.6 enforces is enforced by nothing. *(Already on file as a deferred cross-gate finding, 2026-07-25 — it becomes this story's the moment AC6 depends on it.)*
> 3. **The substrate does not exist.** 13.6 needs 3 teams × 3 regions. CI provisions **one Postgres service and two databases** (`discipline.yml:2687,2706` — `maos_team_a`, then `CREATE DATABASE maos_team_b`). There is no team C. The 3-region pilot's `MAOS_TEST_POSTGRES_{A,B,C}` (`cross_region_live.rs:1877-1881`) appears **nowhere in `.github/`** — those legs have never run in CI, and `check-cross-region-consensus`'s CI job has no Postgres service at all.
>
> **So the closer's real deliverable is the evidence ledger and the substrate it runs on** — not new journey mechanisms. That is what makes a judge trustworthy, and it is the honest reason this story is last.
>
> **What this story does NOT claim.** It does not close NFR-Ops-11 (team axis served, operator axis deferred with sub-axes (i) and (iii) **open and ownerless**, `epic-13:71`). It does not execute 14 institutions — see AC5.

---

## Story

**As** Reza, running a 400-person fintech as one governed Cortex on shared MAOS infrastructure,
**I want** the platform's claim that my three teams can collaborate under governance to rest on a recorded, machine-derived verdict over a real multi-team substrate,
**so that** "the Reza journey works" is something a gate asserted from evidence it produced — and, when a leg is absent, something the gate **refuses** to assert.

---

## Grounded state of the six mechanisms this story judges

Measured at `cb412348`. This table is the story's foundation; re-verify it after 13.6a lands.

| Mechanism | Production-reachable? | Evidence |
|---|---|---|
| Tenant boot / physical + crypto wall | **YES**, live-substrate only | `main.rs:2743-2784`; `tenant-mode-boots-live`, `collective-store-tenant-wall-live` (both `AdvisorySubstrate`) |
| Spirit→collective route (13.5d) | **YES** | `spirits/researcher/src/lib.rs:587,600` → cap-gated `collective_write`/`collective_read` |
| Collective erase / legal hold (13.5b) | **YES** | `main.rs:4947` → `CollectiveMemoryPort::erase`; 13.5b legs on `check-reza-production-path` |
| Vetting promotion (13.4) | **YES** | `check-vetting-attestation`, 7 anti-null legs |
| Enterprise Spirit at the daemon seam (13.5a) | **YES** | `enterprise-governance-reaches-cohort-daemon` + dead-wire negative |
| **Cross-team crossing (write + read)** | **NO → 13.6a** | `is_granted` has one non-test consumer, `bundle.rs:917`, inside a function with zero production callers |

---

## The three defects this story closes — measured, not asserted

### D-1 — the four-state evidence vocabulary exists only in prose

```
$ grep -rn "PROVEN_BLOCKING\|PROVEN_LIVE_SIGNED\|INDETERMINATE" --include='*.rs' --include='*.toml' xtask/ crates/
crates/maos-cohort/src/halt_receipt.rs:128:/// - **INDETERMINATE** = everything else (a bare `TransportFailed`, a handshake
```

One hit, unrelated, a doc comment. The gates emit `{name, binding, attempted, substrate_present, green, detail}` — a *four-way* truth (`attempted × green`) that no consumer projects onto the epic's four states. `epic-13:180` requires *"Every journey-relevant leg emits exactly one evidence state … plus its artifact reference when proven."* **Nothing emits one.**

### D-2 — `ABSENT` is declared, then exits zero

`xtask/src/check_reza_production_path.rs:1153` —

```rust
"passed": blockers.is_empty(),
```

and `LegResult::blocks` returns `!self.green && dev_enforced_red_blocks(class, self.substrate_present)`. A skipped `AdvisorySubstrate` leg has `attempted: false`, is never a blocker, and the gate prints:

```
check-reza-production-path: PASSED (live substrate advisory; 2 absent successors declared)
```

`ABSENT_SUCCESSORS` (`:17-20`) currently declares `"11.4b audit escape-anomaly detector wiring"` and **`"13.6 three-team product journey"`** — this story's own entry, announced in a banner attached to a green gate. Meanwhile `check_multi_tenant_loom.rs:16` has `ABSENT_SUCCESSORS: &[] = &[]`, so the epic's requirement that 13.1 declare journey legs ABSENT *"never disappear or silently green"* has, on that gate, **disappeared**.

This is the project's signature failure mode — a claim standing in for a control — sitting directly under the closer's AC6.

### D-3 — the 3×3 substrate is 2×0

| Axis | Required by 13.6 | Provisioned in CI |
|---|---|---|
| Teams | 3 distinct `datname` | **2** — `maos_team_a` (`discipline.yml:2687`), `maos_team_b` (`:2706`); no `MAOS_TEST_POSTGRES_TEAM_C` anywhere in the repo |
| Regions | 3 | **0** — `MAOS_TEST_POSTGRES_{A,B,C}` (`cross_region_live.rs:1877-1881`) has **zero occurrences under `.github/`**; `check-cross-region-consensus`'s job (`discipline.yml:2611`) provisions no Postgres service |

The 3-region pilot legs — including `three_region_convergence_all_three_equal` and its **topology-fraud negative** (`cross_region_live.rs:2020-2024`: two regions sharing a database is fraud) — exist and have never executed in CI. That negative is exactly the control this story's AC1 needs, and it is dark.

### D-4 — NFR-Scale-5's axis is the host axis, and it is already measured

`NFR-Scale-5: Multi-host A2A peer mesh scales to 14-institution Cortex; v2.0 target with documented capacity envelope` (`prd/non-functional-requirements.md:152`).

**Institution ≠ team.** Reza is **one** institution with three teams. This is the same conflation the epic already caught and corrected for NFR-Ops-11 (*"the requirement is titled multi-operator … Reza is one operator with three teams, and the PRD conflated the two"*, `epic-13:65`). Repeating it here would be the same error twice in one epic.

The host axis **has** a measured artifact: Story 11.3 shipped `check-scale-churn` over a **real N=30 mTLS mesh** (floor ≥25) with derived-and-reconciled **distinct host identities** (cert fingerprint / bound `SocketAddr`) and a duplicate-identity fixture that hard-fails. 14 ≤ 25 ≤ 30. So the 14-institution envelope should be **derived from and reconciled against 11.3's measurement**, with the axis named honestly — not re-measured, and never asserted.

There is no capacity-envelope artifact in the repo today (`docs/release/` holds two unrelated files).

---

## Acceptance Criteria (6)

### AC1 — A real 3-team × 3-region substrate exists and its topology cannot be faked

**Given** CI provisions two databases on one server and no regions (D-3),
**When** the Reza substrate is provisioned,
**Then** three distinct team `datname`s and three physically-distinct region databases are provisioned for CI **and** documented for local use, with the env contract registered (`check-env-contract` is a live gate),
**And** the **topology-fraud negative controls a single stand-in cannot fake** are executed, not merely present: distinct-`datname` reconcile plus pre-replication **physical absence** (`cross_region_live.rs:2020-2024` idiom) — two teams or two regions sharing a database is **RED**,
**And** the previously-dark 11.2b 3-region pilot legs run,
**And** absence of the substrate makes every dependent leg **`ABSENT` (never green)** — enforced by AC5, not by a banner.

### AC2 — One real composition run, judging only what shipped

**Given** the substrate of AC1 and the mechanisms 13.1–13.5j + 13.6a,
**When** a single 3-team × 3-region run executes,
**Then** it composes per-team Postgres, physical + cryptographic tenant walls, asymmetric cross-team consent, the production crossing (13.6a), vetting, the Enterprise Spirit, the production collective route, collective lifecycle and per-team tenant audit — **through production entry points only**,
**And** a **constructed-but-unwired control fails**: deleting any one production wiring site reds the run — proven per site, not asserted,
**And** ⚠ **this story writes no mechanism.** If the run needs something that does not exist, that is a finding and a successor story — never a harness-local implementation (`epic-13:52`). Any such discovery is recorded with a named owner before this story closes.

### AC3 — Allowed collaboration with minimum disclosure

**Given** 13.6a made the crossing production-reachable,
**When** Reza obtains the consolidated cross-team proposal through an allowed A→B asymmetric share,
**Then** the crossing bundle carries **only policy-allowed provenance** — the flattened I11 chain (13.3b leaf v3) — and raw payload, secret-bearing fields and unconsented TL references are **negative controls that red**,
**And** the multi-hop distillation provenance lands **with the row** and dereferences within the consumer team's own database,
**And** **provenance-presence, never provenance-promise** (ADR-049 §7): no leg, artifact or doc asserts the bundle establishes *authorization* provenance.

### AC4 — Explainable refusal, recovery, and lifecycle reconciliation

**Given** governance refusals must be operable, not merely correct,
**When** each of (a) reverse B→A share without a grant, (b) stale tenant map lease, (c) vetting lapse or revocation, (d) legal hold on a destination copy is exercised,
**Then** each produces a **distinguishable** operator outcome naming the **responsible authority** and a **safe next action**, and retry succeeds **only** after a valid manifest / consent / vetting repair,
**And** a source-team erase or hold exercises 13.5b against destination copies so both audit sides reconcile `erased` / `held` / `failed`; a **one-sided result or an unauthorized hold bypass is RED**,
**And** the four causes remain distinguishable **end-to-end through production surfaces**, not only over hand-built error values.

### AC5 — ⚠ THE CLOSER'S OWN MECHANISM: the evidence ledger, and `ABSENT` that actually blocks

**Given** the four evidence states are prose (D-1) and `ABSENT` exits zero (D-2),
**When** the Reza gates run,
**Then** every journey-relevant leg emits **exactly one** of `PROVEN_BLOCKING` / `PROVEN_LIVE_SIGNED` / `ABSENT` / `INDETERMINATE`, plus its **artifact reference** when proven, in machine-readable output,
**And** the projection is **derived from observed leg outcomes**, never a hand-maintained list — a leg added without an evidence state is a hard failure, and a planted lie (a leg claiming `PROVEN_LIVE_SIGNED` with no artifact) **reds**,
**And** a **required** leg in `ABSENT` or `INDETERMINATE` **blocks the Reza/v2.2 product claim** by a mechanism that returns non-zero — the currently-vacuous `passed: blockers.is_empty()` path is closed for required legs, and `ABSENT_SUCCESSORS` is reconciled on **both** Reza gates (`check_multi_tenant_loom.rs:16` is empty; `check_reza_production_path.rs:17-20` still names this story),
**And** development-lane enforcement stays separable from the product claim: an unavailable substrate **may** leave a dev lane advisory while its evidence state is `ABSENT` — the two are orthogonal (`epic-13:180`) and both are recorded.

### AC6 — NFR-Scale-5 as a measured, correctly-axed capacity envelope — and boundary preservation

**Given** NFR-Scale-5 names the **host/institution** axis, Reza is **one** institution with three teams (D-4), and Story 11.3 measured a real N=30 mesh with distinct-identity reconcile,
**When** the capacity envelope is published,
**Then** it is a **documented artifact derived from and reconciled against 11.3's measured evidence** (`check-scale-churn`), stating the axis explicitly — **never** an assertion that 14 institutions executed, and never a re-labelling of three teams as fourteen institutions,
**And** the envelope names what it does **not** cover: the 30-day soak (NFR-Scale-1) and absolute geo-SLO remain release-gate artifacts, and 100-host churn is Epic 14,
**And** boundary preservation holds at close: physical absence, team-key source-reflex, provenance minimum-disclosure, tenant TL isolation, duplicate/correlation reconciliation,
**And** the **final kernel baseline is verified** after 13.5b/13.5d/13.5e/13.5h/13.5j/13.6a (`epic-13:195`) — pin `23401` at this story's start; confirm, do not assume,
**And** every stale artifact claim is corrected: the four NFR-Ops-11 sites (`requirements-inventory.md:242,514`; `implementation-readiness-report-2026-07-10.md:43,54`), and the epic file's **"Eleven stories total"** against an actual 17.

---

## Traps

1. **Do not build a mechanism.** The single rule that makes this story worth having. A missing mechanism is a finding with a named owner, never a harness-local implementation.
2. **The judge must not grade its own code.** AC5's ledger is this story's own mechanism — so the ledger needs its **own** falsification (a planted lie must red it), independent of the journey it reports on.
3. **`ABSENT` must be non-zero for required legs, and the four states must be derived.** A hand-maintained state list is the same null control in a new costume.
4. **Institution ≠ team (D-4).** Relabelling 3 teams as 14 institutions would repeat, inside the closer, the exact conflation the epic caught for NFR-Ops-11.
5. **The substrate is the expensive half.** Budget AC1 first. Nine databases across three regions is CI work with real runtime cost — decide the topology before writing legs.
6. **`skipped ≠ passed`.** Live legs `.expect()` their own env var (13.5g pattern).
7. **One `#[test]` per `--exact` leg** — the gates' anti-vacuity oracle is `"running 1 test"` + `"1 passed"`.
8. **Proven-red per limb**, byte-identical restore, serialized mutations.
9. **Carry 13.5g's open finding into the judgement:** `store.rs:419-433` `init_schema` acquires a second pooled client after the guard validated the first, with a comment asserting the opposite. Benign at `pool_size ≥ 2`; hangs at the legal `pool_size: 1`.
10. **`cargo run -q -p xtask -- <cmd>`.** No `cargo xtask` alias.

---

## Tasks

- [ ] **T1 (AC1)** — Design the 3×3 topology; decide database count and CI cost. Provision in `discipline.yml`; register the env contract; document local setup.
- [ ] **T2 (AC1)** — Turn on the dark 11.2b 3-region pilot legs; execute both topology-fraud negatives; prove each reds on a shared-database fixture.
- [ ] **T3 (AC5)** — Build the evidence ledger: derive one of four states per leg from observed outcomes + artifact reference; machine-readable; **falsify with a planted lie**.
- [ ] **T4 (AC5)** — Close the vacuous-pass path for **required** legs on both Reza gates; reconcile `ABSENT_SUCCESSORS` on both; prove an `ABSENT` required leg returns non-zero.
- [ ] **T5 (AC2)** — Compose the single 3×3 journey run over production entry points; per-site dead-wire falsification.
- [ ] **T6 (AC3)** — Minimum-disclosure negatives: raw payload, secret-bearing fields, unconsented TL refs.
- [ ] **T7 (AC4)** — Four refusal/recovery scenarios + erase/hold reconciliation; one-sided result and unauthorized hold bypass RED.
- [ ] **T8 (AC6)** — Author the capacity-envelope artifact derived from `check-scale-churn`'s N=30 evidence; state the axis; name the exclusions.
- [ ] **T9 (AC6)** — Correct the four NFR-Ops-11 stale sites and the epic's story count; verify the final kernel baseline.
- [ ] **T10** — Gates: `check-kernel-baseline`, `kloc-check`, `check-multi-tenant-loom`, `check-reza-production-path`, `check-scale-churn`, `check-env-contract`, `cargo fmt --all -- --check`, `cargo test --workspace`. Record the dev model.

---

## Dev Notes

### Ownerless items a closer should surface (not necessarily close)

These have accumulated without owners and will otherwise close with the epic in silence:

- **`check-fkcs` exits 0 on a red oracle.**
- **49 in-`src` kernel test modules** are budget-charged and **CI-unexecuted**.
- **No gate reconciles the kernel pin literal with its HISTORY rows.**
- **`check_ship_gate_completeness` never validates CI→registry** — the shared upstream of two known null controls (`abi-diff` value-erasure; `POST_V1_SCHEMA_SECTIONS`) and of `check-abi-ratification` having no CI job (`epic-13:221`).
- **`maos-bench --bench audit_query_latency` has been broken since Story 9.1** (`77a34d0f`) — panics `UnknownKind("capability.invoke")`; the accepted string is `capability.invocation`. Invisible because CI *builds* `--all-targets` but only *runs* four other benches. Same family as the 49 unexecuted modules.
- **kloc ceilings re-based at "done" for four consecutive stories**, and ~20 unlisted crates escape the per-crate ceiling entirely (13.4 retro flag).
- **NFR-Ops-11 sub-axes (i) namespace and (iii) capability-token signing key** — open and **ownerless**, not "scheduled" (`epic-13:71`).

### Budget

- **kernel-core:** ZERO expected @ **23401**; verify the final baseline after 13.6a. *"kernel-core ZERO" ≠ "zero delta"* — state both.
- **fkcs:** frozen `23081`, byte-untouched.
- **kloc:** check `xtask` and `maos-bench` ceilings before writing.

### References

- [Source: `epic-13-reza-cortex-v2-2.md#157-163`] — the original 13.6 AC sketch.
- [Source: `epic-13-reza-cortex-v2-2.md#180`] — evidence state vs enforcement class.
- [Source: `epic-13-reza-cortex-v2-2.md#65-71`] — NFR-Ops-11 operator/team conflation; the precedent for D-4.
- [Source: `prd/non-functional-requirements.md#152`] — NFR-Scale-5.
- [Source: `_bmad-output/implementation-artifacts/11-3-scale-envelope-25-30-host-churn.md`] — measured N=30 mesh, distinct-identity reconcile.
- [Source: `deferred-work.md#548`] — skipped `AdvisorySubstrate` legs emit `passed: true`.

---

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List

---

## Change Log

| Date | Change |
|---|---|
| 2026-07-28 | Created from the grounding pass at `cb412348`. Reframed twice: crossing mechanism split OUT to 13.6a (operator-ratified); evidence ledger + substrate scoped IN as the closer's real deliverable. D-1…D-4 measured. 6 ACs. Status `blocked` on 13.6a. |
