---
baseline_commit: "`5bcc3c76` — HEAD, tree clean. Every number and every `file:line` below was measured against a committed HEAD and is reproducible by `git checkout 5bcc3c76`. The preflight that produced this story ran six parallel scouts on 2026-08-26; where a scout's finding is quoted it was re-verified here before it was written down."
depends_on: "NOTHING. `14-0` is the entrance to Epic 14 and every other row in the epic is `backlog`. Its own inputs — the Epic-13 retro §4 table, `deferred-work.md`, and `epic-14-preflight-decisions.md` — are all landed."
blocks: "**`14-1-100-host-churn-scale-envelope`** (register binding rule 3: D1–D4 settled before 14-1 leaves `backlog`). Re-anchored by this story: **D5, D6 and D16 now also block `14-1`, not `14-4`** — see AC4.7."
kernel_grant: "NONE and none needed. `check-kernel-baseline` GREEN at **24472 == 24472** at `5bcc3c76`. Zero lines of `crates/maos-kernel-core/src` are touched by any AC. ⚠ Note the distinction this story had to learn the hard way: the **PIN** (`kernel-core-baseline.toml`, anti-DRIFT) and the **CEILING** (`kloc.toml`, anti-GROWTH) are different instruments, and D13(a) was open for 13 days precisely because one moved and the other did not."
kloc_grant: "⚠ **`xtask` IS THE BINDING CONSTRAINT AND NO GRANT IS REQUESTED ON AN ESTIMATE.** Measured at `5bcc3c76`: `xtask 39927 / 39960` = **33 lines of headroom**. AC1's register reader is a new `check_*.rs` and will not fit. `kloc.toml:60-65` forbids a grant on an estimate, so the lawful moves are (a) FUND BY RECLAIM — `cargo build -p xtask` emits **39 dead-code warnings** at HEAD, the D11-E1 class, and the `j1-crosshost-1a` precedent (delete `example_spirit_regen.rs`, 133 lines, remove the `mod` line FIRST so the compiler proves it) is directly on point; or (b) come back with a MEASURED grant once the gate is written. Do NOT open with a grant ask. NOTE: `xtask/tests/` is NOT charged (`kloc_check.rs:167-190` excludes `*/tests/*`), so every proven-red vector in AC1–AC3 is free; only `xtask/src/` costs. ⚠ `kloc-check` is **BLOCKING and RED at HEAD** on two keys — `maos-domain 8695 > 8644` (D14, owner 14-7) and `_aggregate_hardfail 151391 > 147057` (D17). Neither is this story's, and this story must NOT absorb either: D17's own ruling is that a grant requires a measured delta to justify it. **`all gates green` is therefore NOT available as a done criterion** — see AC5.4."
model: "frontier-class allowlist {opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, opus-5, equiv}. The literal token `allowlist {` is deliberate: `check_dev_model_used_populated.rs:302` uses it as the boilerplate guard, and without it `:337-344` would extract a model from this POLICY LIST and satisfy `check-dev-model-tier` VACUOUSLY with no dev ever recording what actually ran."
review: "§A6 full-layer net (Blind + Edge Case + Acceptance + Test-Infra + runtime) — **NON-DEGRADABLE**. Rationale specific to this row: every AC here is a CONTROL over other controls, and the failure mode this story exists to close is *a control that reports green over nothing*. A review that reads the diff and not the executed gate output would reproduce that exact defect. The Test-Infra Auditor layer is the load-bearing one and may not be dropped."
---

# 14-0 — Epic 14 preflight decisions

Status: **ready-for-dev** (2026-08-26). Opens Epic 14. Preflight complete: six parallel scouts,
one round-table, **one founder ruling already discharged and landed** (D13(a), commit `5bcc3c76`).

