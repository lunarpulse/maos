---
baseline_commit: b568a052
depends_on: none — substrate-independent and buildable now (13.6a/13.6b/13.6c/13.6d all `done`, but this story needs none of their substrate)
blocked_by: none
blocks: 13-6-reza-cortex-journey-closer-nfr-scale-5
kernel_grant: NONE — ZERO maos-kernel-core Δ expected, pin **23679** (NOT 23401 — re-measured, see Budget)
kloc_grant: ✅ **AUTHORIZED 2026-08-04 (FLAG-Winston, operator-granted) — `xtask` ONLY, at the formula, number set POST-MEASUREMENT.** Not a number today: `kloc.toml`'s rule derives it (`measured + max(100, ceil(0.02×measured))`), so the ceiling cannot exist before the code. Full terms in Dev Notes → `### FLAG-Winston authorization`.
---

# Story 13.6e — Judge machinery: the evidence ledger, and `ABSENT` that actually blocks

Status: **done** — closed 2026-08-11 with Story 13.6 at clean commit `fd1ce75c`. The reopened defect is fixed: published-ledger validation now requires the exact gate-owned leg set, so a ledger that omits a gate-mandatory leg can no longer validate or recompute a claim.

**Kernel-Δ: ZERO expected @ 23679.** Work lands in `xtask/`, `.github/workflows/discipline.yml`, and the live test harnesses that must sign their own transcripts. **No new gate.** **No new crate. No new dependency.**

> **Why this story exists, and why it is not part of the closer.**
>
> Story 13.6 is the Epic-13 closer and the epic forbids it from inventing mechanisms (*"13.6 is last and only judges"*, `epic-13:57`). The 13.6 grounding pass then found that **the judge has no instrument**: the four evidence states the epic's own gate discipline requires (`epic-13:200`) exist nowhere in code, and the two mechanisms that are supposed to stop an unproven claim both return zero. Pre-dev checklist item 5 (`epic-13:253`) says split rather than hide implementation in 13.6 — applied five times already in this epic. **Story 13.6c drew this exact boundary in its own shipped text, three times**, naming this story before it existed:
>
> - `13-6c-three-team-three-region-substrate.md:88` — *"Boundary: this story does not add `BindingClass` to these two gates. That is judge machinery and belongs with 13.6/13.6e."*
> - `:163` — *"this story does **not** attempt the leg-level `BindingClass` fix for these two gates, nor the cross-gate `passed: blockers.is_empty()` fix — both belong to **13.6's evidence ledger / 13.6e's judge machinery**."*
> - `:185` (Trap 8) — *"Do not add `BindingClass` or fix the `AdvisorySubstrate` vacuous-pass here — 13.6/13.6e own them."*
>
> **Operator-ratified 2026-08-04.** The split is *ordering, not deferral* — the same rationale as 13.6a→13.6b. This story is substrate-independent and buildable immediately; the closer consumes it.

---

## Story

**As** the operator who has to decide whether the Reza/v2.2 product claim is true,
**I want** every journey-relevant gate leg to emit exactly one machine-derived evidence state with an artifact reference, and any required leg that is `ABSENT` or `INDETERMINATE` to return non-zero,
**so that** "the journey works" is a verdict a machine derived from evidence it produced — and a leg that never ran can no longer be silently counted as one that passed.

---

## The four defects this story closes — measured at `b568a052`, not asserted

### D-1 — the four-state vocabulary exists only in prose

```
$ grep -rn "PROVEN_BLOCKING\|PROVEN_LIVE_SIGNED\|INDETERMINATE" --include='*.rs' --include='*.toml' xtask/ crates/
crates/maos-cohort/src/halt_receipt.rs:128:/// - **INDETERMINATE** = everything else (a bare `TransportFailed`, a handshake
```

One hit, unrelated, a doc comment. `evidence_state` / `EvidenceState` / `product_claim` / `artifact_ref` across `xtask/` + `.github/` → **zero hits, all four**. `epic-13:200` requires *"Every journey-relevant leg emits exactly one evidence state … plus its artifact reference when proven."* **Nothing emits one.**

### D-2 — pre-implementation ground truth: **TWO** vacuity mechanisms on two gate families (both closed below)

This is the correction that reshaped this story. The original 13.6 draft blamed `passed: blockers.is_empty()` for everything. 13.6c's preflight then blamed `CURRENT_PHASE` instead (`13-6c…md:366`). **Both are right, about different gates.** Measured:

| Family | Gates | Vacuity mechanism | What escapes |
|---|---|---|---|
| **A** — leg-level `BindingClass` (Option C, E12-B1) | `check-multi-tenant-loom`, `check-reza-production-path` | Exit is `blockers.is_empty()` — `check_reza_production_path.rs:1175-1182`, `check_multi_tenant_loom.rs:1688-1695`. A skipped `AdvisorySubstrate` leg is `attempted:false, green:false` and `blocks()` returns `false` (`gate_common.rs:97-102`). | An **ABSENT** leg. A red *attempted* leg does block. |
| **B** — whole-gate phase coupling, never adopted Option C | `check-cross-region-consensus` (`:50`), `check-multi-region-slo` (`:54`) | `CURRENT_PHASE = "v1_5"` (`gate_common.rs:34`) + registry `v1_5 = "advisory"` (`gate-registry.toml:288,293`) ⇒ `Ok(())`. | A **RED live leg**. |

Neither Family-A gate imports `CURRENT_PHASE` (their `use` line is `check_reza_production_path.rs:14` / `check_multi_tenant_loom.rs:14`). Neither Family-B gate carries `BindingClass`. **A fix aimed at one family does nothing for the other.**

⚠ **Historical pre-fix evidence.** Family B's escape was not hypothetical: `13-6c-evidence/after-multi-region-slo.json` recorded `roundtrip-slo` RED at 36 099 µs while the phase-coupled gate exited 0. Story 13.6e retired that escape and the probe-fidelity repair below removed the false current RED without moving the 30 000 µs floor.

⚠⚠ **Do NOT reach for "it is a local rig vs CI's container" — that theory is dead, and the real cause is measured.** 11.2b's own GREEN at ≈21 ms was taken on the *same* rig class (`11-2b…md:294`: local Postgres 17 + pgvector, loopback, one cluster, debug `cargo test`) and it ran the whole gate green (`:308`). Same rig, same build mode, same leg — **the changed variable is the code.**

The timed span (`crates/maos-bench/tests/t_11_2b_cross_region_slo.rs:113-167`, N=200, 10 warm-up iterations discarded, connection/schema setup outside the clock) went from **5 SQL statements per A→B→A iteration to 17**, plus two extra Ed25519 verifies:

| | 11.2b (`c71ad33f`) | HEAD |
|---|---|---|
| `write_with_source` | 1 — single `INSERT … ON CONFLICT`, autocommit | **5** — `BEGIN` + `pg_advisory_xact_lock` + tombstone `SELECT` + 18-column upsert + `COMMIT` (`store.rs:619-708`) |
| `read_all_rows_from` ×2 | 2 | 2 |
| `apply_replication_bundle` ×2 | 2 | **10** (`bundle.rs:808-830`) — and `apply` now verifies internally (`bundle.rs:866`), which it did not at 11.2b |
| **total** | **5** — matching the floor's own provenance note *"~5 SQL ops"* | **17** |

⚠ **And the floor is a LOOPBACK regression tripwire, not a geo-SLO** — stated in four places, incl. `docs/adr/ADR-049…md:125` and `prd/non-functional-requirements.md:146` (*"NOT a geo-SLO — CI Postgres is co-located, so a real geo-RTT is physically unobservable"*). It was measured once on a dev rig at ≈21 ms and pinned at 30 000 µs with ~43 % headroom (`11-2b…md:296`), and then **the CI job that would have re-validated it had no Postgres for a month** — 13.6c added the substrate, so this leg has still never run in CI.

✅ **MEASURED, DIAGNOSED AND RESOLVED 2026-08-04 — the red is GONE and the floor was never touched.** Operator chose "reduce the cost, don't move the floor". Outcome: `check-multi-region-slo` is now **`oracle_green: true`**, all five legs green, clean p95 **17 802 µs** against the untouched **30 000 µs** floor.

