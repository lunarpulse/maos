---
story_id: '20.3b'
story_key: 20-3b-gate-retirement-and-coverage-generator
created: 2026-10-30
context_commit: 23f58da4
baseline_commit: 23f58da4
security_sensitive: true
kernel_grant: "◆ FLAG-Winston REQUESTED, PROPOSED (D03 + D-20-3b-05) — NOT granted. Gate deletion and generation are kernel-Δ 0 by construction (xtask, workflows, registry, YAML, docs). The received FR31 repair is the only kernel-touching scope, priced by throwaway prototype on a `git archive 23f58da4` copy under `/tmp/proto203b` (never the tree; `creation/*.diff`), measured with the prebuilt `check-kernel-baseline --json` / `kloc-check --json` run from the copy root: K1 durability **+1/+1**; K2a identity-keyed spill dirs **+41/+33**; K2b K2a + principal-index owner column **+90/+79** and maos-domain **+1**; recommended R′ = K1 + K2b **+91 physical / +80 tokei** (26409 → 26500; 20618 → 20698), alternative R = K1 + K2a **+42/+34**. ADR-069 (`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46`): 20698 is 1302 under the 22000 alarm — the envelope is not headroom; the grant is the operator's."
kloc_grant: "PROPOSED (D02 A as ruled 2026-10-30: per-story measured envelope + explicit review-correction allowance; no E20-wide numeric grant) — D-20-3b-15. Measured at creation: retirement gross **xtask −5409** (GO; −5316 NO-GO), maos-bin −2, aggregate −5411 (prototype `creation/x1-*`); FR31 maos-bin **+9** (`creation/b1-*`); adds NOT prototyped, forecast xtask +321…+914 [INFERENCE, measured analogues]. Allowance ×3 on unmeasured adds and ×2 on measured adds (E19 landed at 2–3× forecast: `_bmad-output/implementation-artifacts/epic-19-retro-2026-10-30.md:40-44`). Inherited room at creation (live pins exact, zero headroom): maos-bin 71 (E19 STOP 33010 − 32939), xtask 6 (47688 − 47682), maos-cli 11 (6382 − 6371), maos-audit 0, aggregate 0 (hardfail 192779 vs measured 192778). Operator raises still needed: kernel FLAG-Winston (D03/D-05), maos-domain +1 (D-05 B), any aggregate growth landed before or apart from the deletion, and the maos-bin share of the 71 (contended by 20-1/20-4)."
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier (`proof-contract.md:67-70`): a kernel FLAG-Winston grant (D03) and a security-critical cross-Spirit isolation AC (FR31), plus the deletion of ten live or registered controls where one mistake removes protection (epic split rationale, E20@HEAD:103). Blind and Edge on opus-5.5, Acceptance on opus-5.5, Security reading on AC4, Test Infrastructure on glm-5.3 (standing policy `proof-contract.md:64-72`; any session substitution recorded at review). Creation anthropic/claude-opus-5-5."
---

# Story 20.3b: Gate retirement and the generated coverage matrix

Status: ready-for-dev

<!-- Created 2026-10-30 at 23f58da4 (= 3fd7f8b4 + the E20 preflight report) by bmad-create-story under the lighter measured creation contract, after the operator's 2026-10-30 rulings D01 A (ratify the preflight scope and amend the epic), D02 A (per-story measured envelopes, review-correction allowance, savings recognized only after deletion) and D11 A. D03, D04, D08 (preflight §5) and D-20-3b-01…16 are PROPOSED, not ruled — the operator rules; a different ruling rewrites only the clauses marked with that row. The epic is owned by EpicAmend this session: its corrections are filed in `20-3b-…-evidence/creation/records-owed.md` (E-rows), not applied here. No cargo ran (parent rule): cites re-read at HEAD; kloc and kernel by the prebuilt gate binary run on /tmp copies of HEAD with each prototype applied (`creation/measurements.md`); nothing in the tree was modified except records (this file, its evidence dir, the tracker row, four deferred-work annotations). -->

> **The capability:** *the gate set is what CI actually runs, and nothing else: ten dead, duplicated or unscheduled gates are gone with every enrolment, every behaviour that still protected something moved — not lost — into a live gate or a test; the coverage matrix is generated from one requirements-keyed source and blocks; and a Spirit admitted at a recycled pid in a later boot can neither read nor plant another Spirit's principal memory.*

**Closes:**
- Epic-20 §20-3b (split row E20@HEAD:64,103; pre-split ACs AC4/AC5/AC7, E20@HEAD:130-133) as amended 2026-10-30 (preflight §1.4, §3.2), renumbered AC1–AC3 here.
- NFR-Meta-3 — generated coverage report blocks uncovered requirements (`routing-audit-2026-10-22.md:133`).
- The received FR31 security item (E20@HEAD:278-280; `deferred-work.md:1198` confidentiality + `:1199` restore-poisoning, was `:1196-1197`/`:1252`) — AC4, under D03.
- Routed witnesses (`routing-audit-2026-10-22.md:45`): FR31 isolation, FR34 SDK coverage, FR53 halt identity across swap, FR63 catalog/retryability/cause-chain; the I1–I14 guaranteed/asserted ledger and per-invariant adversarial coverage (`:144-145`; PRD `success-criteria.md:38-39`) — AC5.
- `deferred-work.md:902-903` clippy half (deny half closed by 18-0, recorded at creation), `:910-911` (closed at creation, recorded), `:1223-1224` prepush mirror; 15-1's `pub use` group-hash filing (`15-1-green-at-head.md:352-355`); 15-3's 33 coverage-matrix violations; 16-3 §11 row 15 (NFR-Rel rows, now uncovered not mis-named); 17-4 D-17-4-PF-R2 debt; 15-5's `check-adr040-accepted` no-workflow filing (resolved by retirement).

**Does NOT close, and says so:**
- External-author validation of FR34 (FR:84) — operator/cohort-backed (`ops-external-cohort-and-pen-test`); this story lands the SDK harness witness only.
- Any repair a new witness exposes outside FR31: FR53 continuity defects go to their owner (`20-3c` member 12 swap-halt receipt, `deferred-work.md:894`; `20-9` spawned-form parity), FR63 catalog gaps to the gate's owner by rule 10 — the witness lands labelled with the owner, never weakened.
- FR13 quarantine isolation (20-1, D04); the shared-tier pid-keyed writer attribution (`memory/shared.rs:17-28`, confidentiality not affected — shared rows are Host-readable by design; [INFERENCE] attribution only, not routed here).
- Same-name identity squatting across boots under D-20-3b-06 A — labelled residual; closes with 20-1's publisher-key binding (E20 20-1 AC2).
- Perf metric semantics and the perf gates (20-7 owns `check-perf-*`; 3b owns only the coverage map and the gate set).

## Story