> ### This story did not survive its own charter, and that is the finding
>
> `14-0` was chartered as a **decision queue** — a place to record *that a decision is required, who
> owns it, and by when* — on the explicit premise that "recording a target does not decide the
> substance." The preflight measured that premise and it does not hold. Three things killed it:
>
> 1. **No machine reads the register.** Grep for `epic-14-preflight-decisions` across `*.rs` /
>    `*.yml` / `*.toml` → **two hits, both prose comments** (`xtask/kloc.toml:208`,
>    `xtask/src/gate_common.rs:12`). The residual gate hard-codes `dir.join("deferred-work.md")`
>    (`check_dev_record_completeness.rs:537`) and walks only `_bmad-output/implementation-artifacts/`.
>    The register lives in `planning-artifacts/epics/`, **outside every gate's `STORY_DIR`**
>    (`gate_common.rs:37`). Binding rule 2 promises *"did we miss it is a query, not a judgement."*
>    There is no query and no queryer. **Every deadline in the register is judgement.**
> 2. **Eight of nineteen rows are already wrong at HEAD** — describing repaired code, citing dead
>    lines, carrying falsified arithmetic, or marked RESOLVED over unimplemented substance.
> 3. **The founding defect reproduced itself one level down.** The register exists because twelve
>    residuals "pointed at `epic-14`, which is an epic key, not a vehicle." Seven rows now point at
>    *"`14-0` decomposes into a named story"* — a decision vehicle with a TBD target — and **D1's
>    implementation goes to retro C3, D11's to retro C5**, both table rows in a retrospective with no
>    key, no file, and no owner the tracker can page. The register's own epigraph is *"owned by a
>    retrospective is not an owner."*
>
> **A decision queue that nothing reads is a wish list.** So `14-0` is not code-free. The precedent
> was already set and it is an ugly one: **D19 was decided under `14-0` and shipped ~200 lines of
> `gate_common.rs` inside `j1-crosshost-2c`**, because `14-0` did not exist to hold it. Right
> outcome, wrong vehicle — and direct evidence that "`14-0` names a story" targets do not bind.

---

## Blocking conditions

1. **`xtask` has 33 lines of headroom.** See `kloc_grant`. Fund AC1 by reclaim or return with a
   measurement. Do not open with a grant ask.