**The cause was probe infidelity, not machinery.** A single `verify_replication_bundle` costs **7 137 µs** in this debug `cargo test` build, so verify *count* dominates the whole measurement. Story 13.2 moved verification **inside** `apply_replication_bundle`, but the 11.2b probe kept its now-redundant explicit pre-verify — so it counted **two verifies per leg where production counts one** (production's sole caller, `cross_team_crossing.rs:255`, calls `apply` directly). Removing the redundant call: **17 802 vs 11.2b's 16 535 = 1.08×**, i.e. ~8% genuine growth over five stories. Coverage unchanged — `apply` still verifies before any store access, and `.expect` panics if it fails. Full measurement chain, including a persuasive hypothesis that measurement killed, in `deferred-work.md` → *"Story 13.6e preflight — `roundtrip-slo` floor breach"*.

✅ **FALSIFIER CLOSED by the remaining review.** The final mutation test alternates adjacent clean/injected samples and requires the median per-pair delta to carry at least 14 ms of the fixed 15 ms injection. `paired_delta_oracle_requires_the_injection_on_a_majority_of_pairs` is its own proven-red, and the `roundtrip-slo` trusted mapping jointly requires the clean and mutation attestations.

`oracle_green` **is already computed in all four gates and gates nothing** — Family A: `check_reza_production_path.rs:1122`, `check_multi_tenant_loom.rs:1635`. It selects a word in the PASSED line and fills a JSON field. This is the null control the ledger replaces.

### D-3 — `LegResult` is not a shared shape. It is twelve structs in two incompatible families

The ratified 2026-07-30 design said the projection *"rides fields every leg already emits."* **That is true of 2 of 12 gates.**

- **Family A shape** (`{name, binding, attempted, substrate_present, green, detail}`, `#[derive(Serialize)]`): `check_reza_production_path.rs:28-36`, `check_multi_tenant_loom.rs:37-45` — byte-identical twins, along with `blocks()`, `class_name()`, `run_test_leg()`, `write_step_summary()` and the whole ~60-line tail.
- **Family B shape** (`{label, passed: u32, failed: u32, ran, [attempted,] green}` — **no `binding`, no `substrate_present`, no `detail`**): 10 files. Two of them (`check_vetting_attestation.rs:31`, `check_wasm_form_equiv.rs:104`) have **no `attempted` field at all** and therefore *cannot express `ABSENT`*.
- **Family C**: `check_loom_substrate_drift.rs:445-451` `GateVerdict{job, green, problems, exported, required}` — no leg vocabulary at all.

**Consequence for scope:** "every journey-relevant leg" must be given a mechanical definition, and Family B must be *migrated for the gates in scope*, not wished into shape. See AC1's scope rule.

### D-4 — `ABSENT_SUCCESSORS` is prose stored in a Rust const, and the two copies disagree

- `check_reza_production_path.rs:17-20` — two entries, including **`"13.6 Reza three-team traceback journey and NFR-Scale-5 evidence"`**.
- `check_multi_tenant_loom.rs:29` — `const ABSENT_SUCCESSORS: &[&str] = &[];`, under a 12-line doc comment (`:17-28`) that argues *"Empty is a claim, not a default"* and then names two controls it declines to list.

Consumers are a `join()`, a JSON field, and a `.len()`. **No test, no lint, no cross-file reconciliation reads either const.** ⚠ 13.6d rewrote the reza string; an older draft quoted `"13.6 three-team product journey"`, which no longer occurs anywhere (`grep -c` on the 13.6 story at HEAD = 0). Read the const, do not trust a quote of it.

### D-5 — `discipline.yml` has no artifact channel — but the repo does, and it must be copied, not reinvented

`grep -c upload-artifact .github/workflows/discipline.yml` → **0**. Every gate's JSON goes to stdout and dies with the job. `check_ship_gate_completeness::run(json: bool)` (`:90-203`) takes no artifact input; its only inputs are `discipline.yml` text and `gate-registry.toml`. A `product_claim` that no other job can read is a claim, not a control.

⚠ **This is a gap in ONE file, not in the repo.** There are **14** `upload-artifact`/`download-artifact` steps across **7** workflows, including three working producer→consumer evidence hand-offs: `multi-provider.yml:30→47`, `rpo-rto-cadence.yml:83→107`, `fuzz-cadence.yml:95→117`. **Copy the closest of these idioms.** Inventing a fourth is the same hazard as trap 7's "do not add a third YAML parser".

### D-6 — the ratified design names a primitive that does not exist

The 2026-07-30 preflight specified signing *"via `sealed_export::sign_bundle` under `MAOS_AUDIT_KEY_SEED`."* Measured:

- **`MAOS_AUDIT_KEY_SEED` does not exist.** Repo-wide it appears in exactly one place — a string literal *inside an error message* at `crates/maos-bin/src/main.rs:8475`. No code reads it. The real loader is `maos_domain::audit_key::load_audit_key_seed(&Option<PathBuf>)` (`crates/maos-domain/src/audit_key.rs:31`); its env var is **`MAOS_AUDIT_KEY`** (`:92`) and it holds a **filesystem path**, not a seed. Precedence: explicit path → `MAOS_AUDIT_KEY` → `~/.config/maos/audit-signing.key`; 0600 enforced; **fails loudly, no silent keygen** (`:34-42`).
- **`sign_bundle` is the wrong primitive.** It takes `BundleForSigning` (`sealed_export.rs:144-160`) with `entries: Vec<AuditEntry>`, `i12_digest_refs`, `i11_distilled_content`, `freshness` — an audit-bundle shape. Signing a gate transcript through it means fabricating `AuditEntry` rows.
- **The right primitives already exist and are already reachable from `xtask`.** See AC3.

---

## Acceptance Criteria (5)

### AC1 — The four states exist, are DERIVED, and no leg in scope can omit one

**Given** the vocabulary is prose (D-1) and the leg shapes are three incompatible families (D-3),

**When** any gate in the **ledger set** runs,

**Then** every one of its legs carries exactly one `EvidenceState ∈ {PROVEN_BLOCKING, PROVEN_LIVE_SIGNED, ABSENT, INDETERMINATE}`, produced by **one pure projection function** over observed leg outcome fields — never a per-leg annotation and never a hand-maintained state list,

**And** the projection is exactly:

| Condition | State |
|---|---|
| `!attempted` | `ABSENT` |
| `attempted && green && class == Blocking` | `PROVEN_BLOCKING` (hermetic + reproducible; artifact = transcript ref, no signature required) |
| `attempted && green && class == AdvisorySubstrate && signature verifies` | `PROVEN_LIVE_SIGNED` |
| everything else attempted (`!green`, **or** green-live-**unsigned**) | `INDETERMINATE` |

**And** ⚠ **the ledger set is NOT a new list — it is `check_loom_substrate_drift.rs`'s shipped `CONTRACTS` table** (`:146,160,170,181`), whose four entries are already exactly `check-multi-tenant-loom`, `check-reza-production-path`, `check-cross-region-consensus`, `check-multi-region-slo`. **Derive the ledger set from `CONTRACTS` so the two cannot diverge.** Declaring a second list of the same four gates is the null control this AC exists to prevent,

**And** the escape control **already exists and is already blocking** — `run_service_block_drift` (`check_loom_substrate_drift.rs:513-541`) calls `discover_substrate_jobs` and diffs it against `CONTRACTS`, pushing a problem for any job that *"declares `services.postgres` + runs a gate but is not registered as a substrate job"* (`:532-535`), and the gate is `blocking` at all four phases (`gate-registry.toml:359-360`). **Do not rebuild it.** ⚠ `is_service_bearing_gate_job` and `discover_substrate_jobs` are **private** — widening them to `pub(crate)` for the derivation is authorized by this AC and is the only visibility change it permits. ⚠ **Do not add an "explicitly excluded with a written reason" escape hatch**: it does not exist today (the code reds unconditionally) and it would be a hand-maintained list, which trap 3 forbids,

**And** Family-B gates in the set are **migrated to the Family-A leg shape** (they gain `binding`, `substrate_present`, `detail`; `check_cross_region_consensus.rs:119` and `check_multi_region_slo.rs:111` already carry `attempted`). ⚠ **This is bigger than "add three fields":** `check_multi_region_slo.rs` has **20** `LegResult {` construction sites and `check_cross_region_consensus.rs` **6**, and each gate carries its **own private** `PHASE_ORDER` / `CURRENT_PHASE` / `read_disposition` (`check_multi_region_slo.rs:46,50,57`; `check_cross_region_consensus.rs:50,54,61`) while importing only `gate_common::emit_command`. Retiring `CURRENT_PHASE` per AC2 means retiring those local copies too,

**And** a leg added to a gate in the set **without** flowing through the projection is a **compile error** — prove it by adding one. (Prefer the compile-error route: a runtime "hard gate failure" for a leg that simply never registered is not expressible.)

⚠ **Explicit non-goal, recorded rather than silently skipped.** The other 8 Family-B gates are **out of scope**, and two of them (`check_vetting_attestation.rs:31`, `check_wasm_form_equiv.rs:104`) structurally *cannot* express `ABSENT` — they have no `attempted` field. Record this as a named finding with a named owner in `deferred-work.md`; do not migrate them here and do not claim the ledger covers "every gate".

### AC2 — `ABSENT` and `INDETERMINATE` block the product claim — on **both** vacuity mechanisms

**Given** D-2: a skipped `AdvisorySubstrate` leg escapes Family A's `blockers.is_empty()`, and a RED live leg escapes Family B's `CURRENT_PHASE = "v1_5"`,

**When** a **required** leg is `ABSENT` or `INDETERMINATE`,

**Then** the product claim is `NOT_PROVEN` and the mechanism carrying it **returns non-zero** — proven separately for each family, with a planted absence for A and a planted red for B, because *a fix aimed at one family does nothing for the other*,

**And** a **`required` axis exists on legs** — `TestLeg` is `{name, class, args}` today (`check_reza_production_path.rs:22-26`, `check_multi_tenant_loom.rs:31-35`) and has no required/optional dimension; `BindingClass::AdvisorySubstrate` is *not* it (advisory is about substrate availability, required is about whether the claim depends on the leg),

**And** the two Family-B gates adopt leg-level `BindingClass` (`gate_common.rs:78-102`, the Option-C shape their siblings already use) — this is the E12-B1 gate-binding-decay item and 13.6c's named boundary, discharged here,

**And** ⚠ **development-lane enforcement stays separable from the product claim** (`epic-13:200`): an unavailable substrate **may** leave a dev lane advisory while the evidence state is `ABSENT`. Both are recorded; neither is derived from the other. The existing dev-lane exit (`blockers.is_empty()` / `is_blocking_at`) is **not** changed for non-required legs,

**And** ⚠ **the `required` set is NAMED, and the local-run posture is stated.** At HEAD every one of the 17 `ABSENT` legs across the two Family-A gates is `AdvisorySubstrate`; marking them all `required` makes both gates exit non-zero on any machine without Postgres. Decide and write down which legs the *product claim* depends on, and state explicitly whether a required-`ABSENT` non-zero applies to the local lane, to CI, or only to the published `product_claim` — the dev-lane carve-out above is written only for **non**-required legs and does not cover this,

**And** ✅ **the red this AC would have exposed is already resolved (2026-08-04) — `check-multi-region-slo` is `oracle_green: true` with the 30 000 µs floor untouched**, so making Family B bind should not surface it. ⚠ Two things remain yours: **(i)** repair the still-non-discriminating `p95 >= 14_000` assert in `cross_region_roundtrip_mutation` (assert on the injected-vs-clean **delta**, with its own proven-red); **(ii)** if any *other* Family-B leg reds when you flip the binding, apply the governing rule (`13-6c…md:154`) — *"validate the harness first, then fix or hold advisory with a loud banner, owner, and tracking entry. **Never re-can a fixture, never silently relax a floor.**"* Hold with the existing banner idiom (`xtask/src/check_red_team_gate.rs:291-298,313-335`); ⚠ **do not build a waiver registry** — 10.4c D6 forbids it in terms, and a third `BindingClass` variant would be exactly that. Escalate rather than decide it in a dev pass,

⚠ Blast radius if a Family-B leg reds unmanaged: `check-multi-region-slo` → `v1-0-ship-gate` (`discipline.yml:3057` → fail step `:3110-3111`) → `v1-5-ship-gate` (`:3486`) → `aggregate` (`:3474-3478`). **One leg, four red jobs** — and 13.6 cannot absorb it, because 13.6 is blocked on this story,

**And** the same-class defect at **`check_fkcs.rs:325`** is closed in this commit: it computes `dev_blocks` at `:260-264` with the identical Option-C comment as its siblings, then gates its exit on `blocking_now` instead — so a RED oracle prints `passed:false` in JSON and exits 0. One identifier. It is ownerless, it is the same sentence this AC is about, and it would otherwise close with the epic in silence.

### AC3 — `PROVEN_LIVE_SIGNED` is signed by the harness and verified against the operator-pinned key

**Given** D-6: the ratified design's env var is fictional and its signing entry point is audit-bundle-shaped,

**When** a live leg claims `PROVEN_LIVE_SIGNED`,

**Then** the **live test harness signs its own transcript record** — the gate must **not** sign post-hoc, which would attest *"the gate saw this text,"* not *"the test produced it"* (the `record-capture` idiom; this is trap #2, the judge must not grade its own code),

**And** the primitives are the ones that already exist — **invent no crypto**:
- canonical bytes: `maos_audit::sealed_export::canonicalize_value::<T>` (`sealed_export.rs:326`) — the one canonicalizer, ADR-028 D5b,
- sign: `maos_audit::release_verify::sign_sha256sums(bytes, &seed) -> [u8; 64]` (`release_verify.rs:174`),
- verify: `maos_audit::release_verify::verify_release_signature(bytes, &sig, &pubkey)` (`release_verify.rs:154`),
- key: `maos_domain::audit_key::load_audit_key_seed(&None)` (`audit_key.rs:31`) → `MAOS_AUDIT_KEY` path → `~/.config/maos/audit-signing.key`, 0600, fail-loud,
- pubkey: `maos_audit::sealed_export::derive_pubkey(&seed)` (`sealed_export.rs:371`) — **derived from the operator-pinned key, never read from the artifact** (R-RG1, `sealed_export.rs:84-90`),

**And** ⚠ **a gate already does this — copy it, do not invent it.** `xtask/src/check_trial_attestation.rs:10` imports `maos_audit::release_verify::{generate_sha256sums, sign_sha256sums}` and pairs it with derive-side producer functions (`:12-13`) and a `PROVENANCE_STAMP` (`:108`) under Story 11.7's producer/consumer derivation split. That is the closest shipped precedent for "a gate that verifies a signed, derivation-stamped artifact"; read it before writing T6/T7. `xtask` already depends on `maos-audit` and `maos-domain` (`xtask/Cargo.toml:53-54`) — **no manifest change, no new dependency, and no new code in `maos-audit`** (which has only 24 lines of kloc headroom; see Budget),

**And** **key location is operator-local by ratified design**: CI holds no operator key, so `load_audit_key_seed` returns `AuditKeyError::NotFound`. That must **downgrade gracefully** to `ABSENT`/`INDETERMINATE` with a written reason — *not* panic, and *not* fall back to a dev key. **CI dev-key signing is rejected**: a dev-key-forgible artifact named `PROVEN` is the 13.2 "trusted registry" category error replayed,

**And** the artifact reference is bound to the code and run actually tested. A clean local tree carries its HEAD SHA; a dirty local tree carries `<HEAD>+worktree:<sha256>` over HEAD, the full tracked binary diff, and every untracked path/content, plus a per-invocation nonce. GitHub Actions carries `GITHUB_SHA` plus a nonce derived from run id, run attempt, and gate, and the published consumer reconstructs that binding from its own workflow environment. An artifact cannot authorize its own commit, nonce, or gate/leg→test mapping.

### AC4 — The ledger is falsifiable: three planted lies must red it

**Given** trap #2 — the ledger is this story's own mechanism, so it needs its own falsification independent of the journey it reports on,

**When** each blind is planted, serialized, one at a time,

**Then** all three red, and the restore is byte-identical (`diff -q`):

1. a leg claims `PROVEN_LIVE_SIGNED` with an **empty** `signature_block`,
2. a signature is **present but verification fails** (wrong key, or tampered transcript),
3. a signature **verifies but the artifact is not bound to this build** — stale `artifact_ref` (wrong commit SHA / wrong substrate nonce),

**And** two further controls prove the projection is not decorative:
4. a **planted ABSENT required leg** returns non-zero on a Family-A gate,
5. a **planted RED live leg** returns non-zero on a Family-B gate,

**And** ⚠ **the falsifier must plant the absence, not wait for it.** In CI both Family-A gates export every substrate var (`discipline.yml:2787-2790`, `:2850-2851`), so every advisory leg is *attempted* — the vacuity is a local/partial-outage phenomenon and will not appear on its own,

**And** each blind is **one `#[test]` per `--exact` leg** — the gates' only anti-vacuity oracle is `"running 1 test"` + `"1 passed"` (`check_reza_production_path.rs:88`), which is structurally blind to a null assertion.

### AC5 — The verdict travels, and `ABSENT_SUCCESSORS` stops being prose

**Given** D-4 (two divergent hand-maintained consts with zero readers) and D-5 (no artifact channel exists),

**When** a ledger-set gate completes,

**Then** it emits a machine-readable `product_claim: "PROVEN" | "NOT_PROVEN(<reasons>)"` alongside the per-leg evidence states and artifact references, set `PROVEN` only when **every required leg** is `PROVEN_BLOCKING` or `PROVEN_LIVE_SIGNED`,

**And** the verdict is **published as a workflow artifact** — this is net-new plumbing (`upload-artifact` count in `discipline.yml` is currently zero) — and **consumed** by `check_ship_gate_completeness`, which reds a ship badge asserted while any ledger-set gate reports `NOT_PROVEN`. This closes that gate's ownerless *"never validates CI→registry"* gap (`check_ship_gate_completeness.rs:90-203` reads only workflow text and registry text today),

**And** `ABSENT_SUCCESSORS` becomes **derived** from the legs that came back `ABSENT` this run, replacing both hand-maintained consts — `check_reza_production_path.rs:17-20` and the empty `check_multi_tenant_loom.rs:29` — reconciled so the two gates can no longer disagree,

**And** ⚠ **`check_reza_production_path.rs:19` currently declares this epic's own closer as an absent successor** in a banner attached to a green gate. After derivation it appears only if a leg actually came back `ABSENT`, and it disappears when 13.6 lands — because a leg proved it, not because someone deleted a string,

**And** ⚠ **derivation must not silently destroy the ownership record it replaces.** `check_multi_tenant_loom.rs:17-28` is a 12-line doc comment naming two controls *"owned by Story 13.6 to judge"* — the kernel collective-cause erasure, and the three-team journey closer. **Derived `ABSENT` comes from legs, and there is no leg for "the kernel erases every collective cause."** So deleting the const without a replacement deletes the only in-code record of that ownership. Either (a) register a real `ABSENT` leg that the kernel-cause claim would invert — which makes the hand-off mechanical and is the preferred outcome — or (b) preserve the ownership statement in a form something reads, and say which. **Do not delete it into prose.**

---

## Traps

1. **Two vacuity mechanisms, not one (D-2).** A fix aimed at Family A does nothing for Family B. Prove each separately or you have closed half the hole and claimed the whole.
2. **The judge must not grade its own code.** The harness signs; the gate verifies. A gate that signs attests only that it read the text.
3. **A hand-maintained state list is the same null control in a new costume.** Derive the state from observed outcomes. `oracle_green` is already computed in all four gates and gates nothing — do not add a sixth field nobody reads.
4. **`MAOS_AUDIT_KEY_SEED` does not exist (D-6).** Anything written against it will not compile. The var is `MAOS_AUDIT_KEY` and it is a **path**.
5. **`substrate_present` means "env var was non-empty", not "substrate reachable"** — `check_reza_production_path.rs:51-55`, `check_multi_tenant_loom.rs:60-72`. Exporting a garbage URL flips a leg from `ABSENT` to attempted. Today the direction is fail-safe (the test then fails → red → blocks), but `PROVEN_LIVE_SIGNED` must **never** treat "env set" as "substrate proven" — that is what the signature is for.
6. **`SignatureBlock` is ambiguous** — two distinct types with that name: `sealed_export.rs:135` and `erasure/proof.rs:37`. Import explicitly.
7. **Two YAML parsers already read `discipline.yml`** — `check_ship_gate_completeness.rs:217-271` hand-scans lines and `continue`s past blanks/comments at `:258` and `break`s at `:262`; `check_loom_substrate_drift.rs:317-320` uses `serde_yaml`. Extend the parsed one; do not add a third.
8. **`is_story10_ship_gate` (`check_ship_gate_completeness.rs:123-153`) is a silently-incomplete allowlist** — it omits `check-multi-tenant-loom`, `check-fkcs`, `check-vetting-attestation`, `check-escape-detector`, `check-enterprise-identity`, `check-trial-attestation`, all of which ARE in `EXPECTED_GATES` (36 entries). Adding `product_claim` consumption there inherits the hole. Derive or fix it, do not extend it — ⚠ **and fixing it is safe**: all six already carry `[[ship_gate]]` rows in `gate-registry.toml`, so the disposition check will not red on them.
9. **Stale copy-paste text**: `check_multi_tenant_loom.rs:82` and `:1640` still say *"two-datname Postgres substrate"* while `live_substrate_present()` at `:60-72` now requires **three** (13.6c). A stale failure message reads like a specification — the 13.6d escalation of the 13.6a T1 precedent. Fix it in this commit.
10. **One `#[test]` per `--exact` leg.** Two tests behind one leg name defeats the oracle.
11. **Live legs `.expect()` their own env var** — skipped ≠ passed (the 13.5g pattern).
12. **Proven-red per limb, serialized, byte-identical restore.** Do not batch mutations.
13. **`cargo run -q -p xtask -- <cmd>`.** There is no `cargo xtask` alias.
14. **Measure the kloc baseline in an isolated `git worktree`** (the 13.5i error, fixed at 13.5j). **Code first, then the pin.** 13.6d halted before writing code on a red `kloc-check` it did not cause — read Budget before you start.
15. **`abi-diff` is not evaluable on a dirty worktree** — `git stash` first.

---

## Tasks

- [x] **T1 (AC1)** — Author the `EvidenceState` enum + the pure projection over `{attempted, green, class, signature-verified}`. One function, one home (`xtask/src/gate_common.rs`, after `dev_enforced_red_blocks` at `:102`). Unit-test the truth table exhaustively.
- [x] **T2 (AC1)** — Migrate `check_cross_region_consensus.rs:119` and `check_multi_region_slo.rs:111` legs to the Family-A shape (add `binding`, `substrate_present`, `detail`; they already have `attempted`). Additive only.
- [x] **T3 (AC1)** — Derive the ledger set from `check_loom_substrate_drift.rs`'s `CONTRACTS` (`:146-186`); widen `discover_substrate_jobs`/`is_service_bearing_gate_job` to `pub(crate)` if needed. The **job-level** escape control already ships at `:513-541` — verify it covers the set and do not rebuild it. Separately prove the **leg-level** guarantee (a leg with no evidence state does not compile).
- [x] **T4 (AC2)** — Add the `required` axis to `TestLeg` on all four gates; wire `product_claim`; prove non-zero for a planted ABSENT (Family A) **and** a planted RED (Family B).
- [x] **T5 (AC2)** — ✅ **The `roundtrip-slo` floor breach is RESOLVED (2026-08-04, probe-fidelity fix; floor untouched, gate `oracle_green: true`). Do not re-litigate it.** Yours: **repair the `p95 >= 14_000` assert** in `cross_region_roundtrip_mutation` (delta-based, own proven-red — see D-2). Then adopt leg-level `BindingClass` on the two Family-B gates, retiring their private `PHASE_ORDER`/`CURRENT_PHASE`/`read_disposition`/`phase_disposition`/`is_blocking_at` (`check_multi_region_slo.rs:50,54,61-75,79-87,90-95`; `check_cross_region_consensus.rs:50,54,61`). Fix `check_fkcs.rs:325` (`blocking_now` → `dev_blocks`) — ⚠ safe: all six gates missing from `is_story10_ship_gate` already have `[[ship_gate]]` rows, so fixing the allowlist reds nothing.
- [x] **T6 (AC3)** — Harness-side signing: `canonicalize_value` → `sign_sha256sums` under `load_audit_key_seed`; local clean/dirty worktree identity or GitHub run identity + substrate nonce in `artifact_ref`. Add `signature_block` + `artifact_ref` to the leg shape.
- [x] **T7 (AC3)** — Gate-side verification: `derive_pubkey` from the operator-pinned seed, `verify_release_signature`. **Graceful `AuditKeyError::NotFound` downgrade in CI — never panic, never a dev key.**
- [x] **T8 (AC4)** — Five blinds (empty sig / verify-fail / stale binding / planted ABSENT / planted RED), serialized, each proven-red, byte-identical restore verified by `diff -q`.
- [x] **T9 (AC5)** — `upload-artifact` plumbing in `discipline.yml` (first in the file); `check_ship_gate_completeness` consumes `product_claim` and reds a ship badge over `NOT_PROVEN`.
- [x] **T10 (AC5)** — Derive `ABSENT_SUCCESSORS` on both Family-A gates and reconcile. ⚠ **Before deleting `check_multi_tenant_loom.rs:17-28`, land its replacement** (AC5's last clause) — preferably a real `ABSENT` leg for the kernel collective-cause erasure, so 13.6's hand-off is mechanical. Fix the stale "two-datname" strings (trap 9).
- [x] **T11** — Record the out-of-scope Family-B gates as a named finding with a named owner in `deferred-work.md` (AC1's explicit non-goal).
- [x] **T12** — Gates: `check-kernel-baseline`, `kloc-check` (**expect FLAG-Winston**), `check-multi-tenant-loom`, `check-reza-production-path`, `check-cross-region-consensus`, `check-multi-region-slo`, `check-loom-substrate-drift`, `check-ship-gate-completeness`, `check-fkcs`, `cargo fmt --all -- --check`. Record the dev model in the Dev Agent Record.

### Review Findings

- [x] [Review][Patch] [HIGH] Required green-but-unsigned evidence exits zero [xtask/src/evidence_ledger.rs:514]
- [x] [Review][Patch] [HIGH] Verification proof is publicly forgeable inside xtask [xtask/src/evidence_ledger.rs:254]
- [x] [Review][Patch] [HIGH] Signatures are gate-bound, not leg/outcome-bound [xtask/src/evidence_ledger.rs:361]
- [x] [Review][Patch] [HIGH] Configured audit-key failures are silently downgraded [xtask/src/evidence_ledger.rs:280]
- [x] [Review][Patch] [HIGH] Ship consumer trusts an unvalidated PROVEN string [xtask/src/evidence_ledger.rs:749]
- [x] [Review][Patch] [MEDIUM] Ledger write/upload/download failures are non-fatal [xtask/src/evidence_ledger.rs:806]
- [x] [Review][Patch] [HIGH] Kernel oracle resolves source from the wrong working directory [xtask/tests/story_13_6e_evidence_ledger.rs:30]
- [x] [Review][Patch] [HIGH] Kernel oracle mistakes payload binding for cause discrimination [xtask/tests/story_13_6e_evidence_ledger.rs:64]
- [x] [Review][Patch] [MEDIUM] Ledger consumer is absent from v1-0 ship-gate needs [.github/workflows/discipline.yml:3090]

#### Remaining-chunk review — contract and escape controls

- [x] [Review][Patch] [HIGH] Restrict the FKCS hold to the exact recorded admission mismatch [xtask/src/check_fkcs.rs:320]
- [x] [Review][Patch] [MEDIUM] Align the FKCS advisory flag with the actual blocking verdict [xtask/src/check_fkcs.rs:356]

#### Remaining-chunk review — harness and evidence producers

- [x] [Review][Patch] [HIGH] Validate published ledger bindings against the consuming workflow run [xtask/src/evidence_ledger.rs:931]
- [x] [Review][Patch] [HIGH] Bind each published leg to a trusted expected-test mapping [xtask/src/evidence_ledger.rs:967]
- [x] [Review][Patch] [HIGH] Verify the mutation attestation before proving the roundtrip SLO leg [xtask/src/check_multi_region_slo.rs:313]
- [x] [Review][Patch] [MEDIUM] Measure paired latency deltas instead of subtracting sequential batch percentiles [crates/maos-bench/tests/t_11_2b_cross_region_slo.rs:365]

#### Remaining-chunk review — Family-B operational gates

- [x] [Review][Patch] [HIGH] Isolate consensus legs from unrelated ignored tests and require a non-empty consensus substrate [xtask/src/check_cross_region_consensus.rs:251]
- [x] [Review][Patch] [HIGH] Keep the structural read-path chokepoint blocking without Postgres [xtask/src/check_multi_region_slo.rs:387]
- [x] [Review][Patch] [HIGH] Run and verify the live scan identity test that the leg claims [xtask/src/check_multi_region_slo.rs:422]
- [x] [Review][Patch] [MEDIUM] Derive non-empty A/B and A/B/C substrate predicates per SLO leg [xtask/src/check_multi_region_slo.rs:115]
- [x] [Review][Patch] [MEDIUM] Report green-but-unsigned successful gates as advisory [xtask/src/evidence_ledger.rs:1349]
- [x] [Review][Patch] [LOW] Remove the clean roundtrip sink when mutation spawning fails [xtask/src/check_multi_region_slo.rs:298]

#### Remaining-chunk review — Family-A operational gates

- [x] [Review][Patch] [HIGH] Register both deleted Reza successor controls as observational legs [xtask/src/check_reza_production_path.rs:1021]
- [x] [Review][Patch] [MEDIUM] Project the enabled hermetic kernel oracle as blocking proof [xtask/src/check_multi_tenant_loom.rs:110]
- [x] [Review][Patch] [MEDIUM] Recognize nested transport-cause mappings in the kernel probe and oracle [xtask/src/check_multi_tenant_loom.rs:61]
- [x] [Review][Patch] [MEDIUM] Surface kernel-source read failures instead of fabricating absence [xtask/src/check_multi_tenant_loom.rs:99]
- [x] [Review][Patch] [MEDIUM] Bind local ledger artifacts to the actual dirty working tree [xtask/src/evidence_ledger.rs:296]

#### Remaining-chunk review — documentation and planning artifacts

- [x] [Review][Patch] [HIGH] Apply the authorized post-review KLOC ceilings and replace stale completion measurements [xtask/kloc.toml:203]
- [x] [Review][Patch] [HIGH] Refresh and unblock Story 13.6 from the shipped judge-machinery facts [13-6-reza-cortex-journey-closer-nfr-scale-5.md:3]
- [x] [Review][Patch] [MEDIUM] Separate the original nine-finding review from the five remaining review chunks [13-6e-judge-machinery-evidence-ledger.md:300]
- [x] [Review][Patch] [MEDIUM] Record 30 guards as 29 ordinary proofs plus one jointly verified mutation falsifier [sprint-status.yaml:235]
- [x] [Review][Patch] [MEDIUM] Document clean-GitHub and dirty-local artifact binding separately [13-6e-judge-machinery-evidence-ledger.md:184]
- [x] [Review][Patch] [MEDIUM] Record all five non-required legs and six current Reza ABSENT successors [13-6e-judge-machinery-evidence-ledger.md:426]
- [x] [Review][Patch] [MEDIUM] Distinguish enforced unsigned blocking from the resolved historical roundtrip RED [sprint-status.yaml:235]
- [x] [Review][Patch] [MEDIUM] Replace the preimplementation epic row and add 13.6e to compiled context [epic-13-reza-cortex-v2-2.md:54]
- [x] [Review][Patch] [MEDIUM] Assign owners and closure criteria to ownerless deferred work [deferred-work.md:619]
- [x] [Review][Patch] [MEDIUM] Remove duplicate unattributable coverage-run records [intent-lineage-coverage-report.md:612]
- [x] [Review][Patch] [MEDIUM] Replace the unrelated blind-5 citation with the paired-delta proven-red test [deferred-work.md:618]

#### Reopened by Story 13.6 review — 2026-08-08

- [x] [Review][Patch] [HIGH] Require the complete gate-owned leg set when validating a published ledger [xtask/src/evidence_ledger.rs] — `expected_ledger_legs(gate)` dispatches to per-gate `ledger_leg_names()` accessors DERIVED from each gate's own leg declarations (no second hand-maintained list), and `validate_against` fails on any missing gate-owned leg or unknown serialized leg BEFORE the product-claim comparison. Tests cover omission (named missing leg), unknown extra leg, exact-set success, and the ordering guarantee that a ledger missing the journey leg cannot reach a `PROVEN` comparison. Verified: `cargo test -p xtask evidence_ledger` green; all four gates re-published at `fd1ce75c` validate under the new rule and are consumed by `check-ship-gate-completeness`.

#### Original nine-finding review resolution — 2026-08-04

All nine original review patches applied. Original-pass verification: 17 evidence-ledger unit tests passed; 8 ship-consumer unit tests passed; both then-affected live-test binaries compiled; `check-loom-substrate-drift` passed; enforced missing-substrate smoke produced a ledger and exited non-zero; the successor oracle reached its intended collapse assertion from the xtask package working directory; formatter check passed. The original pass measured `xtask 34,604/34,736`. These receipts predate the later five-chunk review and are not its verification evidence.

#### Remaining five-chunk review resolution — 2026-08-04

All 28 remaining-review patches applied: 17 code findings across contract controls, evidence producers, Family-B gates, and Family-A gates; 11 documentation/config findings. No decision or defer items remain. Final verification:

- `cargo check -p xtask` passed;
- evidence-ledger tests: 21 passed across 28 targets; ship-consumer tests: 8 passed across 28 targets;
- all seven live producer binaries compiled (`maos-loom-lite` ×2, `maos-bench` fault-injection ×1, `maos-bin` ×4);
- `check-loom-substrate-drift` passed: four env-consistent gates and four byte-identical service blocks;
- all four local gate smokes passed and wrote dirty-worktree-bound ledgers; `check-ship-gate-completeness --json` passed with `ledger_problems: []`;
- `check-fkcs --json` held only the exact Story-13.4 admission fingerprint and reported no blocking RED;
- `cargo fmt --all -- --check` passed;
- `kloc-check --json` passed at `xtask 35226/35931`, aggregate `141396/144224`, with no over-budget crates.

No live Postgres or operator audit key was available during this remaining review. The four smokes therefore prove local advisory/no-substrate behavior, artifact generation, successor projection, and consumer revalidation—not live signed success. Current CI remains unverified until pushed.

---

## Dev Notes

### Budget — read before writing code

- **kernel-core: ZERO expected @ 23679.** ⚠ **The pin is 23679, not 23401.** Verified by execution: `check-kernel-baseline: PASSED (maos-kernel-core/src = 23679 lines, pinned 23679)` and independently by hand-recount (96 `.rs` files). `xtask/kernel-core-baseline.toml:462`. The 23401 number in four story frontmatters and two ADR-055 bullets is **278 lines stale**.
  ⚠ **The HISTORY ledger has a gap.** `kernel-core-baseline.toml:438` records *"23596 -> 23517"* but **23596 has no originating row** — no documented delta ever pinned 23401→23596. The last two rows also abandon the `#   <N>  Story …` format used by all 28 prior rows. `grep -rn "HISTORY" xtask/src/` → **zero hits**: no gate reconciles the pin literal with its HISTORY rows. Do not fix the ledger here — record it; it is the closer's AC6.
- **kloc — final post-review grant applied from the authorization below.** Formatted measurement: `xtask 35226/35931` (705 lines of formula headroom), aggregate `141396/144224` (2828), `maos-audit 6641/6665` unchanged, `maos-kernel-core` physical pin 23679 unchanged. The remaining review temporarily breached the implementation-time ceilings; that was a new review-patch regression, not the older repository-wide state, and it was closed before completion.
  Context for the escalation remains: repeated story-level `xtask` grants are a decomposition signal for the Epic-13 retrospective, not permission to hide correctness defects.
- **fkcs:** frozen `23081`, byte-untouched.

### FLAG-Winston authorization receipt — final post-review ceilings (2026-08-04)

The operator authorized the measured formula before implementation: `ceiling = measured + max(100, ceil(2% × measured))`, and allowed `_aggregate_hardfail` to move only on an actual breach. Both post-review conditions were measured after formatting, so `xtask/kloc.toml` records:

- `xtask 34736→35931` from measured 35226 + 705;
- aggregate `140707→144224` from measured 141396 + 2828.

The fence held: zero `maos-audit` code, zero `maos-kernel-core` code/pin movement, no unrelated ceiling relief.

**What is NOT authorized — re-escalate rather than spend the grant elsewhere:**
- **`maos-audit` (24 lines of headroom) is fenced OFF.** AC3 is deliberately designed to add **zero** lines there — it calls `release_verify::{sign_sha256sums, verify_release_signature}` and `sealed_export::{canonicalize_value, derive_pubkey}`, all shipped, all already reachable from `xtask`. **If you find yourself editing `maos-audit`, that is a design deviation, not a budget problem. Stop and escalate.**
- **`maos-kernel-core`.** The pin stays **23679**. The ceiling and the pin are different mechanisms — the rule says so explicitly (*"the PIN is anti-DRIFT… the CEILINGS are anti-GROWTH"*). This grant touches neither the pin nor the kernel ceiling.
- **Ceiling relief as scope relief.** If the measured number implies the ledger outgrew the four gates in AC1's set, that is scope creep wearing a budget's clothes. Re-escalate.
- **Lowering anything.** Migration rule: this mechanism only ever raises.

**A standing observation, recorded rather than acted on.** This is a fourth consecutive story-level `xtask` grant across 13.6d, 13.6c, and the implementation/review phases of 13.6e. `xtask` is now ~35.2k LOC of gate machinery against a ~23.7k physical kernel. The Epic-13 retrospective owns the decomposition decision; this story does not trade correctness for ceiling headroom.

### The primitives, resolved against code (AC3)

| Need | Use | Why not the ratified choice |
|---|---|---|
| canonical bytes | `sealed_export::canonicalize_value::<T>` `:326` | — (this *is* the ratified canonicalizer, ADR-028 D5b) |
| sign arbitrary bytes | `release_verify::sign_sha256sums` `:174` | `sealed_export::sign_bundle` `:244` needs `BundleForSigning{entries: Vec<AuditEntry>, i12_digest_refs, i11_distilled_content, freshness}` — signing a transcript through it means fabricating audit rows |
| verify | `release_verify::verify_release_signature` `:154` | symmetric with the above |
| seed | `maos_domain::audit_key::load_audit_key_seed(&None)` `:31` | `MAOS_AUDIT_KEY_SEED` **does not exist** (D-6) |
| pubkey | `sealed_export::derive_pubkey(&seed)` `:371` | R-RG1: derive from the pinned key, never read it from the artifact |

Both `release_verify` functions are already reachable — `xtask/Cargo.toml:53-54` and `xtask/src/release_verify.rs:13`.

### Where the code goes

| Concern | File | Anchor |
|---|---|---|
| `EvidenceState` + projection | `xtask/src/gate_common.rs` | after `dev_enforced_red_blocks`, `:102` |
| Family-A leg shape (`signature_block`, `artifact_ref`, `required`) | `check_reza_production_path.rs:22-36`, `check_multi_tenant_loom.rs:31-45` | byte-identical twins — change both or they drift |
| leg construction | `run_test_leg` `check_reza_production_path.rs:57-101` (twin in `check_multi_tenant_loom.rs`) | the skipped-leg early return at `:58-67` is where `ABSENT` originates |
| verdict + exit | `check_reza_production_path.rs:1150-1182`, `check_multi_tenant_loom.rs:1663-1695` | `blockers.is_empty()` appears twice per gate: JSON `"passed"` and the `Ok/Err` |
| Family-B migration | `check_cross_region_consensus.rs:119`, `check_multi_region_slo.rs:111` | both already have `attempted` |
| Family-B binding | `check_cross_region_consensus.rs:50`, `check_multi_region_slo.rs:54` | `CURRENT_PHASE` consts to retire in favour of `BindingClass` |
| ship-gate consumption | `check_ship_gate_completeness.rs:90-203` | ⚠ traps 7 and 8 both live here |
| artifact upload | `.github/workflows/discipline.yml` jobs `:2750`, `:2794`, `:2611`, `:2672` | zero `upload-artifact` steps exist today |

### What this story does NOT do

- It does not compose the Reza journey, run a 3×3 topology, or judge any mechanism. **That is 13.6.**
- It does not migrate the 8 out-of-scope Family-B gates (AC1 non-goal, recorded with an owner).
- It did not relax the `roundtrip-slo` floor. The 36 099 µs result was historical pre-fix evidence; removing a redundant explicit verify restored 17 802 µs against the unchanged 30 000 µs floor, and the paired-delta mutation control now guards it. Current CI remains unverified until pushed.
- It does not touch `kernel-core`, `maos-audit`, or the kernel HISTORY ledger.

### CI state at HEAD — do not assume green

The last green CI run (`30758932431`, all 154 jobs) belongs to **`bd508469`**, which is **not an ancestor of `b568a052`** — it is a sibling, both children of `b45e12e6`, on no branch. Its `discipline.yml` content was re-landed in `b568a052`, so the substance survived, but **`b45e12e6` and `b568a052` have never been proven green in CI**, and `b568a052` moved the kernel pin and added a new kernel module. Push and re-run before making any evidence claim.

Locally at HEAD, with no Postgres: both Family-A gates exit 0 with `"passed": true, "oracle_green": false` and 13 / 4 legs `attempted:false` (every one of them `AdvisorySubstrate`; **zero red hermetic legs**). **That is D-2, reproducible on your machine right now.**

### References

- [Source: `epic-13-reza-cortex-v2-2.md#200`] — evidence state is separate from enforcement class; the four states; `ABSENT` never becomes green.
- [Source: `epic-13-reza-cortex-v2-2.md#253`] — pre-dev checklist item 5: split rather than hide implementation in 13.6.
- [Source: `13-6c-three-team-three-region-substrate.md#88,163,185`] — the boundary this story discharges, drawn by 13.6c before this story existed.
- [Source: `13-6c-evidence/SUMMARY.md#46-53,62-66`] — the live `roundtrip-slo` red behind the Family-B advisory; *"turning the region legs on does not make them binding."*
- [Source: `xtask/src/gate_common.rs#14-34,78-102`] — Option C, the two axes, `BindingClass`, `dev_enforced_red_blocks`.
- [Source: `deferred-work.md#548`] — skipped `AdvisorySubstrate` legs emit `passed: true` (filed 2026-07-25, still open).
- [Source: `project_gate_binding_decay`] — E12-B1, the standing owner of leg-level binding.

---

## Dev Agent Record

### Agent Model Used

`anthropic/claude-opus-5` (Oh My Pi harness, `bmad-dev-story` workflow), 2026-08-04.

### Debug Log References

#### The five blinds — serialized, each proven RED, byte-identical restore (T8/AC4)

Restore verified by SHA-256 over each touched file before and after; every entry below was planted alone, run, and reverted before the next.

| # | Blind | Where planted | Command | Result |
|---|---|---|---|---|
| 1 | an **empty** `signature_block` is accepted | `EvidenceVerifier::verify` — empty-signature guard flipped to `Ok(())` | `cargo test -p xtask --bin xtask empty_signature_block_is_not_proven` | **RED** — `test result: FAILED. 0 passed; 1 failed` |
| 2 | a signature **present but not verifying** is accepted | `EvidenceVerifier::verify` — `verify_release_signature`'s error discarded | `cargo test -p xtask --bin xtask signature_that_fails_verification_is_not_proven` | **RED** — exit 101, `FAILED` |
| 3 | a verifying signature **not bound to this build** is accepted | `EvidenceVerifier::verify` — commit + nonce comparisons deleted | `cargo test -p xtask --bin xtask signature_not_bound_to_this_build_is_not_proven` | **RED** — exit 101, `FAILED` |
| 4 | a **planted ABSENT required leg** on a Family-A gate | ENVIRONMENT, not source: no Postgres + `MAOS_LEDGER_ENFORCE=1` | `cargo run -q -p xtask -- check-reza-production-path --json` | **exit 1**, `"passed": false, "ledger_enforced": true`. Negative control (`MAOS_LEDGER_ENFORCE=0`): **exit 0**, `"passed": true` |
| 5 | a **planted RED live leg** on a Family-B gate | `run_three_region_convergence_leg` — the `!pg` arm returns `attempted:true, green:false, substrate_present:true` | `cargo run -q -p xtask -- check-multi-region-slo --json` | **exit 1** on the LOCAL lane (`ledger_enforced:false`) — leg-level `BindingClass` binds it. At HEAD the same red exited 0 through `CURRENT_PHASE = "v1_5"` |
| 6 | a **non-held RED leg** on `check-fkcs` (extra control for the hold below) | `run_release_graph_absence_leg` forced red | `cargo run -q -p xtask -- check-fkcs` | **exit 1**, `BLOCKING — oracle RED: release-graph-absence` — the hold is one name, not a class |

#### The compile-error guarantee (T3/AC1) — proved by adding one

Planted in `check_multi_region_slo.rs::run`, a leg pushed with a HAND-ANNOTATED state instead of a projected one:

```
xtask/src/check_multi_region_slo.rs:567:25: error[E0308]: mismatched types:
    expected `EvidenceVerdict`, found `EvidenceState`
```

`EvidenceVerdict`'s inner field is private to `gate_common`, and `EvidenceVerdict::project` is the only thing in the crate that can mint one. A leg cannot NAME a state it did not derive. Restored byte-identical.

#### `check_fkcs.rs:325` — the fix, and the RED it uncovered

Reverting the one-identifier change (`dev_blocks` → `blocking_now`) and re-running reproduces the defect exactly: **exit 0** over a RED `admission-path-unmodified` leg. With the fix: **exit 1**. The red is pre-existing — `git status` shows both admission sources untouched by this diff, and they last changed in Story 13.4 (`148a33ee`) while `admission_baseline.sha256` last moved in `5767cf0d` (Story 12.6). Disposition: **held advisory** with a loud banner, a named owner and a `deferred-work.md` entry (governing rule `13-6c…md:154`, second branch). Not re-pinned — re-pinning would re-can a fixture over another story's unreviewed admission-path change.

#### Original implementation-time gate runs (before the remaining five-chunk review)

| Gate | Exit | Evidence |
|---|---|---|
| `check-kernel-baseline` | 0 | `maos-kernel-core/src = 23679 lines, pinned 23679` — ZERO Δ |
| `kloc-check` | 0 | `aggregate=140224` after the FLAG-Winston grant; `_aggregate_hardfail` untouched (140224 < 140707) |
| `check-reza-production-path` | 0 | 71 `PROVEN_BLOCKING`, 4 `ABSENT`; `NOT_PROVEN(4 legs)`; 4 derived absent successors |
| `check-multi-tenant-loom` | 0 | 83 `PROVEN_BLOCKING`, 14 `ABSENT`; 14 derived absent successors incl. `kernel-collective-cause-distinguishable` |
| `check-cross-region-consensus` | 0 | 4 `ABSENT` live legs + `kernel-abi-diff` `PROVEN_BLOCKING`; `NOT_PROVEN(4 legs)` |
| `check-multi-region-slo` | 0 | 2 `ABSENT`, 1 `INDETERMINATE`, 2 `PROVEN_BLOCKING` |
| `check-loom-substrate-drift` | 0 | 4 gates env-consistent, 4 service blocks byte-identical — the workflow edits did not drift it |
| `check-ship-gate-completeness` | 0 | all 36 expected gates present; ledger consumer wired; no badge asserted over `NOT_PROVEN` |
| `check-fkcs` | 0 | 1 leg held advisory (named, owned, tracked); every other RED blocks |
| `cargo fmt --all -- --check` | 0 | clean |

### Completion Notes List

**AC1 — the four states, derived, unforgeable.** `EvidenceState` + `EvidenceVerdict::project` live in `gate_common.rs` immediately after `dev_enforced_red_blocks`, exactly as specified. The compile-error guarantee is structural rather than conventional: `EvidenceVerdict` is a newtype whose inner field is private to `gate_common`, so no gate module can construct one without calling the projection. The truth table is unit-tested **exhaustively** (all 2×2×2×2 inputs), not sampled. The ledger set is `check_loom_substrate_drift::contract_jobs()` — a derivation over the shipped `CONTRACTS` table — and a test welds it to the four gates' own `GATE_NAME` constants so a rename or a fifth contract reds. `discover_substrate_jobs`/`is_service_bearing_gate_job` did **not** need widening; the derivation reads `CONTRACTS` directly, so the authorized visibility change was not spent.

**A design decision worth naming: the twins were merged, not duplicated.** `LegResult`, `blocks()`, `class_name()`, `run_test_leg()`, `write_step_summary()` and the ~60-line tail existed as byte-identical copies in the two Family-A gates and as a third divergent copy across the two Family-B gates. Rather than edit four copies of the same tail, they became one `EvidenceLeg` + `run_exact_test_leg` + `finish_ledger_gate` in `evidence_ledger.rs`, called by all four. `check_reza_production_path.rs` lost 190 lines net; the "change both or they drift" hazard in the story's own routing table no longer exists.

**AC2 — the posture, stated.** `required` is a RULE, not a per-leg annotation: every ledger leg is required **by construction** except five named controls — `kernel-baseline-pinned`, `kernel-abi-diff`, and the three observational successors `kernel-collective-cause-distinguishable`, `audit-escape-anomaly-detector-wiring`, and `reza-three-team-three-region-journey`. A leg added tomorrow is required unless someone deliberately exempts it. ⚠ **Deviation from T4's letter, recorded:** the axis lives on `EvidenceLeg` (derived by `leg_is_required`) rather than as a `required: bool` field on `TestLeg`; this keeps the fail-safe default instead of writing `required:true` at every construction site.

The enforcement posture AC2 demanded be written down: **the GitHub Actions lane and the published claim — not the local lane.** Every required `ABSENT` or `INDETERMINATE` leg makes `product_claim` `NOT_PROVEN` and returns non-zero when `GITHUB_ACTIONS` is set (or `MAOS_LEDGER_ENFORCE=1`). Off that lane, absent substrate and green unsigned evidence remain advisory; attempted RED evidence with substrate present still blocks. `GITHUB_ACTIONS` rather than generic `CI` is deliberate because agent shells and editors export `CI=1` on developer machines. CI has no operator key by ratified design, so green live evidence downgrades safely to `INDETERMINATE` and the enforced lane refuses the claim.

**AC3 — who signs and what is bound.** `tests/harness/evidence_record.rs` is `#[path]`-included by seven live test files. Thirty guards emit only after normal test completion: 29 ordinary gate-proof records plus `cross_region_roundtrip_mutation`, which is jointly required with the clean record for `roundtrip-slo`. The canonical payload binds gate, exact test identity, `PASSED`, and run identity. Local dirty runs use a full worktree digest plus nonce; GitHub uses SHA + run id/attempt/gate and consumer-side binding reconstruction. Only `EvidenceVerifier` can mint verified proof; invalid configured keys fail loudly and key absence is the sole graceful local downgrade.

**AC4 — six blinds, not five.** The five the AC names, plus one proving the `check-fkcs` hold is a single name rather than a class. See the Debug Log table.

**AC5 — the verdict travels and is independently revalidated.** Four strict `upload-artifact` steps publish `tests/reports/evidence-ledger-<gate>.json`; missing uploads/downloads fail. `check-ship-gate-completeness` rejects malformed, duplicate, remapped, or stale artifacts, uses trusted gate/leg→test mappings, reconstructs the consuming GitHub run binding, reprojects every leg, verifies live records, and recomputes `product_claim`.

`ABSENT_SUCCESSORS` is derived. The two Reza controls deleted from prose are restored as observational legs (`audit-escape-anomaly-detector-wiring`, `reza-three-team-three-region-journey`), alongside Loom's `kernel-collective-cause-distinguishable`. Kernel source read failures block; nested cause mappings are recognized; once its probe flips, the hermetic kernel oracle is `PROVEN_BLOCKING` and needs no operator signature. Current no-substrate Reza output is 71 `PROVEN_BLOCKING` + 6 `ABSENT`, while only the four required live absences appear in `NOT_PROVEN(4 legs)`.

**Trap 8, fixed not extended.** `is_story10_ship_gate`'s allowlist became `LEGACY_PRE_REGISTRY_GATES` — a four-name denylist of the v1.0 infrastructure gates that predate the registry — so the disposition requirement is now the DEFAULT. That closes the silent omission of six gates and is verified safe by a test asserting (a) every non-legacy `EXPECTED_GATES` entry has a `[[ship_gate]]` row and (b) none of the four legacy names does.

**Trap 9.** The stale "two-datname Postgres substrate" strings are gone: the skip detail is now generated per gate by the shared runner, and the banner is derived. The two remaining occurrences of "two-datname" are a leg *name* (a 13.1 control genuinely about two datnames) and a comment about a two-datname witness — both accurate.

**Budget.** ZERO kernel-core Δ (pin 23679, verified by execution). ZERO lines in `maos-audit` (6641, unchanged — the grant's fence held). `xtask` measured **34054**, ceiling set to **34736** by the founder formula (`34054 + max(100, ceil(0.02×34054)=682)`), POST-MEASUREMENT, in `kloc.toml:203` with a HISTORY comment in house format. Aggregate measured **140224** against the `140707` hardfail, so `_aggregate_hardfail` was **not** touched, exactly as the grant terms require. `maos-loom-lite`, `maos-bench`, `maos-bin` and `maos-domain` are all byte-unchanged in measured LOC — every harness edit landed under `tests/`.

**Not done, deliberately, and recorded:** the eight out-of-scope Family-B gates (AC1's non-goal), the two that structurally cannot express `ABSENT`, `check-kernel-baseline`'s stdout pollution, and the `check-fkcs` admission-baseline hold — all four in `deferred-work.md` with named owners.

### File List

**Added**

- `xtask/src/evidence_ledger.rs`
- `xtask/tests/story_13_6e_evidence_ledger.rs`
- `tests/harness/evidence_record.rs`

**Modified**

- `xtask/src/gate_common.rs`
- `xtask/src/main.rs`
- `xtask/src/check_reza_production_path.rs`
- `xtask/src/check_multi_tenant_loom.rs`
- `xtask/src/check_cross_region_consensus.rs`
- `xtask/src/check_multi_region_slo.rs`
- `xtask/src/check_loom_substrate_drift.rs`
- `xtask/src/check_ship_gate_completeness.rs`
- `xtask/src/check_fkcs.rs`
- `xtask/kloc.toml`
- `.github/workflows/discipline.yml`
- `.gitignore`
- `crates/maos-bench/tests/t_11_2b_cross_region_slo.rs`
- `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs`
- `crates/maos-bin/tests/cross_team_consent_13_3.rs`
- `crates/maos-bin/tests/cross_team_crossing_13_6b.rs`
- `crates/maos-bin/tests/tenant_audit_phase_a_13_5g.rs`
- `crates/maos-loom-lite/tests/cross_region_live.rs`
- `crates/maos-loom-lite/tests/tenant_wall_live.rs`
- `_bmad-output/implementation-artifacts/deferred-work.md`
- `_bmad-output/implementation-artifacts/13-6e-judge-machinery-evidence-ledger.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/epic-13-context.md`
- `_bmad-output/implementation-artifacts/13-6-reza-cortex-journey-closer-nfr-scale-5.md`
- `_bmad-output/planning-artifacts/epics/epic-13-reza-cortex-v2-2.md`

---
## Change Log

| Date | Change |
|---|---|
| 2026-08-04 | Created by the 13.6 closer grounding pass at `b568a052`, operator-ratified **SIXTH SPLIT** of Epic 13 (13.6 → 13.6e judge machinery + 13.6 closer). Five adversarial scouts disproved six premises of the ratified 2026-07-30 AC5 design: `MAOS_AUDIT_KEY_SEED` **does not exist** (D-6); `LegResult` is **12 structs in 3 families**, not one shared shape (D-3); there are **TWO** vacuity mechanisms on two gate families, not one (D-2); **zero `upload-artifact`** steps exist so `product_claim` has no channel (D-5); the kernel pin is **23679**, not 23401; `check_multi_tenant_loom.rs`'s `ABSENT_SUCCESSORS` is at `:29` and the reza string was rewritten by 13.6d (D-4). Ledger set given a mechanical definition + an escape control. `check-fkcs`'s one-identifier same-class defect carved in. 5 ACs, 12 tasks, ZERO kernel-core @23679, kloc grant expected and flagged. |
| 2026-08-04 | **IMPLEMENTED (dev pass, `anthropic/claude-opus-5`), status `ready-for-dev` → `review`.** Shipped the sealed four-state ledger, fail-safe required axis, four gate migrations, 30 harness guards, artifact channel/consumer, and original implementation-time KLOC grant. Historical implementation receipt: `xtask 34054/34736`, aggregate `140224/140707`; three non-required controls at that stage. Later review rows supersede the implementation-only counts and binding/successor details. |
| 2026-08-04 | **ORIGINAL CODE REVIEW COMPLETE, status `review` → `done`.** Three parallel layers produced nine actionable findings (7 high, 2 medium); all were patched. This row and its focused verification predate the later five-chunk remaining review. |
| 2026-08-04 | **FIVE-CHUNK REMAINING REVIEW COMPLETE.** Fifteen parallel adversarial layers produced 28 actionable findings (17 code, 11 docs/config); all were patched, verified, and checked off above. Final KLOC authorization receipt: `xtask 35226/35931`, aggregate `141396/144224`. Story 13.6e remains `done`; Story 13.6 is unblocked to `ready-for-dev`. |

| **2026-08-07** | **REOPENED and re-closed — three defects found by Story 13.6's fourth-pass re-grounding, all fixed here rather than worked around in the closer.** **(1) The enforcement posture blocked on a state CI can never reach.** `blocks_product_claim` blocked every required `INDETERMINATE` on the enforced lane, but a GREEN-but-unsigned live leg projects `INDETERMINATE` precisely because no operator signature could be verified — and CI holds no operator key **by this story's own ratified design** (`evidence_ledger.rs`: *"a CI that holds the operator key would be theatre"*). The four journey gates were therefore unconditionally red in CI, with no configuration able to satisfy them. This also added a **third** enforcement axis on top of the two E12-B1 separated, contradicting `gate_common.rs`' own invariant (*"governs ONLY the GA ship-gate ladder — NEVER dev-time enforcement"*) three lines below its `project_gate_binding_decay` citation. **Fixed:** `INDETERMINATE` now blocks only when the leg is actually RED; a green-but-unsigned leg refuses the **claim** (`product_claim` = `NOT_PROVEN`) without blocking either **lane**, which is `epic-13:200`'s explicit split (*"an unavailable live substrate can remain advisory for a development lane while its evidence state is ABSENT, which prohibits the Reza completion claim"*). The refusal keeps its teeth at the GA phase: both Family-A gates are `v2_2 = "blocking"` and `check_ship_gate_completeness` refuses a badge over a non-`PROVEN` claim. `ABSENT` semantics are unchanged — in CI, absence still means the substrate did not come up and still blocks. The test that pinned the old behaviour was rewritten to assert the corrected contract, and a paired test now pins that an attempted RED leg still blocks on both lanes. **(2) `MAOS_LEDGER_ENFORCE=""` silently disabled the whole posture** — an exported-but-empty variable was read as an explicit opt-out. **Fixed:** empty is now "unset" and falls through to the default. **(3) The three-team journey successor was registered on a two-team gate.** `reza-three-team-three-region-journey` sat on `check-reza-production-path`, whose CI job provisions exactly `maos_team_{a,b}` and grows no third database by design (D-7) — so the leg could never be earned, while this story's own doc comment claimed the single registration meant *"the two gates cannot disagree."* **Fixed:** moved to `check-multi-tenant-loom`, whose contract already requires `MAOS_TEST_POSTGRES_TEAM_{A,B,C}` and which already runs 8+ legs from the same `cross_team_crossing_13_6b` harness — earnable with **no new database, no workflow change, no `CONTRACTS` change**. `absent_successor`/`failed_successor_probe` were promoted from a private copy in the Reza gate to shared `evidence_ledger` helpers rather than duplicated, per this story's own "merge the twins" principle; `kernel_probe_error` now delegates to the shared helper. Also corrected the kernel successor's detail string: the kernel collapses **eight** `TransportCause` variants, not "five". **Verification:** `cargo check -p xtask` clean with no new warnings; `cargo test -p xtask --bin xtask` **428 passed / 0 failed**; `check-loom-substrate-drift`, `check-multi-tenant-loom` (98 legs), `check-reza-production-path` (76 legs) and `check-ship-gate-completeness` all exit 0; journey leg confirmed present on the loom gate (`ABSENT`, non-required — Story 13.6 flips `required` when it lands the oracle) and absent from the Reza gate; `cargo fmt --all -- --check` clean. Story remains `done`. |
| 2026-08-08 | **REOPENED by Story 13.6 review.** `PublishedLedger::validate_against` recomputes a claim only from serialized legs and therefore accepts omission of the required Reza journey. The attempted Story 13.6 verifier patch was reverted to preserve that closer's declarations-only AC5 boundary. Repair and review stay owned here. |
| **2026-08-11** | **CLOSED with Story 13.6 at `fd1ce75c`.** The reopened omission is fixed at the root: each ledger gate exposes `ledger_leg_names()` derived from the same declarations it executes, `expected_ledger_legs(gate)` dispatches to them, and `validate_against` rejects any missing gate-owned leg or unknown serialized leg before recomputing `product_claim`. A published ledger can no longer earn a claim by omitting the leg that judges it. Verified on the operator lane: all four substrate gates re-published at `fd1ce75c` validate under the new rule, `check-ship-gate-completeness` consumes them, and the full xtask suite is green. |