As **Murat (test architect) and the operator who reads CI**,
I want **the dead gates deleted with all their enrolments, the coverage matrix generated and blocking, and pid reuse unable to cross Spirit memory**,
so that **every green check is a control that ran, xtask has room for the remaining Ship-It work, and FR31's isolation claim is witnessed rather than asserted.**

**Contract:**
- Epic §20-3a/20-3b split (E20@HEAD:95-121) + pre-split AC4/AC5/AC7 (`:130-133`) + received §20-3b (`:278-280`), as amended 2026-10-30 (D01 A; preflight `epic-20-preflight-2026-10-30.md` §1.4, §2 row 20-3b, §3.2) — create-story may tighten, not add uncited ACs; AC4/AC5 are the received and routed items the preflight assigns to this key (§2).
- Tracker `sprint-status.yaml:299`; rules 9–11 (E20 confidence rules); proof contract `docs/testing/proof-contract.md:24-31` (one plant per AC), `:33-75` (review, tiers), `:150-170` (prepush).
- 20-3a done (`sprint-status.yaml:298`): gates are honest about evidence state, so retirement decisions read off them (E20@HEAD:105).

## Current-tree corrections — read before implementing

Full OLD → NEW table: `creation/cites-23f58da4.md` (C1–C18, F1–F10, T1–T2, D1–D5).

| Premise (epic / tracker) | Fact at `23f58da4` and consequence |
|---|---|
| "TEN retirements across SIX enrolment sites" | **The closure is wider** (C6): besides `main.rs` (`mod` `:11-83`, commands `:421-941`, dispatch `:1353-1494`), `EXPECTED_GATES` (`check_ship_gate_completeness.rs:17`, entry `:24`), the YAML (`:2004`), `aggregate.needs` (`discipline.yml:6394-6535`, **133** entries, not 123) and the registry (`gate-registry.toml:34,64,68,72`, `[[ship_gate]]` `:270`): `prepush.rs:25-28,42`; the job `smoke-discipline-7-1-5` (`discipline.yml:3718`) and its **maos-bin** arm (`crates/maos-bin/src/main.rs:9214-9216,13836-13887`); the aggregate summary script (`:6582-6588,6689-6695,6769-6772`); v1-0/v1-5 ship-gate needs and summaries (`:6254,6257,6313,6316,6359,6362,6830,6845`); xtask tests (`d19_story_file_governance.rs:157-386`, `story_10_2_proven_red.rs:784-1188`, `coverage_matrix_completeness_tests.rs`); docs (`proof-contract.md:156,159,167`; ADR-031 `:75`, ADR-040 `:5`, ADR-043 `:3,14-29`, ADR-045 `:60`, ADR-068 `:6`). → AC3 closure check, not a site count. |
| `check_dev_model_used_populated.rs` is deletable | **Two live gates import four of its helpers** (`check_dev_model_tier.rs:19-21`, `check_dev_record_completeness.rs:476-478`): `load_sibling_sprint_status`, `is_pre_development_status`, `agent_model_section_model`, `looks_like_model` (+ `PRE_DEVELOPMENT_STATUSES`, `KNOWN_MODELS`; module `:17-108,301-367`). They move; the prototype moved them into `check_dev_record_completeness.rs`. |
| "`check-cross-form-equiv` never scheduled" | False: job `discipline.yml:4059-4077`. But `docs/cross-form/results/cross-form-results.json` is absent (only the schema), so the job always takes its `::warning::` branch and the module never runs; registry row advisory/advisory (`gate-registry.toml:270-272`); binding equivalence is `check-wasm-form-equiv` (`check_wasm_form_equiv.rs:7`, ADR-031 `:75`). → D-20-3b-04. |
| Arithmetic: 6406 retired, net −4.5k…−5.2k | 6402 physical / **5499 tokei**; **measured gross −5409 xtask** with enrolment edits (GO), −5316 (NO-GO), maos-bin −2, aggregate −5411. Adds unmeasured. |
| AC5: generated YAML at `mode: error`; `coverage_matrix.rs:93-120` unchanged | Parser accepts `warning`/`hard` only (`coverage_matrix.rs:110-117`); `hard` enforces (`:65`). → **D08** (PROPOSED). `delivered_phase: v0.1-alpha` (`tests/coverage-matrix.yaml:2`) defers every row above v0.1-alpha — `hard` today would enforce only unknown names/phases, not coverage of v1.0 rows (the completeness gate does that for v1.0 NFRs). Spike question S7. |
| "absorbs 15-3's 33 violations" | **33 reproduced by replica** (`creation/coverage-matrix-replica.py`; the gate was not run here): 27 unknown-gate refs in 21 rows (`approval_log_is_distinct_table`, `audit_spine_integration`, `cap_registry_integration`, `cap_token_verify_assertion`, `check-kernel-baseline`, `coverage-matrix-nfr-test-3`, `fr4_1000_call_fixture`, `iac_transparency_log_tests`, `journal_append_p99_measurement`, `journal_survives_cold_restart`, `journey-hermetic-tier-1`, `nfr-rel-3-hsis-95pct`, `smoke-skill-7-4`), 3 unknown corpora (`hsis-corpus-v0`, `quarterly-audit-v0`, `crates/maos-kernel-core/tests/fixtures/manifest`), 3 phases outside `phase_order` (`NFR-Rel-7`/`NFR-Scale-2` `v2.2`, `NFR-Test-8` `v2.0`). → AC2.4, D-20-3b-03. |
| `check-deprecations-declared` "in no workflow" | True, but `prepush.rs:42` runs it and `proof-contract.md:159,167` names it a **CR-2 exception** — both retire with it; `stability-matrix --check` is already the prepush/CI leg (`prepush.rs:15`). |
| `check-adr040-accepted` retirement is free | ADR-068 `:6` says ADR-040's `Status:` line "is read by `check-adr-040-accepted`"; ADR-040 `:5` names the gate. Docs amended with the retirement. |
| `check-literal-reappearance` guards live literals | Its two literals (`:9`) occur in no production `src` (`git grep`: `crates/maos-bin/Cargo.toml:80,84` comments + one skipped test file); registered (`gate-registry.toml:64`), never scheduled. |
| `cargo deny` advisory (`deferred-work.md:902,909`) | **Deny half closed by 18-0**: blocking `cargo-deny` job `discipline.yml:157-189`, `aggregate.needs` `:6397`; lockfile rustls 0.23.46 / rustls-webpki 0.103.16 / chacha20 0.10.2 (`Cargo.lock:4541-4542,4574-4575,664-665`). Recorded at creation (`deferred-work.md:903,911`). Clippy half open: `components: rustfmt, clippy` `:81`, no invocation. → D-20-3b-11. |
| FR31 "+1/+1 durability, identity +20–50 [INFERENCE]" | **Measured**: K1 +1/+1, K2a +41/+33, K2b +90/+79 (+ domain +1). The principal index (`memory/principal.rs:16-26`) and the export path (`memory/mod.rs:921`) are pid-keyed too, and the GDPR forget walk skips any non-numeric spill dir (`private.rs:849`) — identity dirs need the walk changed or forget silently misses them. Principal-write enforcement is a pid set (`mod.rs:99,401-407,715-726`), so it does not stop the cross-boot case. → D03, D-20-3b-05/06. |
| Received cites (E20@HEAD:280) | `private.rs:264-279` → `:263-278` (pid component `:273`); `mod.rs:835-860` → read `:779-830`, scan `:832-860`; `deferred-work.md:1252` → `:1198-1199`; threshold `main.rs:2219-2222` → `:2154-2157`; shell note `:198-200` → `:197-199` (F1–F9). |
| NFR-Rel-1/2/11 "job ids absent from the registry" (16-3 §11 row 15) | Rows now `tests/coverage-matrix.yaml:1421,1431,1436`, `gates: []`, phase `v0.8` — uncovered, not mis-named; deferred by `delivered_phase`. → AC2. |
| `pub use` group-hash | Same (`check_service_boundary.rs:393-396,480-483`): every leaf of one `Item::Use` gets the whole item's hash (one added name → five removal alarms, `15-1-green-at-head.md:352-355`). → D-20-3b-13. |
| prepush mirrors the Blocking set | `prepush.rs:14-45` still lacks `check-epic-close-coherence` (`deferred-work.md:1223`); `error-catalog-check` resolves the root from runtime `CARGO_MANIFEST_DIR` (`check_error_catalog.rs:644-657`). → D-20-3b-12. |