2. **`kloc-check` is RED at HEAD on two keys that are not this story's** (D14, D17). `all gates
   green` is not an available done criterion; AC5.4 states the honest form.
3. **Do not "fix" a row by re-pointing it at another non-vehicle.** Every target this story writes
   must resolve to a `development_status` **story** key — not an epic key, not a retro action, not
   "a story that will be named later." AC2 makes that mechanical so the rule cannot decay again.
4. **D16 is a race. Measure it more than once.** Two scouts and the orchestrator got 3/20, 6/20 and
   5/5-pass on the same source. See AC3.2 for the settled number and the reason the readings
   diverged — a dev who runs it once and sees green will draw the wrong conclusion.

---

## Measured grounding (2026-08-26, at `5bcc3c76`)

Claimed-vs-actual for every number the register hard-codes. Binding rule 2 and retro C1 both say a
mechanical deadline queried against stale numbers is not a query; **the register is violating its own
rule at eight numbers**, and D11-E1 already corrected this exact set once, on 2026-08-14, for this
exact reason.

| Register claim | Measured at HEAD | |
|---|---|---|
| `EXPECTED_GATES` 36/67 → 37/68 | **38 / 69** | re-drifted |
| kernel-core 18933 / 18248 (+685) | **18933 / 18933** | ✅ DISCHARGED — D13(a), `5bcc3c76` |
| maos-domain 8694 / 8644 (+50) | **8695 / 8644 (+51)** | +1 |
| maos-bin 16211/16178 → 16219 (+41) | **16870 / 16870, GREEN** | superseded ×4 |
| aggregate 147549 / 147057 (**+492**) | **151391 / 147057 (+4334)** | **~9× understated** |
| `af788c3e` "net +878 across 34 files" | **+741 over 36 file-rows** | does not reproduce |
| `2688c6d0` "a further +52" | **+52** (+59/−7) | reproduces — register right |
| dead-code "~12 further sites" | **20 sites** (39 warnings) | +8 |
| `Mailbox::*` rows "six + one" | **5** | −2 |
| `example_spirit_regen` 133 lines | **133**, file now deleted | exact, then closed by 1a |

**Ten citations have rotted** (file exists, cited lines now hold something else): D2's sole evidence
`check_reza_production_path.rs:474-545`; D7's `main.rs:3075-3080` (real site now `:2357-2362`); five
D18 citations displaced by 2b's `router.rs` growth (`map_a2a_error_to_iac_bus` is now `:1902-2035`);
`DelegationLeg::delegate` (now `:267`); D15's `kloc.toml:264` value and its `maos-a2a-core 4654/4654`
precedent (now 4785).

---

## Acceptance Criteria

### AC1 — The register gets a reader. Binding rule 2 becomes true instead of asserted.

**AC1.1** A gate parses `_bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md` and
resolves each row's **Target story** cell against `gate_common::governed_story_keys()`. A target that
is an `epic-*` key, a retro action (`C3`, `C5`), or a phrase rather than a key is a finding.

**AC1.2** An **OPEN row whose deadline has passed REDS.** Deadline resolution is mechanical and
reads `sprint-status.yaml`: *"before X leaves `backlog`"* has passed when X's status is not
`backlog`; *"before X reaches `done`"* has passed when X is `done`. This is the whole of binding rule
2 — the gate never judges whether a decision was *wise*, only whether the tracker says its moment has
gone by while the row is still open.

**AC1.3** **FAILS CLOSED — non-negotiable, and Murat's condition at the round-table.** An unreadable
register, an unparseable table, or a resolution yielding **zero rows** is an `Err`, never a pass. A
gate that governs nothing passes for the wrong reason, and `findings.is_empty()` is blind to it. The
exact precedent and wording to mirror is `gate_common.rs:72-79` (D19's own fix).

**AC1.4** Deadlines that are **NON-MECHANICAL by construction** are declared as such and are not
silently treated as satisfied. Three exist today: D3 *"before any Epic 14 kernel-core edit"* (a code
event), D17 *"before the v2.2 wave closes"* (no such transition; its alternate target
`epic-14-retrospective` is status `optional`), D18 *"before `j1-crosshost-2b` writes its first
line"*. These must be visible as UNQUERYABLE, not counted as green.

**AC1.5 — proven-red, and it is about the DEFECT not the file.** A planted expired-and-open row reds
a Blocking gate. A planted unresolvable target reds. A planted `epic-*` target reds. A complete,
current register is **GREEN**, so the reds are not vacuous. An empty or unparseable register **REDS
rather than governing nothing**. Vectors live in `xtask/tests/` (not charged).

> **Why this AC is first.** Every finding in this preflight — D18 blown four stories ago under a
> RESOLVED tag, D2 authored four days after its subject was repaired, D6 open 24 days after the fix
> landed, D8 printing an owner the register retired — would have been caught mechanically by AC1 on
> the day it happened. It is the only AC here that prevents the *next* preflight from finding the
> same thing.

### AC2 — `epic-*` is rejected as an owner. The founding defect becomes mechanically detectable.

**AC2.1** `check_dev_record_completeness.rs` `classify_owner_assertions` (`:181-190`) must reject an
`epic-*` key as a deferred-row owner. Today `Owner: epic-14` buckets **`Ok`** — because `epic-14:
backlog` *is* a real `development_status` key, and `owner_tokens` (`:212-239`) rejects it only as a
token before the backtick fallback at `:151-160` finds it in the key map and green-lights it.
**The gate the register cites as its own verification is structurally blind to the register's
founding defect.** `story_key_from_filename` (`:243-250`) already rejects `epic-*` filenames; this is
the same rule, one field over.

**AC2.2** The rows this reds must be **re-homed in the same change**, not suppressed:
`deferred-work.md:704` (`CrossWallRecallRefusal` = **D7** → `14-4`) and `:786`
(`MAOS_REGION_HOME` = **D4** → see AC4.4). The register re-homed both months ago; the residual
register never got the memo, and the two files have disagreed ever since.

**AC2.3** The **Ownerless bucket must be emitted.** Today it is computed and reported nowhere
(`:161-163` `continue`s on rows with no owner cue), so five rows are structurally invisible to the
sweep — including **two of D5's four items** (`deferred-work.md:545`, `:546`), which are therefore
unowned *and* unpageable.

**AC2.4 — proven-red.** A planted `Owner: epic-N` reds. A planted ownerless row reds. A row owned by
a real story key is green.

### AC3 — The one-line repairs, each landing with the control that would have caught it

**AC3.1 — D13(a) residue: one sprint key, three holes.** Add a `development_status` key for
`spec-epic-5-review-finding-closure`. That single line (a) makes its deadline observable — it has
none today because a status no tracker records cannot transition; (b) pulls its story file under
`governed_story_keys()`, which currently exempts it from all five converted story-file gates; and (c)
un-blinds the three gates its **own closure list at `:79-80` already invokes** and which structurally
cannot see it. **Also add `kloc-check` to that closure list** — D13's own prescription, still undone
(`grep -in kloc` on that file → zero hits at HEAD). It is the hole that let the pin and the ceiling
disagree for 13 days.

**AC3.2 — D16: the lock AND the leg. The leg is the deliverable.**
- The lock is **one line** at `cross_team_consent_13_3.rs:497` — `LIVE_LOCK` taken by
  `cross_wall_traceback_refuses_without_cohort_preconditions` (`:496`), which sets `MAOS_HOME` at
  `:504` unguarded. Measured after: binary 25/25, whole package **0 failures across 37 binaries**.
- ⚠ **D16's stated cause is wrong and must not be chased.** Integration-test files are separate
  processes; `std::env` is per-process, so the three files it names **cannot race each other**. The
  inconsistency is intra-file (`:247` locked vs `:504` unlocked). Both accused siblings are innocent:
  `cross_team_crossing_13_6b.rs:2726` holds `LIVE_LOCK` at `:2358` *and* is `#[ignore]`d at `:2354`;
  all four callers of `seed_remote_artifact` hold `env_lock()`. It also misses a fourth file
  (`host_grants_2b.rs`), and `git blame` puts both sites in `b400d127` = **13.6d**, not 13.3.