## Kernel and kloc pricing — measured at creation

**Instrument:** prebuilt `/tmp/xtask-maos-w3` (`check-kernel-baseline --json`, `kloc-check --json`, tokei 14.0.0) run from a `git archive 23f58da4` copy root per prototype; the untouched copy reproduces the tree exactly (26409 / 95 files / set hash `66dc2e24…`; aggregate 192778). Detail and diffs: `creation/measurements.md`, `creation/tokei-23f58da4-and-prototypes.json`.

**Kernel (◆ FLAG-Winston row — PROPOSED, D03 + D-20-3b-05):**

| Shape | Kernel physical | Kernel tokei | Other | Diff |
|---|---:|---:|---|---|
| K1 Principal values always durable (`private.rs:644`) | +1 (26410) | +1 (20619) | — | `k1-principal-durability.diff` |
| K2a identity-keyed spill dirs + forget walk + adapter delegate | +41 (26450) | +33 (20651) | 2 new pub fns → class rows + surface baseline | `k2a-identity-keyed-spill-dirs.diff` |
| K2b K2a + index `writer_owner` column (+ guarded `ALTER`), export by owner | +90 (26499) | +79 (20697) | maos-domain +1 (`PrincipalIndexRow.writer_owner`) | `k2b-identity-index-owner-column.diff` |
| **R** = K1 + K2a (D-05 A) | **+42 (26451)** | **+34 (20652)** | — | `krec-k1-k2a-bundle.diff` |
| **R′** = K1 + K2b (D-05 B, recommended) | **+91 (26500)** | **+80 (20698)** | maos-domain +1 | `kfull-k1-k2b-bundle.diff` |

Both pins move with any grant (`xtask/kernel-core-baseline.toml:27-28` + EOF block; `kloc.toml:264`), plus content hashes for `memory/{private,mod,principal}.rs`, `docs/ci-baselines/kernel-surface-v0.1-beta.json` and `xtask/kernel-api-classes.toml` (`PrivateMemoryStore` is `data-movement`, `:440,456,481`). Not compiled ([INFERENCE]; measurement owed item 3).

**kloc** (tokei; HEAD = every pin, zero headroom):

| Crate · component | Measured / forecast | Basis |
|---|---:|---|
| xtask · ten retirements + enrolment edits + helper move (GO) | **−5409** (NO-GO −5316) | prototype `x1-*` |
| xtask · three folds | +115…+344 [INFERENCE] | retired non-test logic 72 / 335 / 87 |
| xtask · generator (on GO) | +200…+500 [INFERENCE] | `coverage_matrix.rs` 174 non-test + completeness 80 |
| xtask · group-hash, prepush pin, ledger | +6…+70 [INFERENCE] | `measurements.md` |
| **xtask net** | **−5088…−4495** (forecast) | — |
| maos-bin · retire smoke names (D-10 A) / delete arm (D-10 B) | **−2** / ≈ −46 (arm 43 tokei + dispatch) | prototype / tokei of `main.rs:13836-13887` |
| maos-bin · storage-identity binding at admission + shell (D03 A, D-06 A) | **+9** | prototype `b1-*` |
| maos-domain · `writer_owner` (D-05 B) | **+1** | prototype `k2b-*` |
| kernel | R′ +80 tokei / R +34 | above |
| maos-audit · governance round-trip as `crates/maos-audit/tests/*` (D-08 A) | 0 (tests uncharged, `kloc_check.rs:303,311`) | — |
| maos-cli, maos-control, maos-shell, spirits, tests | 0 | not touched |
| **aggregate** | ≈ −5411 + adds; worst case with allowance ≈ **−2485** | sum |

**Envelope (D-20-3b-15, PROPOSED; D02 A — allowance ×3 on unmeasured adds, ×2 on measured adds):** xtask landing ≤ **45015** (GO: 42273 measured after deletion + forecast adds ×3 upper 2742; NO-GO ≤ 43608); maos-bin ≤ **+16** (+9 ×2 − 2) drawn from the inherited 71 (E19 STOP `epics/epic-19-founder-loop-w4.md:40`; 20-1's +60…+120 and 20-4's +40…+80 contend for the same 71, preflight §1.3); maos-domain ≤ **+2** (+1 ×2; raise from live 0); kernel ≤ **+182 physical / +160 tokei** (R′ ×2) under the FLAG-Winston row; maos-audit / maos-cli **0**. Aggregate: L1 (deletion + xtask adds) only lowers it; under D-20-3b-01 A, L2's adds (≤ +178 tokei with allowance: kernel 160, bin 16, domain 2) need an exact aggregate grant against L1's lowered pin (D-20-3b-16 A); under D-20-3b-01 B they net against the deletion in one landing. Inherited and untouched by this story: maos-cli 11 (`epic-19…:45`), xtask STOP 6 (`:41`) — irrelevant once the deletion lands — maos-audit 0 (`:48`), aggregate 0 (`kloc.toml:718`). Exact formatted pins land with the driver (`kloc.toml:60-89`); lowering is unconditional.

## Decisions — PROPOSED (the operator rules; none is decided here)