- ⚠ **The rate.** Settled at clean HEAD over 20 runs of the compiled binary: **3 PASS / 17 FAIL**.
  Not 5/5 as filed. One scout read 5/5-*pass* — while another scout had its one-line fix applied to
  the shared tree. **Run it twenty times, not once.**
- **THE ACTUAL DELIVERABLE IS THE CONTROL.** CI is green today and **cannot go red**: no job runs
  `cargo test -p maos-bin` whole-package, `--workspace` appears only under `cargo build`, and
  **23 of 35** `maos-bin` test files are referenced in no workflow at all. The one enrolled leg
  (`check_multi_tenant_loom.rs:1333`, `BindingClass::Blocking`) runs `--exact`, so libtest never has
  a second test in the process and the race is **structurally unreachable by the only thing watching
  it**. Add **one `TestLeg`** running `-p maos-bin` whole-package under default parallel flags. It is
  green with the lock. That converts per-file guard discipline from prose into an executed assertion.
- Ship the lock without the leg and CI stays exactly as green as it was while broken.

**AC3.3 — D12: repair or retire, and say which.** `crates/maos-bench/benches/audit_query_latency.rs`
`:24` and `:235` emit `"capability.invoke"`; the accepted kind is `"capability.invocation"`
(`maos-audit/src/lib.rs:716`). Broken since 9.1, run **zero** times. It is declared only in
`maos-bench/Cargo.toml:89` and appears in **no workflow and no xtask**, so fixing the string alone
leaves a bench that still never runs — a D11(b) instance in its own right. Either give it an
execution path or retire it; a repaired-but-unreachable bench is not a closure.

### AC4 — The rulings. Recorded decisions with named evidence; no invented mechanisms.

Each sub-item closes **by decision** against its ID (binding rule 1), including the ones whose
outcome is "no change" (binding rule 4 — silence is not an outcome).

**AC4.1 — D3: ZERO kernel-Δ, and re-scoped from an instance to a class.** The FLAG-Winston ruling
D3 asks for is **granted as: repair out-of-kernel, class-wide.**
- The premise that this needs a kernel decision is near-vacuous: the signature is *already*
  `Result<(), CapError>`, so the propagate repair is **+1 kernel-core line, +3 `maos-domain`, and
  ZERO broken call sites** — all five production sites already type-handle it (3 via `?`, 2 via
  `let _ =`).
- But `record_drop()` lives in **`maos-capability`** (954 lines headroom, outside
  `check-kernel-baseline`'s scope and outside `kernel-crates.toml`). Making it observable or
  enforcing there costs **zero kernel-core lines, zero re-pin**, and repairs **all seven** drop sites
  instead of the one D3 names. In-repo precedent is exact: `maos-telemetry`'s `OtelSink::drop_count()`
  **is** read by a gate (`otel_gates.rs:818`).
- **Three facts D3 omits, all verified, all part of the ruling:**
  (a) `audit_drop_count()` has **four references repo-wide** — its definition, its own unit test
  twice, and one integration test. **Zero production readers.** Any ratification resting on "at least
  it is counted" is a claim standing in for a control.
  (b) **`cap_tokens/mod.rs:280-282` is the same defect UNCOUNTED**, on the operator revoke path:
  `let _ = self.audit.try_send(Revoke{..}); Ok(())`, no `record_drop()`. An Epic-1b reviewer patched
  `issue()` and `revoke_all()` for precisely this and **missed `revoke()`**; it has survived to HEAD.
  (c) Writer-task death puts the process in **silent no-audit mode for its lifetime** — the I2 panic
  at `transparency_log.rs:1364-1406` runs inside a spawned tokio task and surfaces only as an
  `eprintln!` at drain (`main.rs:8204-8208`), exit code unchanged.
- **Constraint on any repair:** `cap_audit_backpressure.rs:121-124` asserts `drops > 0`. A
  "never drop" repair **reds a shipped test**. Rule on that test in the same breath.

**AC4.2 — D5: decomposed into three named story keys; the kernel half rides AC4.1's ruling.**
All four items verified INTACT at HEAD, and the item this preflight's own brief could not find
**does exist** — `deferred-work.md:546`, *"`decommission_region_key` hardcodes `completed: true`"*.
`:549` is **not** a D5 item (retro §4 dispositions it *accepted risk*, `RELEASE-HOLDS.md:49`).
- **`14-e1-erasure-attestation-honesty`** — folds D5.2 + D5.3 under one control: *no signed erasure
  attestation may assert a completion it did not observe.* ZERO kernel-Δ. `CategoryStatus::CoverageGap`
  **already exists** (`erasure/proof.rs:22-27`), so the row's claim that this "reopens the AC1
  vocabulary decision" is false. Crates `maos-bin` + `maos-audit`, **both at zero headroom** → one
  measured grant under `kloc.toml:87`. **Scope IN.**
- **`14-e2-legal-hold-erase-serialization`** — D5.1 alone. KERNEL-TOUCHING. **Routed through AC4.1's
  ruling, not a second escalation**: same crate, same ZERO-Δ fence, same moment. Two rulings on one
  crate in one afternoon is the single-source defect this project has paid for three times.
- **`14-e3-mutation-to-audit-reconciliation`** — D5.4 alone; largest (≥4 crates, a persisted intent
  record and a startup reader). Same design as **ADR-059 residual #4**, also open and ownerless — scope
  them together or neither. **DEFER, with a named key** (`v25-erasure-crash-reconciliation`), because
  "defer to v2.5" has exactly one existing key today (`v25-signed-transparency-log-artifact-identity`)
  and it is not a home for this. Minting a phase label instead of a key is the move this register
  exists to stop. D5.4 currently has **two** open homes (`deferred-work.md:547` and ADR-059 #7) —
  close both or they drift.

**AC4.3 — D6: CLOSED by decision, on measurement. The fork it poses is moot.**
All three named defects were **repaired 2026-08-02** by `608facde`, titled *"fix(ci): clear
discipline gate blockers"* — a title naming none of it (D13's own warning that titles are unreliable,
firing again). The Vec-collect fix the row itself prescribed is at `private.rs:281-294`; `io_lock`
now spans all four entry points (`:639/:687/:746/:813`); traversal is `statat(SYMLINK_NOFOLLOW)` +
descriptor-anchored `open_dir_component` + `unlinkat`. Its first item (`:553`) was already annotated
CLOSED by 13.5i. **D6 names one closed row and three repaired ones**, open for 24 days.
Required: annotate `deferred-work.md:557-559` with the citations; **correct ADR-059 residual #9**,
whose stated mechanism is inverted at HEAD (it asserts in-memory removal precedes the first fallible
FS op; `forget_principal_unix` walks the filesystem first at `:847-905` and removes from the map last
at `:910`); and re-file the surviving general non-atomicity, **which is the live private-tier
residual D6 never mentioned.**

**AC4.4 — D4: split three ways; neither named vehicle can host the enforcement half.**
- ⚠ **Both halves of D4's rationale sentence are false.** *"The final harness derives region honestly
  from the signed entry"* — it does not: `cross_team_crossing_13_6b.rs:2138-2145` is a hard-coded
  `match team { "team-a" => "region-a", … }`, and **both** the `TeamEntry` (`:2170`) and the daemon
  env (`:2267`) read that same literal. They agree by shared constant, not derivation. That false
  claim propagated verbatim through four artifacts, each citing the previous. And *"production does
  not"* is also false — `cross_team_consent.rs:126-145` derives team keys from the **verified
  manifest** region. Production has both an honest path and an unreconciled one, **and they can
  disagree in the same process.** Correct all four artifacts before the row closes.
- **D4a — enforcement (runtime).** Constructible, and cheaper than feared: `main.rs:9855`
  `reconcile_transport_identity_with_manifest` already reconciles four env-vs-signed axes including
  `team_id`. Region is the omitted field of an existing check — ~20 lines mirroring
  `cross_team_crossing.rs:892-916`. **14-7 and 14-8 are both static-scan stories and cannot host a
  runtime boot check**, so D4's target *and its own stated fallback* are both wrong. **14-0 names a
  story** (D5 pattern). Must state that a daemon-boot leg does **not** cover `maosctl`.
- **D4b — registration.** Stays **14-8**, correctly sequenced behind 14-7. Today the gate cannot even
  see the variable: `check_env_contract.rs:119` scans only `crates/maos-bin/src`, and both primitive
  reads are in `maos-kernel-core` and `maos-domain`. `MAOS_REGION_HOME` is absent from
  `env_contract.rs`'s 67 entries.
- **D4c — classification + crypto. Security-relevant, and there is no slot for it.**
  `sealed_export.rs:303-315` derives the signing key by HKDF over the region tag, and
  `derive_team_signing_seed` welds the per-team key over that seed — so an unregistered env var
  **silently selects which Ed25519 key signs your audit bundle**. It is undetectable at every
  verifier: `resolve_verify_key` derives the expected key from *the bundle's own claimed region*, so
  a wrong-but-self-consistent region **verifies GREEN**, and `key_a == key_b` never fires.
  `EnvStability::Secret` is the **wrong slot** — the value is not secret and must stay echoable
  (`operator_config.rs:227` prints it); classifying it Secret would red an existing correct
  `eprintln!` under 14-9's own gate. **Decide whether a slot for "non-secret value, key-derivation
  input, integrity-critical" is added, or state the boundary.** Also repair `ADR-047:51`, which tells
  operators the TL signing seed lives in `MAOS_REGION_HOME` (it is `audit-signing.key` /
  `MAOS_AUDIT_KEY`, `audit_key.rs:27-43`) — the likely origin of the whole confusion; and
  `RELEASE-HOLDS.md` row 9, whose Residual column reads *"None — … not debt"* while live code defers
  to it.

**AC4.5 — D1: restated to the subject set that actually has the defect.**
- ⚠ **D1's second clause is close to inverted.** `check_vetting_attestation.rs:220-231` and
  `check_wasm_form_equiv.rs:244-256` **hard-fail** on any unmeasured leg, and `gate_common.rs:244`
  names the former *the reference implementation* of the vacuity guard. They are the **strictest**
  gates in the repo. D1 asks to add ABSENT vocabulary to two hermetic gates with no substrate to be
  absent. (Both do carry a `ran: bool` proxy the row does not mention, so "cannot represent ABSENT at
  all" overstates by one field.)
- **The boundary D1 fears expanding is principled and measured**, not arbitrary: `discipline.yml` has
  exactly **four `services:` blocks across 158 job keys**, owned by exactly the four ledger gates.
  **Ledger membership == CI substrate provisioning.**
- **The real subject set is seven gates that report `passed: true` over absent evidence** and D1
  names none of them: `check_pentest_gate.rs:79`, `check_red_team_gate.rs:127`,
  `check_third_party_trial.rs:202`, `check_cna_registration.rs:126`, `check_cross_form_equiv.rs:183`,
  `check_escape_detector.rs:691`, `check_migration_merkle.rs:276-281`.
- **Prior art already ships out-of-ledger** — `check_rto_gate.rs:77-84` (*"No evidence — SKIPPED (not
  a silent PASS)"*) and `check_migration_merkle.rs:169-193`. Ratify that pattern as the separate
  evidence contract; do not expand the ledger.
- The Family-A/B vocabulary is **not** contradictory (a premise this preflight raised and then
  disproved): families are defined by leg-struct shape (`13-6e…md:85-87`) — 2 A + 10 B = 12, 13.6e
  migrated 2 B into the A shape, ledger = 4, outside = 8. Both usages are arithmetically consistent.

**AC4.6 — D2: CLOSED by decision; the real residual is elsewhere and is worse.**
- The named defect was closed by **13.6e, commit `c45df0be`, 2026-08-07**. The stale-owner sweep ran
  **2026-08-08**, walked that exact file, annotated the rows on either side, and **skipped this one**.
  The stale row then propagated verbatim → retro C3 → `sprint-status.yaml:367` → **D2**. *D2 was
  authored describing a code site that had not existed for four days.* Annotate
  `deferred-work.md:548`. This is a **rule-1 violation in the inverse direction** — not closed by
  implication, but left open by omission after being deliberately addressed.
- Ratify the surviving `passed` / `product_claim` split as the explicit claim boundary. It already is
  one: `evidence_ledger.rs:1538` emits `"passed": blockers.is_empty()` while the same run emits
  `product_claim: NOT_PROVEN`, per-leg `ABSENT`, and a WOULD-HAVE-BLOCKED banner.
- **THE REAL RESIDUAL — E12-B1 is six of eight, and it is recorded `done`.** B1's ratified text was
  *"decouple blocking-disposition from `CURRENT_PHASE`."* Eight gates still carry a private
  `const CURRENT_PHASE`; six also adopted `BindingClass`, so theirs is vestigial. **Two adopted
  nothing** — `check_escape_detector.rs` (`CURRENT_PHASE` `:62`, private `is_blocking_at` `:94`) and
  `check_cohort_mesh.rs`. **Observed, not derived:**
  ```
  $ cargo run -p xtask -- check-escape-detector   → exit 0
  ::warning::Escape-detector oracle RED — would block ship at v2.0
  ```
  A red oracle exiting zero with `"passed": true` (`:691`). Its own vacuity guard cannot catch it:
  on a seccomp-blocked host the legs return `failed=1, green=false`, so `passed==0 && failed==0` is
  false, and the advisory tail converts RED into `passed: true`. **The fix for gate-binding decay
  decayed the same way** — and Epic 13's retro recorded B1–B6 as 6/6 done, noting its tracking had
  erred *pessimistic*. Here it erred optimistic, which is the direction that hurts.

**AC4.7 — deadline re-anchoring.** D5 and D6 are anchored to *"before `14-4` leaves `backlog`"*.
**14-4 is not a coherent anchor**: nothing blocks it, its only dependency (11-7) is `done`, and
14.1–14.6 are "largely parallelizable" — so 14-4 can leave `backlog` **before** 14-1, firing D5/D6's
deadline earlier than D1–D4's and inverting the register's own tiering. If 14-4 is never picked up,
they never bind at all. **Re-anchor D5, D6 and D16 to "before `14-1` leaves `backlog`"** — 14-0
already blocks 14-1, so the anchor is enforceable by construction. 14-4 remains correct for D7 and
D12, which are causally its.

### AC5 — Reopen what was falsely closed, and state the epic's honest opening posture

**AC5.1 — D18 is the most dangerous row in the register.** Marked **RESOLVED**; substance
**unimplemented**. `router.rs:2021-2030` still collapses both `ConsentUnclassified` variants into a
stringly `CrossHostRouteFailure`, discarding the typed `UnclassifiedReason`; `grep Unclassified
crates/maos-domain/src/iac_bus_types.rs` → **zero hits**, so the promised variant was never added.
Its re-pinned deadline — *"before `j1-crosshost-2b` writes its first line"* — **blew four stories
ago**: 2b, 2c, 2d and 2e are all `done`, while `14-4` sits `backlog`. The register has no
expired-vs-resolved distinction, so a reader skimming the RESOLVED tag cannot see it. **Reopen it, or
state the boundary per binding rule 4.** AC1.2 makes this class self-detecting from here on.

**AC5.2 — D17 is arithmetically falsified.** Its load-bearing claim is that the breach *"is
arithmetic downstream of D13 (+685), D14 (+50) and D15 (+41) — not a fourth independent overrun."*
At HEAD the aggregate is **151391 / 147057 = +4334**, of which D13+D14 account for **736**; the
remaining **83% is growth in crates now green under re-based ceilings**. The claim must be withdrawn
or re-derived. Note also that D17's implied prohibition on re-basing has been overtaken: `kloc.toml:61`
permits recalculation *"under an explicitly authorized measured grant"*, and that door has been used
repeatedly since — including by this story's own D13(a) predecessor.

**AC5.3 — D10 was overtaken by events.** D10 forbids *"a third unscoped grant by implication."*
Since it was filed, **two more landed**: `j1-crosshost-2b` 4654→4669 and `j1-crosshost-2c`
4669→**4785** (`kloc.toml:252`), each self-authorizing via `kloc.toml:87`. Either ratify the escape
valve or state the boundary; the row as written is moot. Also small but real: **D8's own gate output
still prints `OWNER: Epic-13 retrospective`** — the register re-homed D8 to 14-3 and never told the
gate — and D8 is RED and exits 0.

**AC5.4 — the epic's honest opening posture, stated not implied.** `kloc-check` is Blocking and
**RED at HEAD** on `maos-domain` (D14 → 14-7) and `_aggregate_hardfail` (D17). D13(a) is discharged
and kernel-core is green at 18933/18933 with **zero headroom** — deliberately, so every further
kernel-core line still costs its own measured FLAG-Winston grant and a HISTORY row. **`all gates
green` is NOT an available done criterion for this story**, and a dev must not manufacture one by
absorbing another row's debt. A standing red with named debtors is an honest state.

---

## Dev notes

- **Do not chase D16's stated cause, D1's stated subject, D2's stated site, D6's stated defect, or
  D4's stated rationale.** Each was measured and each is wrong; the corrections are in the ACs. This
  story's single most repeated finding is that *a row can be true when written and false when read*,
  and the register has no mechanism that notices.
- **AC1 before AC5.** AC5 reopens rows by hand; AC1 is what stops the next set from going unnoticed.
  If the story is cut short, AC1 + AC2 are the two that compound.
- **Fund `xtask` by reclaim, and prove it the way 1a did** — remove the `mod` line first and let the
  compiler prove there are no callers. 39 dead-code warnings at HEAD, including two inside D19's own
  fix (`gate_common.rs:37 STORY_DIR`, referenced only from a doc comment) and two in
  `check_j1_two_host_signed_run.rs` (`:201 leg_green`, `:1230 GATE`).
- **`xtask/tests/` is free; `xtask/src/` is charged.** Put every proven-red vector in `tests/`. Note
  the related open finding: **8 `check_*.rs` live in `xtask/src/tests/`**, are budget-charged, and are
  invisible to the top-level gate census — D11-E3's category, and part of why `EXPECTED_GATES`
  arithmetic keeps drifting.
- **The third blind direction is documented as intentional and is uncompensated.**
  `gate_common.rs:131-134` governs only files whose stem is a sprint key; its doc at `:127-129` says
  *"a design note that is not a sprint key is equally ignored."* There are **37 orphan `.md` files**
  in `implementation-artifacts/`, two carrying non-terminal story frontmatter and governed by
  nothing. AC3.1 fixes the one that matters; the class is worth a finding. A fourth gap sits in the
  same helper: the inverse check exempts `done` as well as `backlog` (`:94-99`), so a `done` key
  whose file vanishes reds nothing.
- **Two conventions for one field.** `deferred-work.md` uses full story keys (`:641`, `:752`, `:791`);
  the register uses short forms (`14-4`, `14-8`). They resolve by prefix today
  (`check_dev_record_completeness.rs:173-177`) but AC1 should not depend on that accident.

## Decision ledger — disposition of all nineteen rows

| Row | Disposition under this story |
|---|---|
| D1 | **RESTATE** (AC4.5) — real subject set is 7 gates; ratify the `check_rto_gate` pattern, do not expand the ledger. Its C3 target is not a vehicle. |
| D2 | **CLOSE by decision** (AC4.6) — closed 2026-08-07 by `c45df0be`; annotate `:548`. Real residual = E12-B1 at 6/8. |
| D3 | **RULED** (AC4.1) — ZERO kernel-Δ, out-of-kernel, class of 7 (8 with the uncounted `revoke()`). |
| D4 | **SPLIT** (AC4.4) — D4a new story · D4b 14-8 · D4c 14-9 with a new slot or a stated boundary. |
| D5 | **DECOMPOSED** (AC4.2) — `14-e1` scope-in · `14-e2` rides D3's ruling · `14-e3` defer to a named v2.5 key. |
| D6 | **CLOSE by decision on measurement** (AC4.3) — repaired 2026-08-02; correct ADR-059 #9; re-file the live residual. |
| D7 | 14-4, unchanged. Re-home its `Owner: epic-14` (AC2.2); citation rotted to `main.rs:2357-2362`. |
| D8 | 14-3, unchanged. RED and exits 0; gate string still names the retro (AC5.3). |
| D9 | 14-3, unchanged. |
| D10 | **MOOT — ratify or bound** (AC5.3); two further grants landed while it waited. |
| D11 | 14-6, unchanged. Numbers re-based to **38/69**; E1 closed by 1a; E2 and E3 open. Its C5 target is not a vehicle. |
| D12 | **REPAIR OR RETIRE, and say which** (AC3.3). |
| D13 | **(a) RESOLVED `5bcc3c76`** — founder grant, 18248→18933 exact/zero-headroom. **(b) OPEN**, 14-6. |
| D14 | 14-7, unchanged. Re-measured **+51**. Re-anchor per AC4.7. |
| D15 | Closed in time; recorded value superseded ×4 (now 16870/16870). |
| D16 | **RESTATE + lock + leg** (AC3.2). Cause, count, blame and rate all corrected. |
| D17 | **FALSIFIED — withdraw or re-derive** (AC5.2). +4334, not +492. |
| D18 | **REOPEN or bound** (AC5.1). Substance unimplemented; deadline blew four stories ago. |
| D19 | Resolved and verified green — the best row in the file. Its contract was still not followed (shipped inside `2c`), which is why AC1 exists. |