Preflight rows that bear on this story stay as filed (`epic-20-preflight-2026-10-30.md:287-292`); their 20-3b consequence is added.

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D03** (preflight) | FR31 two-boot confidentiality/integrity | **A** include stable-identity + principal durability production repair with a measured FLAG row and the 3b witness · **B** separate named security prerequisite story before witness/rc · **C** label the invariant "asserted" | **A.** Measured now, not inferred: R′ +91/+80 (R +42/+34) — small and confined to `memory/`; not inseparable from a larger migration (the preflight's B trigger), so A holds. C leaves a known read and restore-poisoning path (`deferred-work.md:1198-1199`) behind a label. | PROPOSED |
| **D04** (preflight) | FR13 quarantine vs drain-and-marker (20-1) | **A** measure real isolation, grant necessary kernel change · **B** sufficient existing non-kernel isolation API with a hostile-effect witness · **C** ship drain+marker as quarantine | **A unless prototype proves B** (preflight). 20-3b consequence: AC5's ledger lists FR13 quarantine **asserted, owner 20-1** until 20-1 lands; under C the row must read "not isolated". Kernel seam: D03 A and D04 A both touch `maos-kernel-core` — serialize the pin updates (one integration owner), shared ADR-069 room 1382 tokei to alarm. | PROPOSED |
| **D08** (preflight) | Coverage enforcement token | **A** existing `mode: hard` as strict and correct the epic literal · **B** migrate parser/source/contracts to `error`, delete `hard` · **C** keep `warning` | **A.** `coverage_matrix.rs:65,110-117` already enforce `hard`; keeps the epic's "parser unchanged" (E20@HEAD:130) true; B costs a parser + fixture change for a spelling; C fails NFR-Meta-3 ("blocks"). | PROPOSED |
| **D-20-3b-01** | Landing shape for the ratified (D01 A) scope | **A** one key, two landings: L1 = deletion + generator + gate dispositions (AC1–AC3, AC6, AC7); L2 = FR31 repair + witnesses + ledger (AC4, AC5); `done` only after L2 · **B** one landing for all · **C** split AC4/AC5 to a new key | **A.** L1 frees ≈ 5.4k xtask the next E20 stories need (preflight §2 priority 1) without waiting on a FLAG-Winston grant and a DIFFICULT security review; E19's joint wave needed 4 review rounds / 13 agents (`19-4-…md:287-322`). C re-opens D01 A's ratified scope. Under A, L2's adds need an aggregate grant against L1's lowered pin (D-16 A). | PROPOSED |
| **D-20-3b-02** | Generator design the spike tests (AC1) | **A** epic's design: requirements-keyed `tests/coverage-source.toml` (phase/valid_until/notes/corpora per requirement) + `covers = [...]` per gate in `gate-registry.toml`; generator `gen-coverage-matrix [--check]` emits the YAML byte-identically · **B** `covers` in the registry only; YAML stays hand-kept for phase/notes, generator only checks the gate half · **C** NO-GO now: keep the hand-kept YAML, fix the 33 at origin, `mode: hard`, keep completeness | **A, decided by the spike's oracle.** NFR-Meta-3 wants a *generated* report (`routing-audit:133`); a per-gate `covers` cannot carry phase/notes (E20@HEAD:130 (1)–(2)). C is the measured NO-GO fallback (xtask −5316 vs −5409). `gen-abi-docs --check` (`prepush.rs:17`) is the existing generate/check precedent. | PROPOSED |
| **D-20-3b-03** | Phases outside `phase_order` (`NFR-Rel-7`, `NFR-Scale-2` = `v2.2`; `NFR-Test-8` = `v2.0`) | **A** add `v2.2` (and normalise `v2.0` → `v2.0+`) in both `tests/coverage-matrix.yaml` and `phase-config.toml` · **B** relabel the three rows `v2.0+`, keep the PRD phase in `notes` | **B.** `phase_order` must equal `phase-config.toml`'s (`coverage_matrix.rs:119-121`); A changes the phase vocabulary every phase consumer reads for three rows; `v2.0+` already orders after `v1.5`. A is right only if the operator wants v2.2 as a planning phase. | PROPOSED |
| **D-20-3b-04** | `check-cross-form-equiv` | **A** retire module, job, registry + ship rows, `EXPECTED_GATES`, YAML; amend ADR-031 `:75` to name `check-wasm-form-equiv` as the only equivalence gate · **B** keep it advisory (no retirement; −312 tokei less) · **C** retire the gate, keep the Mann-Whitney statistics as a test | **A.** The module never runs in CI (artifact absent, C8); D10 relabeled it non-binding; the equivalence-binding row is `check-wasm-form-equiv` (`gate-registry.toml:283-284`, advisory at v1.0/v1.5, blocking at v2.0). The ADR edit is the "preserve/ratify equivalence obligation" the preflight requires (§3.2). | PROPOSED |
| **D-20-3b-05** | FR31 kernel shape (if D03 A) | **A** R = K1 + K2a (+42/+34): identity spill dirs, durable principals; the principal index stays pid-keyed (cross-boot subject-access export reads the current boot's owner of that pid) · **B** R′ = K1 + K2b (+91/+80, domain +1): index records the owner; export reads by recorded owner | **B.** A fixes read/poison but leaves GDPR Art.15 export of earlier boots' rows resolving through a recycled pid (`mod.rs:921`) — incomplete or mis-attributed export, a record asserting something false (rule 11). B is +49/+46 more kernel and needs a maos-domain +1 raise. | PROPOSED |
| **D-20-3b-06** | Storage identity source (maos-bin) | **A** the admitted `spirit_id` (manifest class name) bound after `on_loaded(pid)` (`admission.rs:859`) and on the shell path (`main.rs:4363-4368`), +9 measured · **B** `spirit_id` + publisher-key fingerprint — needs 20-1's signer binding (no signer at admission today: `grep signer\|publisher admission.rs` = 0) · **C** operator-assigned install id persisted in the store | **A, with the same-name residual labelled to 20-1.** Closes the pid-reuse read/plant between differently named Spirits now; a later Spirit *named* `butler` still lands in Butler's tree — the trust boundary is admission (20-1 AC2 binds the publisher key). C adds an id registry for no closed attack today. Migration: pre-fix spills under `<pid>/` stay forgettable and purgeable but are no longer read by bound Spirits (Butler's acceptance ledger restarts empty once — disclosed in AC4.6). | PROPOSED |
| **D-20-3b-07** | Registered-but-unscheduled gates: `check-adr040-accepted`, `check-literal-reappearance`, `check-governance-categories`, `check-deprecations-declared` | **A** retire all four; keep only behaviour with an owner (governance round-trip → test, D-08; deprecations → existing `stability-matrix --check`); amend ADR-040 `:5`, ADR-045 `:60`, ADR-068 `:6`, `proof-contract.md:156-169` · **B** schedule them in CI instead (re-enrol; −429 tokei less) | **A.** None runs in CI (`.github` grep empty, C9–C12); the literal guard's literals exist in no production source (C10); ADR-040 is `accepted` and superseded by ADR-031 (`ADR-040…md:2-3`); B adds CI cost to re-assert closed facts. | PROPOSED |
| **D-20-3b-08** | Governance round-trip placement | **A** `crates/maos-audit/tests/governance_categories_round_trip.rs` (kloc-free) · **B** inline `#[cfg(test)]` beside `kind_category_to_kinds` (`maos-audit/src/lib.rs:832`), ≈ +20 maos-audit (live room 0 → raise) | **A.** Same assertion, no raise against a crate at 0 (`kloc.toml:409`). | PROPOSED |
| **D-20-3b-09** | The three migrations of the Epic-6 beacon | **A** fold: bare-placeholder check → `check-review-findings-resolved`; dev-model presence → `check-dev-record-completeness`; `if: false` scan → `check-epic-close-coherence`; preserve each negative witness · **B** retire without fold, relying on the successors' existing coverage | **A.** B has no measured proof the successors already catch all three; the d19 governance vector (`d19_story_file_governance.rs:351-386`) and ADR-043's successor map (`:14-29`) treat them as live controls. Forecast +115…+344. | PROPOSED |
| **D-20-3b-10** | `smoke-discipline-7-1-5` (job `discipline.yml:3715-3720`, arm `main.rs:13836-13887`) | **A** shrink to the two surviving gates (maos-bin −2) · **B** delete job + arm (≈ −46 maos-bin): both gates already run in their own aggregate-enrolled jobs | **A.** Story 7.1.5 AC5's smoke is a contract this story did not set out to retire; B is a cheap follow-on if the operator wants the bytes. | PROPOSED |
| **D-20-3b-11** | Clippy (`deferred-work.md:902-903`) | **A** blocking `cargo clippy --workspace --all-targets --locked -- -D warnings` job in `aggregate.needs`, with T0 count first (STOP and re-ask if > 0) · **B** advisory job labelled with owner + reason (rule 11(b)) · **C** close by record: clippy is not a v1.0 gate; drop `clippy` from `components` (`:81`) | **A with the T0 STOP.** Installed-but-never-run is rule 11's exact case; the warning count is unmeasured (no cargo here, measurement 4), so the STOP keeps a large lint sweep from riding in unpriced. | PROPOSED |
| **D-20-3b-12** | prepush mirror (`deferred-work.md:1223`) | **A** derive `GATES` from the registry Blocking set + workflow literals · **B** add the literal `check-epic-close-coherence` leg + an `xtask/tests` pin asserting every Blocking aggregate gate with a local form appears in `GATES` · **C** leave | **B.** Same protection, +1…+5 production lines; A must reconstruct CLI arguments the registry does not carry. Both fix the runtime `CARGO_MANIFEST_DIR` read for copied binaries (cwd-ancestor fallback, the `check_kernel_baseline.rs:462-466` pattern). | PROPOSED |
| **D-20-3b-13** | `pub use` group hash | **A** hash each leaf from its own path tokens; regenerate the surface baseline once · **B** keep group hash, report one alarm per `Item::Use` · **C** leave filed | **A.** Fixes the attribution at origin (rule 10(a)); one-time baseline regen with the FR31 surface change is a single re-pin. | PROPOSED |
| **D-20-3b-14** | I1–I14 ledger mechanism | **A** a `claim: guaranteed \| asserted` frontmatter key per `docs/invariants/I*.md` with the property test / adversarial corpus named, read by the existing `invariant-lock` (`invariant_lock.rs`) · **B** a new xtask gate `check-invariant-ledger` (+60…+150) | **A.** One instrument (the lock already owns `I*.md`, `docs/invariants/README.md:7-12`); PRD `success-criteria.md:38` sanctions the downgrade. | PROPOSED |
| **D-20-3b-15** | Per-crate envelope (D02 A applied) | **A** the envelope above (allowance ×3 unmeasured / ×2 measured) · **B** exact measured grants only, every review correction re-asks · **C** allowance ×1.3 (rule 6) | **A.** E19 landed at 2–3× forecast with material review corrections (`epic-19-retro-2026-10-30.md:40-44`); ×1.3 is not demonstrated adequate (preflight §1.3). A is a ceiling, never a reservation — pins land exact. | PROPOSED |
| **D-20-3b-16** | Freed room | **A** lower xtask and aggregate pins to measured (+1 aggregate) at each landing; later adds — including L2 — ask exact measured grants · **B** hold 20-3b's L2 forecast under the old pins until done · **C** re-grant freed lines as an E20 envelope for 20-2/20-5/20-7 | **A.** B and C are reservations, forbidden (`kloc.toml:86-89`); D02 A recognizes savings only after deletion. | PROPOSED |

## Acceptance Criteria

*(Epic §20-3b as amended 2026-10-30; clauses marked (D-…) follow the recommended row.)*

### AC1 — the spike (≤ 2 days, rule 5): a written GO/NO-GO before any generator code

1. `20-3b-…-evidence/spike-go-no-go.md` answers E20@HEAD:130 (1)–(6) with HEAD cites, plus: S7 what `delivered_phase: v0.1-alpha` means for `mode: hard` (which rows become enforced, and whether the matrix's phase must move with `phase-config.toml`); S8 each of the 33 violations' disposition (rename to the registered name, register, correct the phase per D-20-3b-03, or remove the row's claim) — no violation baselined away.
2. **Oracle:** the spike generator regenerates today's `tests/coverage-matrix.yaml` **byte-identically** from source + registry before any intentional change (`cmp` exit 0 recorded). No proc-macro crate.
3. NO-GO ⇒ AC2 is cut, `check-coverage-matrix-completeness` stays (and its prepush leg), the 33 are still fixed at origin and `mode: hard` still lands (D08 A) on the hand-kept YAML.

### AC2 — the generated matrix blocks (on GO; D-20-3b-02 A, D08 A)

1. `cargo run -q --locked -p xtask -- gen-coverage-matrix --check` exits 0 on the committed pair and 1 on drift; the YAML header says it is generated.
2. `invariant_lock.rs:156` reads the source file as the corpus-delta witness, not the generated YAML.
3. The generator refuses a `v1.0` requirement with empty `gates` (subsumes `check_coverage_matrix_completeness.rs`, which retires in AC3).
4. The 33 violations are zero; `tests/coverage-matrix.yaml:3` → `mode: hard`; `coverage-matrix --json` passes. NFR-Rel-1/2/11 (`:1421,1431,1436`) are named honestly (gate or empty-with-deferral).
5. prepush: `check-coverage-matrix-completeness` leg (`prepush.rs:25-28`) replaced by `gen-coverage-matrix --check`; `proof-contract.md:156` updated.
6. **Proven red:** a source row naming an unregistered gate ⇒ the generator refuses; a hand-edited generated YAML ⇒ `--check` exits 1 (in-tree `xtask/tests` vectors).

### AC3 — ten retirements, deletion first, closure proven

1. Order: `check_epic_6_bridge.rs` first (load-bearing, E20@HEAD:133); the deletion lands in the same landing as every xtask add (D-20-3b-16).
2. Each of the ten is removed from **every** site in `creation/cites-23f58da4.md` C6 (modules; `mod`/command/dispatch; `EXPECTED_GATES`; registry `gates` + `[[ship_gate]]`; YAML; workflow jobs, `aggregate.needs`, ship-gate needs and summaries, the aggregate script outputs; `smoke-discipline-7-1-5` + maos-bin arm per D-20-3b-10; prepush; xtask tests; ADR/doc text) — `check_coverage_matrix_completeness.rs` only on GO.
3. Retained behaviour (nothing protective is lost): cross-form per D-20-3b-04 (ADR-031 `:75` amended); deprecations → `stability-matrix --check`, CR-2 exception text deleted (`proof-contract.md:159,167`); governance round-trip → `crates/maos-audit/tests/` (D-20-3b-08); the three folds per D-20-3b-09 with negative witnesses: a bare `_No review findings._` placeholder reds `check-review-findings-resolved`; a declared-active story with no file / no model reds `check-dev-record-completeness` (the d19 vector `:351-386` keeps its surviving walkers); a job-level `if: false` reds `check-epic-close-coherence`; the dev-model helpers move with the two gates that import them; ADR-040 `:5`, ADR-043 `:3`, ADR-045 `:60`, ADR-068 `:6` amended.
4. **Closure check (the AC's command):** `git grep -nE '<ten CLI names>|<ten module names>'` over `crates xtask .github docs tests` returns only an explicit provenance allowlist (ADR history lines, the `discipline.yml:2890-2898` retirement comment) recorded in the Dev Agent Record; `cargo run -q -p xtask -- check-epic-close-green` exits 2 (clap unknown subcommand) for each retired name.
5. **Proven red (one plant, in-tree):** a planted `if: false` job in a scratch workflow reds `check-epic-close-coherence` after the fold (the retired gate's control survives).
6. `check-ship-gate-completeness` passes with the shrunk set (46 → 45 ship rows; `EXPECTED_GATES` −1).

### AC4 — FR31: a recycled pid crosses no Spirit's memory (D03 A, D-20-3b-05 B, D-20-3b-06 A) ◆ FLAG-Winston

1. Kernel: `kfull-k1-k2b-bundle.diff` shape — principal values always durable; spill tree under `id-<hex(identity)>` once bound; forget walks identity and legacy pid dirs; the index records the owner (guarded `ALTER` for existing DBs) and export reads by recorded owner; non-Unix twin of `read_owner` refuses like the store (`private.rs:658-664`).
2. maos-bin: `bind_storage_identity(pid, spirit_id)` after `on_loaded(pid)` and before admission/start on every admission path (`admission.rs:859`), and on the shell hello path before `start` (`main.rs:4363-4368`); a refusal rolls back typed (`AdmissionRefusal::Load`).
3. **Witnesses (red at `23f58da4`, green after):** (a) confidentiality — boot 1 Spirit A at pid 1 writes `principal:<id>:<schema>`; boot 2 Spirit B at pid 1 reads it ⇒ `None`; (b) integrity — boot 1 a non-Butler Spirit at pid 1 writes an all-accept `butler.acceptance/v1`; boot 2 Butler restores 0 records (`deferred-work.md:1199`); (c) durability — a non-Markdown principal value ≤ 4 KiB survives a restart for the same identity; (d) forget erases identity dirs and legacy pid dirs (count asserted); (e) export of a boot-1 row in boot 2 returns the boot-1 owner's value. Kernel vectors in `crates/maos-kernel-core/tests`, the admission binding in a maos-bin test (kloc-free).
4. **Proven red (one plant):** `owner_dirname` returning the pid ⇒ (a) and (b) red.
5. Pins: both kernel pins, content hashes, surface baseline, class rows; FLAG-Winston decision record per the ruling; HISTORY rows cite this key.
6. Disclosed, not hidden: pre-fix pid-keyed spills are not read by bound Spirits (Butler's acceptance ledger restarts empty once); same-name squatting is the D-20-3b-06 residual owned by 20-1.

### AC5 — routed witnesses and the invariant ledger (`routing-audit-2026-10-22.md:45,144-145,155-161`)

1. FR34: a binary-level witness that the `spirit-test` SDK harness covers lifecycle hooks, IAC frame I/O, halt resolution, manifest self-check and class (FR:84) for a fixture Spirit — external-author validation remains the operator lane.
2. FR53: a witness that an active halt keeps its **identity** and replay context across hot swap (not only that the successor re-fires, `one_daemon_one_door_16_1.rs:2631-2633`).
3. FR63: a witness over the published catalog: each kernel-emitted error variant has a catalog row with retryability and cause-chain semantics (`docs/errors/error-catalog.json`; `check_error_catalog.rs`).
4. I1–I14 (D-20-3b-14 A): each `docs/invariants/I*.md` carries `claim: guaranteed | asserted`, the property test and the adversarial corpus that earn `guaranteed`; any without is `asserted` with an owner; FR13 quarantine reads per D04.
5. A witness that is red at landing lands **labelled** with its owner's deferred row (rule 11(b)) and the ledger says `asserted` — never weakened to pass.

### AC6 — gate dispositions

1. Clippy per D-20-3b-11 (A: T0 count, then a blocking job in `aggregate.needs`).
2. Group hash per D-20-3b-13 A: one added name in a grouped `pub use` raises exactly one surface change; in-tree vector over a fixture module.
3. The deny closure recorded at creation (`deferred-work.md:903,911`) is re-checked at landing (advisory DB current at that run).

### AC7 — prepush mirrors every local Blocking gate (D-20-3b-12 B)

1. `check-epic-close-coherence` in `prepush.rs` `GATES`; an `xtask/tests` pin fails when a Blocking aggregate gate with a local form is missing from `GATES`.
2. `error-catalog-check` resolves the workspace root from cwd ancestors when `CARGO_MANIFEST_DIR` is absent; prepush as a copied binary does not red that leg.

## Tasks / Subtasks

- [ ] **T0 — Preflight.** Operator rulings on D03, D04, D08, D-20-3b-01…16 recorded; tracker → `in-progress`. Run `creation/measurements.md` items 1, 2, 4; re-measure every cite this story touches (rule 9). If any measured figure exceeds the ruled envelope, STOP and return a D-row.
- [ ] **T1 — Spike (AC1).** Two days max; GO/NO-GO file; byte-identical oracle.
- [ ] **T2 — Retirement (AC3).** `check_epic_6_bridge.rs` first, then the other nine with the closure table; helpers moved; folds with negative witnesses; docs/ADRs.
- [ ] **T3 — Generator (AC2)** on GO; invariant-lock re-point; 33 violations at origin; `mode: hard`.
- [ ] **T4 — Dispositions (AC6, AC7).** Clippy (after T0 count), group hash, prepush pin, root resolution.
- [ ] **T5 — L1 landing (D-20-3b-01 A).** `cargo fmt --all`; `kloc-check --json`; exact lower re-pins (xtask, maos-bin, aggregate); prepush; records.
- [ ] **T6 — FR31 (AC4)** after the FLAG-Winston ruling: kernel bundle, maos-bin binding, five witnesses, one plant; pins.
- [ ] **T7 — Witnesses and ledger (AC5).**
- [ ] **T8 — Plants** (one per AC, `proof-contract.md:24-31`): AC2 drift, AC3 `if: false`, AC4 pid-keyed owner; AC1/AC5/AC6/AC7 are falsified by their in-tree vectors.
- [ ] **T9 — L2 landing.** Exact pins inside the ruled envelope; FLAG-Winston HISTORY rows; `check-kernel-baseline`, `check-service-boundary`, `check-pub-field-constructors`, `check-ship-gate-completeness`, `check-dev-record-completeness`, `check-epic-close-coherence`, `prepush`; records owed at landing. Revert `intent-lineage-coverage-report.md` if a run modifies it. No commit or push without the operator.

## Dev Notes

### What changes, by file

| Area | Files | AC |
|---|---|---|
| xtask (delete) | `xtask/src/check_{epic_6_bridge,cross_form_equiv,adr_040_accepted,literal_reappearance,governance_categories,coverage_matrix_completeness,deprecations_declared,bare_review_findings,dev_model_used_populated,epic_close_green}.rs` | AC3 |
| xtask (edit) | `main.rs:11-83,421-941,1353-1494`; `check_ship_gate_completeness.rs:24`; `prepush.rs:14-45`; `check_dev_record_completeness.rs` (helpers, fold), `check_dev_model_tier.rs:19-21`, `check_review_findings_resolved.rs`, `check_epic_close_coherence.rs` (folds); `coverage_matrix.rs` (unchanged under D08 A); `invariant_lock.rs:156`; `check_service_boundary.rs:393-396,480-483`; `check_error_catalog.rs:644-657`; NEW generator module | AC2, AC3, AC6, AC7 |
| xtask data / tests | `gate-registry.toml:34,64,68,72,270-272`; `tests/coverage-matrix.yaml` (+ NEW `tests/coverage-source.toml`); `phase-config.toml` (D-03 A only); `xtask/tests/{d19_story_file_governance,story_10_2_proven_red,coverage_matrix_completeness_tests}.rs` | AC2, AC3 |
| CI | `.github/workflows/discipline.yml:81,3371-3399,3636-3649,3715-3720,4001-4014,4059-4077,6244-6390,6391-6823,6824-6857` | AC3, AC6 |
| Kernel ◆ | `crates/maos-kernel-core/src/memory/{private,mod,principal}.rs`; pins `xtask/kernel-core-baseline.toml`, `xtask/kloc.toml:264`; `docs/ci-baselines/kernel-surface-v0.1-beta.json`; `xtask/kernel-api-classes.toml` | AC4 |
| Domain / bin | `crates/maos-domain/src/memory.rs:319-325`; `crates/maos-bin/src/admission.rs:859`, `main.rs:4363-4368,9214-9216,13836-13887` | AC3, AC4 |
| Tests (kloc-free) | `crates/maos-kernel-core/tests/` (FR31 a/c/d/e), `crates/maos-bin/tests/` (binding, b), `crates/maos-audit/tests/` (governance), SDK/halt/catalog witnesses | AC4, AC5 |
| Docs / records | ADR-031 `:75`, ADR-040 `:5`, ADR-043 `:3`, ADR-045 `:60`, ADR-068 `:6`; `docs/testing/proof-contract.md:150-170`; `docs/invariants/I*.md`; `xtask/kloc.toml:339,409,425,517,718` | AC3–AC5 |

### Overlap (serialize; re-measure)

- **`maos-kernel-core`** — 20-1 (D04 A), 20-3c (member 12, `halt/mod.rs:465-474`), 20-8/20-9/20-13/20-14 all forecast kernel touches; one integration owner per pin update.
- **`crates/maos-bin/src/main.rs`** — 20-1 / 20-4 / 20-13 (preflight §6 pooled surfaces); this story's edits are small and local (`:4362`, `:9214-9216`, `:13836-13887`).
- **`discipline.yml` `aggregate.needs` / `prepush.rs`** — 20-7 adds perf jobs, 20-2 the air-gap gate, 20-5 the release precondition; land the retirement first so later enrolments start from the shrunk set.
- **`tests/coverage-matrix.yaml`** — every story adding a gate edits it; after AC2 they edit the source instead.

### Current state that must be preserved

- `check-wasm-form-equiv`'s binding disposition and every live Blocking gate's CLI and job.
- The three Epic-6 migrations' negative behaviour (AC3.3) and the d19 walker vector for surviving gates.
- `stability-matrix --check`, `gen-abi-docs --check`, the existing `coverage-matrix` parser (`hard`/`warning`).
- FR31 behaviour for the same identity across restarts (18-4's Butler restore), forget/hold serialization (`mod.rs:83-86`), principal-write enforcement (`:715-726`), the region chokepoints (`:709-713,796-800`).

### Guardrails

- Delete only with the closure table in hand; a retired name still referenced outside the provenance allowlist is a red, not a follow-up.
- No kernel line without the ruled FLAG-Winston row; any `crates/maos-kernel-core/src` change outside AC4's bundle is STOP.
- Never weaken a witness to pass; never baseline a coverage violation away.
- One cargo process; `env -u CI -u GITHUB_ACTIONS`; `CARGO_TARGET_DIR=/mnt/build/cargo`; gates via `cargo run -q --locked -p xtask -- <gate>`; plants byte-restored.

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red |
|---|---|---|
| Generated matrix (AC2) | hand-edit one generated row | `gen-coverage-matrix --check` exit 1 |
| Retired control survives (AC3) | job-level `if: false` in a scratch workflow | `check-epic-close-coherence` FAIL |
| FR31 isolation (AC4) | `owner_dirname` → pid | witnesses (a), (b) FAIL |

### Tests and commands

```bash
# local; one cargo process; CARGO_TARGET_DIR=/mnt/build/cargo
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p xtask
env -u CI -u GITHUB_ACTIONS cargo test --locked -p xtask --test d19_story_file_governance --test story_10_2_proven_red
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --lib memory && env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-kernel-core -p maos-bin -p maos-audit
cargo run -q --locked -p xtask -- gen-coverage-matrix --check && cargo run -q --locked -p xtask -- coverage-matrix --json
cargo run -q --locked -p xtask -- check-ship-gate-completeness --json && cargo run -q --locked -p xtask -- check-dev-record-completeness --json && cargo run -q --locked -p xtask -- check-epic-close-coherence --json
cargo run -q --locked -p xtask -- kloc-check --json && cargo run -q --locked -p xtask -- check-kernel-baseline --json && cargo run -q --locked -p xtask -- check-service-boundary
cargo run -q --locked -p xtask -- prepush
```

### Review plan (`proof-contract.md:33-75`)

- DIFFICULT: Blind and Edge (opus-5.5) — Edge walks every retired name's closure, the fold negatives, legacy pid dirs vs identity dirs under forget/export/hold, the guarded `ALTER` on an existing DB, an admission refusal after binding, a restart between write and index; Acceptance (opus-5.5); Security on AC4 (read + poison directions, same-name residual); Test Infrastructure (glm-5.3) re-reads the workflow closure and the new jobs.
- Focused set: `xtask` (all), `maos-kernel-core` (`memory`), `maos-domain`, `maos-bin` (admission + shell + the 7.1.5 arm), `maos-audit` (governance test), every reader of `gate-registry.toml` (`grep -rl gate-registry xtask/src`).

### Previous-story intelligence

- **20-3a (done):** gate honesty; `D-20-3a-G` — gates this story deletes were deliberately not repaired; `BindingClass`/`EvidenceState` vocabulary is what the generator reads.
- **19-4 / 19-5:** priced by prototype and still landed far over forecast; the kloc ruling `w6_kloc` raised maos-bin at landing — hence D-20-3b-15's allowance. 19-5 shows the kernel re-pin bookkeeping (content hashes, surface, class rows) the AC4 grant needs.
- **18-4:** wrote the first production principal record and routed the pid-reuse finding here; D-18-4-02 C's +1/+1 is K1, re-measured.
- **18-6:** forget/redistillation relies on boot-local pids being non-resumable (`18-6-…md:32,144`); identity keying does not change that contract.
- **20-16:** built `prepush`; its CR-2 exception for `check-deprecations-declared` retires here.

### Git intelligence

| Commit | Content |
|---|---|
| `23f58da4` | Epic 20 preflight report (this creation's HEAD; no code) |
| `3fd7f8b4` | xtask d11 ratchet grant for 19-4 (47474 → 47682); deferred-work flake rows |
| `43ce3ea7` | 19-4, 19-5, epic 19 done; E19 retro |

### References

- Epic `epic-20-ship-it-w5.md` @23f58da4 `:64,95-121,123-133,278-280`; preflight `epic-20-preflight-2026-10-30.md:95-111,121,184-191,281-297`; tracker `sprint-status.yaml:298-299`; `deferred-work.md:902-903,910-911,1198-1200,1223-1224`; routing audit `:45,113,133,144-145,155-161`; PRD `functional-requirements.md:67,77,84,106`, `success-criteria.md:38-39`; E19 `epic-19-founder-loop-w4.md:40-49`, retro `epic-19-retro-2026-10-30.md:40-44`.
- Code: `xtask/src/{main.rs,prepush.rs,coverage_matrix.rs,invariant_lock.rs,check_ship_gate_completeness.rs,check_service_boundary.rs,check_dev_model_tier.rs,check_dev_record_completeness.rs,check_error_catalog.rs}` (lines above); `xtask/gate-registry.toml`; `xtask/kloc.toml:60-116,264,339,409,425,489,517,718`; `crates/maos-kernel-core/src/memory/{private.rs:56-74,263-278,618-677,679-726,806-923, mod.rs:77-138,401-414,688-860,905-935, principal.rs:16-140}`; `scheduler/scheduler_loop.rs:29-33`; `crates/maos-domain/src/memory.rs:319-325`; `crates/maos-bin/src/{admission.rs:840-956, main.rs:2154-2157,4355-4372,9214-9216,13836-13887}`; ADR-069 `:35-46`.
- CI: `discipline.yml:81,157-189,2890-2898,3371-3399,3636-3649,3715-3720,4001-4014,4059-4077,6244-6390,6391-6823,6824-6857`.

## Dev Agent Record

### Story context creation

Created 2026-10-30 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter measured contract) at `23f58da4`, without cargo (parent rule).

**Inputs:** the epic's §20-3b text at HEAD (EpicAmend amending it concurrently; preflight §1.4/§3.2 and D03/D04/D08 taken as authoritative where they differ); direct reads of every cited range at HEAD (`creation/cites-23f58da4.md`); kloc and kernel by the prebuilt gate binary on `/tmp/proto203b` `git archive` copies (`creation/tokei-23f58da4-and-prototypes.json`); prototypes K1, K2a, K2b, R, R′, B1, X1 (GO/NO-GO) saved as diffs/scripts, `rustfmt`'d, never applied to the tree; coverage violations by a Python replica of `coverage_matrix.rs:93-160` (the gate itself not in this run's permitted list).

**Measurements owed:** `creation/measurements.md` items 1–6.

**Records owed:** `creation/records-owed.md` (E-rows to the epic owner; tracker and deferred-work rows applied here; landing rows).

**Revert:** nothing to revert — no crate source was modified (prototypes only under `/tmp/proto203b`); no test ran, so `intent-lineage-coverage-report.md` is untouched.

**Gates after the record edits** (prebuilt `/tmp/xtask-maos-w3`, `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=…/xtask`, `--json`, in-tree): `check-dev-record-completeness` rc 0 (195 done stories, 105 deferred-owner assertions, 0 violations); `check-epic-close-coherence` rc 0 (21 epics, pin 26409, 0 violations); `kloc-check` rc 0 (aggregate 192778, `alarm=true`, `over_budget=[]`); `check-kernel-baseline` rc 0 (26409 / 95 files, set hash `66dc2e24…`, no drift).

### Agent Model Used

anthropic/claude-opus-5-5 (creation).

### Debug Log References

- Creation evidence: `20-3b-gate-retirement-and-coverage-generator-evidence/creation/{measurements.md, cites-23f58da4.md, records-owed.md, tokei-23f58da4-and-prototypes.json, coverage-matrix-replica.py, x1-retirement-prototype.py, x1-retirement-enrolment-edits.diff, k1-principal-durability.diff, k2a-identity-keyed-spill-dirs.diff, k2b-identity-index-owner-column.diff, krec-k1-k2a-bundle.diff, kfull-k1-k2b-bundle.diff, b1-maos-bin-identity-binding.diff}`.

### Completion Notes List

### File List

- Creation: this story file; `20-3b-…-evidence/creation/*`; `sprint-status.yaml:299`; `deferred-work.md:903,911,1200,1224` (annotations).

### Change Log

- 2026-10-30: Story created at `23f58da4` (ready-for-dev; D03, D04, D08 and D-20-3b-01…16 PROPOSED). Enrolment closure re-measured (wider than six sites; maos-bin smoke arm and two library dependents found); retirement gross measured −5409 xtask (GO) / −5316 (NO-GO); 33 coverage violations reproduced by replica; FR31 kernel repair priced R′ +91/+80 (R +42/+34), maos-bin +9, maos-domain +1; deny half of `deferred-work.md:902` and all of `:909` recorded closed by 18-0's blocking job and the landed lockfile.
