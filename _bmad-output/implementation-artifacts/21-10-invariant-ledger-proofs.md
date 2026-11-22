---
story_id: '21.10'
story_key: 21-10-invariant-ledger-proofs
created: 2026-11-20
status: in-progress
context_commit: de16fa0af0f3644f6a3fa21610ab24ac10d4ee3f
baseline_commit: de16fa0a
security_sensitive: true
kernel_grant: "Forecast only; no prototype or grant requested. With the approved D-row options the kernel shrinks: −223…−133 tokei (deletion of the unused auto-allow path −222…−233, I14 fix-at-origin +10…+40, contingent in-src test seams 0…+60). A fix forced by a falsified property is contingent and unforecast. ADR-069 kfree applies inside the envelope: measure at landing, re-pin the kernel exact pin with scripts/repin.py and write its HISTORY row in the same commit. No forecast reaches the 22,000 alarm."
kloc_grant: "Read-the-files forecast, not measured growth or permission: maos-domain +120…+350 (D-21-10-02 A); xtask +25…+80; maos-bin −3 + 0…+120 (D-21-10-06); maos-manifest 0 (D-21-10-05 A). Tests, corpora, fuzz targets and workflows are outside production budgets. Re-measure at landing."
review: "DIFFICULT: the ledger is the v1.0 product claim, and the work touches capability mediation, consent, hot swap and the kernel T3 arm. Follow docs/testing/proof-contract.md and D-E20-6. Story creation anthropic/claude-opus-5-5 (DraftInvariantLedger). No independent review or execution claimed."
---

# Story 21.10: I1–I14 invariant ledger proofs

Status: in-progress

**Operator approval baseline:** At approval on 2026-11-21 UTC, the story was `backlog` (not promoted to `ready-for-dev`). The operator approved all seven D-rows. `21-10` remains the one tracker key, with three independently green landings; L-b, including I14, waits for 21-9's default-on build. I5 stays `asserted` until both FR31 and FR13 are proven. The approval itself changed no task, story, tracker or guarantee status and supplied no execution evidence.

**GA-required.** Size 3 × M under one tracker key: L-a (mechanics + I1–I4), L-b (I5, I6, I7, I14; after 21-9), L-c (I8–I13). Each landing must be independently green and may flip only invariants it proves (D-21-10-01 A). Authority: `planning-artifacts/epics/epic-21-multi-host-on-live-substrate-w6.md:251-279` (section and 2026-11-13 note `:271`), exit lines `:36-43`; tracker row `sprint-status.yaml:422`; `planning-artifacts/sprint-change-proposal-2026-11-13.md:37,54,79`; `planning-artifacts/sprint-change-proposal-2026-11-12.md:79,95-97,216-226`; PRD `planning-artifacts/prd/success-criteria.md:38`; live definition `planning-artifacts/architecture-maos-minimal-opus/3-vocabulary-invariants.md:22-73`. Paths under `planning-artifacts/` are relative to `_bmad-output/`.

## Story

As **an operator or auditor of a MAOS v1.0 install**,
I want **each of the fourteen kernel invariants to carry one falsifiable predicate, a property test and an adversarial corpus that run in CI, with the ledger reading `guaranteed` only on that evidence**,
so that **"All 14 kernel invariants (I1–I14) empirically verified by v1.0 … No predicate, no claim" (`success-criteria.md:38`) is true, and `25-1-maos-verify` can run the same predicates on my install**.

**Exact scope:** epic AC1–AC7, plus AC8 (callable predicates, SCP 2026-11-13 §2), AC9 (deletion of the unused auto-allow approval path inside the I4 work, SCP 2026-11-13 §3 item 2), AC10 (production caller of the kernel T3 admission arm, D-E20-1 A) and AC11 (the invariant gate's own defects, rule 10(a)). The GA bar is **14 of 14 `guaranteed`** (D-21-PF-20 A); no invariant is narrowed below its §3.2 live definition. I5 flips only after 21-10 proves FR31 and `later-registry-quarantine-closure` proves FR13. Not in scope: the quarantine fences themselves, the `maos verify` verb (25-1), interactive approval (24-3, post-GA), the J4-extended I14 fuzz corpus (v1.5 cadence), new property crates (D-21-PF-19 C).

## Inventory at HEAD — source measurements, not execution

Read at `de16fa0a` (the touched files are clean against HEAD in this worktree). No Cargo, test, gate, formatter or prototype was run. `tokei` was run on one file to size a deletion.

| Surface | State at HEAD | Consequence for this story |
|---|---|---|
| Ledger | `docs/invariants/I1.md`…`I14.md`: all `claim: asserted`; `property_test: null` in 13 of 14. I5 names `principal_identity_20_3b.rs#recycled_pid_cannot_disclose_another_spirits_principal_spill`, corpus = the test file itself, `ci_job: workspace-test-suite`. Owners: `20-3b` ×12, `20-1b-…` (I5), `20-9-…` (I14), all done keys. | AC6/AC7. I5's current evidence would not pass the gate (rows below). |
| Claim gate | `xtask/src/invariant_lock.rs:52-100` `validate_claim`; `:104-287` `executing_witness`. It **refuses** a test file with any file-level `cfg`/`cfg_attr` (`:168-171`); it accepts only fns whose attributes are all exactly `#[test]` (`:177-178`), so `#[tokio::test]` is refused; the corpus must be bound by `include_str!`/`include_bytes!` in a `let` and used in an assertion (`:189-219`); the package must use normal targets (`:231-236`); the job must hold a one-line `cargo test [-p] <pkg> --test <bin> [name]` step with the corpus path in job/step `env` (`:248-286`). | Every guaranteed property test is a synchronous `#[test]` with no file-level cfg. `principal_identity_20_3b.rs` (`#![cfg(unix)]`), `spawned_swap_20_9.rs` (`#![cfg(all(target_os = "linux", feature = "network", feature = "wasm-host"))]`, `#[tokio::test]`) and `routed_halt_swap_20_3b.rs` (`#[tokio::test]`) are anchors, not admissible property tests. |
| Lock gate | `invariant_lock.rs:374-486`: corpus delta = `tests/coverage-matrix.yaml` in the changed list (`:432`); phase commitment = `has_cadence_change` (`:505-513`), which searches the **whole** `git diff HEAD~1` text for the substring `enforcement_cadence`; reviews only with a PR (`:515-562`); forward-only cadence (`:564-593`). Domain invariant types are detected by the path `crates/maos-domain/src/invariants.rs` (`:408-413`), a file that does not exist (the module is the directory `crates/maos-domain/src/invariants/`). | [INFERENCE from reading] Diff context lines include `enforcement_cadence:` whenever a frontmatter line 4–8 changes, so requirement (c) passes without a cadence change; edits to the domain invariant modules are not lock-gated. Both are this story's controls (AC11, D-21-10-03). |
| CI binding | `invariant-lock` job `.github/workflows/discipline.yml:933-972`: claim gate `:949`, changed files `HEAD~1` `:951`, lock run `:952-967`. `workspace-test-suite` (`:5291`) runs nextest, not per-binary `cargo test` steps. | No existing job satisfies the gate's step shape for these tests (D-21-10-04). |
| Fuzz | Targets: `crates/maos-kernel-core/fuzz/fuzz_targets/cap_token_verify.rs`, `crates/maos-domain/fuzz/fuzz_targets/frame_deser.rs`, `crates/maos-manifest/fuzz/…/manifest_parser.rs`, `crates/maos-exec-deps/fuzz/…/fuzz_exec_deps_parse_elf.rs`. Enrolled: `fuzz-cadence.yml:24-35` and `fuzz-prerelease.yml` list manifest_parser, frame_deser, exec-deps; discipline `fuzz-build` `:4016-4021` builds the same three. | `cap_token_verify` (I1) is **not enrolled anywhere**. I8 and I11 have no target. D-21-PF-19 C requires all three on the existing fuzz workflows. |
| Predicates | `crates/maos-domain/src/invariants/i1.rs`…`i14.rs` (1,092 tokei): marker types plus value types (`Scope::entitles` `i1.rs:158`, `LogBeforeDeliver` `i2.rs:36-62`, `FrameOrigin` `i3.rs:31-50`, `ApprovalDecision` `i4.rs:36`, `AllowedPromotionSet::allows` `i13.rs:80`, `HaltContinuityCheck` `i14.rs:32`). No predicate function exists. | AC8 adds them (D-21-10-02). |
| Auto-allow path | `crates/maos-kernel-core/src/security/approval.rs` (277 physical / 221 tokei): `ApprovalManager::prompt` always returns `Ok(true)`; `prompt_with_posture` silently allows. Constructed and dropped at `crates/maos-bin/src/main.rs:3416` (import `:89`); no production caller. Placeholder `SecurityManagerPort::approval_class` (`crates/maos-domain/src/ports/security.rs:33`) auto-approves at `crates/maos-kernel-core/src/security/mod.rs:511-521`; no caller. Class rows `xtask/kernel-api-classes.toml:344-346`; I9 exemption `docs/invariants/i9-exemptions.md:148-182`; test `crates/maos-kernel-core/tests/approval_prompt_e2e.rs`. Approval-required capabilities are refused (`capability/mod.rs:221-223`). | AC9 (D-21-10-07). Real Approval Decision Log producers remain (`halt/mod.rs:71`, `security/posture.rs:82`, `operator_door.rs:726,886`, `orchestrator_root.rs:753`, `skill_authoring.rs:185`, `maos-orchestrator-buffer/src/lib.rs:44-88`), so I4's predicate is not vacuous. |
| T3 arm | `security/mod.rs:362-386` runs the image lock only at effective T3; effective tier = strictest of manifest/trust/operator floors (`cap_policy/mod.rs:225-247`), each defaulting to T2 (`i9.rs:90`); bin admission admits at `DEFAULT_FLOOR` (`maos-bin/src/admission.rs:300-305`). The only T3 production shape is the `[cli_wrapper]` Worker grant path (`worker_spawn.rs:172-180,232-301`), which does not call `admit_spirit`. | AC10 (D-21-10-06). |
| Open I14 rows | `deferred-work.md:132` (successor without `halt_protocol_compatibility` treated as v1: I14 fail-open, ownerless, open); `:133` (gate halt set judged at step 3, monitor baseline at step 11, ownerless, open); `:1660` (I14 doc does not name the spawned swap vectors; owner `21-10`). | §3.2's I14 text requires the successor to declare compatibility; `:132` falsifies that at HEAD. D-21-10-05. |

## Decisions

### Already settled; do not ask again

| Decision | Binding disposition |
|---|---|
| `ga3` (2026-11-12) | GA precondition. |
| D-21-PF-18 A | This story edits `docs/invariants/I*.md` and the README routed-witness table under `invariant-lock`, superseding for 21-10 the "never edited by a story" ruling recorded at `deferred-work.md:1660`. Push-to-main needs no PR approvals (`invariant_lock.rs:515-533`). |
| D-21-PF-19 C | Hand-rolled seeded generators; cargo-fuzz targets for I1, I8, I11 on the existing fuzz workflows; no property crate. |
| D-21-PF-20 A | GA bar 14/14 `guaranteed`; a ledger entry flips on evidence only. |
| D-E20-1 A | This story owns a production caller for the kernel T3 admission arm (I9 tier floors). Its shape is D-21-10-06. |
| D-E20-8 A | LIVE = built-binary tests only. |
| D-E20-6 | GPT-family security review standing while GLM quota is out; hold for a new trust boundary. |
| SCP 2026-11-13 §2 / §3 item 2 | Each predicate is a callable function reused by `25-1`; the unused auto-allow path is deleted inside the I4 work; interactive approval is a v1.0 claim boundary (24-3 after GA). |
| ADR-069 / `kfree` | In-envelope kernel change is self-service; measured at landing. |

### Operator-approved D-rows — 2026-11-21 UTC

| ID | Choice and options | Approved disposition and rationale |
|---|---|---|
| **D-21-10-01** | Key structure. **A** one tracker key `21-10`, this file, three landings L-a (mechanics + I1–I4), L-b (I5, I6, I7, I14; after 21-9), L-c (I8–I13), each independently green and flipping only what it proves. **B** split into tracker keys `21-10a/b/c` as the SCP proposed; this file becomes 21-10a and b/c get their own files. **C** one XL landing. | **A approved.** Tracker `:422` and epic `:18,:74` keep one key; `25-1` depends on `21-10` (`epic-25-proof-you-can-run-w10.md:41,56`); the landings give the per-part greenness the SCP 2026-11-12 `:216-226` split wanted without re-keying the GA list (`sprint-status.yaml:484`). C is the SCP's not-recommended option. |
| **D-21-10-02** | Home and shape of the callable predicates (AC8). **A** pure functions `maos_domain::invariants::iN::check(&Observation) -> Result<(), Violation>` in the existing `i1.rs`…`i14.rs`, over typed observation records the tests and `25-1` both build. **B** a new leaf crate (e.g. `maos-invariant-predicates`) depending on `maos-domain`: new kloc row, `check-workspace-count` change. | **A approved.** The modules already exist per invariant and are on `maos-bin`'s dependency path; kernel 0. Lock coverage of those files needs the AC11 path fix. |
| **D-21-10-03** | ADR-037 requirement (c) on a flip. **A** amend ADR-037 (`docs/adr/ADR-037-constitutional-amendment-process.md:11`) and the gate: a validated `asserted → guaranteed` flip is the phase commitment; `has_cadence_change` reads zero-context hunks; one planted red each. **B** keep (c) literal and fix the gate to zero context; every flip landing must change a cadence cell. **C** leave the gate as is and label the limitation. | **A approved.** v1.0 cells are fixed by §3.2.1 (`3-vocabulary-invariants.md:63`); B would force invented promotions; C keeps a control that passes on context lines (rule 11). |
| **D-21-10-04** | CI binding for the property tests. **A** one new compiling discipline job (e.g. `invariant-ledger-proofs`) with one direct `cargo test --locked -p <pkg> --test <bin> <name> -- --exact` step per guaranteed invariant, corpus path in step `env`; the claim gate's CI parser is unchanged. **B** teach the gate a nextest-archive consumer invocation. **C** add steps to existing per-crate jobs. | **A approved.** It matches the gate's only accepted shape (`invariant_lock.rs:248-286`) and keeps the 20-15 consumer rules intact (`proof-contract.md:129-172`). B widens a security gate's parser. Measure the test-budget cost of the duplicate workspace-suite run at landing. |
| **D-21-10-05** | Ownerless I14 rows `deferred-work.md:132,133`. **A** fix at origin in L-b: a successor without `halt_protocol_compatibility` is refused when the predecessor has pending halts; the gate's carried set is the one snapshot handed to the monitor. **B** as A, but an omitted field becomes an explicit manifest default (schema/version discipline, maos-manifest change). **C** leave both open; I14's predicate excludes them, which conflicts with §3.2's I14 text and D-21-PF-20 A. | **A approved.** Rule 10(a): both sit in `hot_swap/coordinator.rs`, inside this story's I14 blast radius; refusal is the fail-closed reading of "the successor's manifest declares" (`3-vocabulary-invariants.md:41`). Kernel +10…+40. |
| **D-21-10-06** | Shape of the T3 production caller (AC10). **A** a `[cli_wrapper]` Worker that resolves to T3 is admitted through `SecurityManagerAdapter::admit_spirit`, so the kernel image-lock arm runs on the daemon path (bin +40…+120, kernel 0). **B** the manifest `[sandbox] tier` becomes a raise-only input to the effective tier for Spirits (kernel +5…+20; admission semantics change; widens the readers per `proof-contract.md:49-55`). | **A approved.** `[INFERENCE]`: the Worker grant path is the only T3 production shape at HEAD (`worker_spawn.rs:232-301`); B changes admission for every Spirit. |
| **D-21-10-07** | Auto-allow deletion scope (AC9). **A** delete `security/approval.rs`, its `pub mod`, the `main.rs:3416` construction and `:89` import, `approval_prompt_e2e.rs`, class rows `kernel-api-classes.toml:344-346` and the i9-exemption entry `:148-182`. **B** A plus the placeholder `SecurityManagerPort::approval_class` (`ports/security.rs:33`) and its impl (`security/mod.rs:511-521`). | **B approved.** The placeholder also auto-approves and has no caller; "v1.0 ships no approval code that is not real" (`epic-24-multi-operator-control-plane-w9.md:25`). B touches a public domain port: ABI/public-api baselines and the surface pin move in the same commit. |

No kernel permission, surface-hash permission or STOP exception is requested. No forecast crosses 22,000; if a landing's measured forecast would, open an **ENVELOPE ALARM** D-row; above 23,000 needs a new operator decision.

## Acceptance Criteria

1. **AC1 (command).** On each landing's final tree: `cargo test --locked -p xtask --test invariant_claims_20_3b && cargo run -p xtask -- invariant-lock --changed-files <landing list> --json` passes; every invariant flipped by that landing reads `claim: guaranteed`, its `property_test` a non-ignored synchronous `#[test]` with no file-level cfg, its `adversarial_corpus` loaded by that test, its `ci_job` an enabled discipline job running it (D-21-10-04). After all landings and the quarantine closure: `test "$(grep -l '^claim: guaranteed' docs/invariants/I*.md | wc -l)" -eq 14` (epic exit line 2).
2. **AC2 — predicates and existing tests first.** Each `I*.md` gains one falsifiable predicate sentence that covers its §3.2 live definition (table below) without narrowing it. Existing anchors are inventoried and linked before a new test is written.
3. **AC3 — property tests.** Seeded generators in the owning crate's `tests/` drive the real code path and check the AC8 predicate; seeds are printed on failure. I1, I8, I11 also get cargo-fuzz targets (I1 = the existing `cap_token_verify`, enrolled) on `fuzz-cadence.yml`, `fuzz-prerelease.yml` and discipline `fuzz-build`.
4. **AC4 — adversarial corpora.** Each corpus is a checked-in file loaded with `include_str!`/`include_bytes!` in the property test body (`docs/invariants/README.md:16-26`); it holds violating inputs the predicate must refuse. Content is defensive test data, not exploit material.
5. **AC5 — proven red.** Each property test has one planted violation that reds it, run once on one image (`proof-contract.md:10-31`). An ignored witness never earns `guaranteed`.
6. **AC6 — I5 and I14.** I5 flips only after this story proves FR31 and `later-registry-quarantine-closure` proves FR13 (its AC5/AC9, epic `:324,:328`). I14 names `spawned_swap_20_9::a_hot_swap_keeps_the_pid_tokens_memory_and_state_and_waits_for_the_active_turn` and `::a_predecessor_that_exits_after_its_snapshot_still_completes_its_swap` (closes `deferred-work.md:1660`) and carries the D-21-10-05 outcome.
7. **AC7 — owners.** No frontmatter `owner` or README routed-witness row (`README.md:46-50`) names a done key. A still-asserted entry names a live owner (I5: `later-registry-quarantine-closure`).
8. **AC8 — callable predicates (SCP 2026-11-13 §2).** One public predicate per invariant at the D-21-10-02 home, used by the property test, with no test-only dependency, so `25-1` can call it from the installed binary.
9. **AC9 — auto-allow path deleted (SCP 2026-11-13 §3 item 2).** Scope per D-21-10-07. Approval-required capabilities stay refused; the real producers keep writing I4 records.
10. **AC10 — kernel T3 arm has a production caller (D-E20-1 A).** Per D-21-10-06, a built-binary test (D-E20-8 A) shows a daemon-path admission at effective T3 passing through `security/mod.rs:362-386` and refusing an unverified image lock typed. This replaces the bin-side-only FR48/NFR-Sec-15 boundary recorded at `epic-20-retro-2026-11-12.md:316`.
11. **AC11 — the gate is a real control (rule 10(a), rule 11).** `invariant-lock` detects edits under `crates/maos-domain/src/invariants/`; the phase-commitment check follows D-21-10-03 and is not satisfied by diff context; each change has a planted red in `invariant_claims_20_3b.rs` or the lock tests.

### Predicate coverage required by the live definition (`3-vocabulary-invariants.md:28-41`)

| Inv. | The predicate must refuse | Landing · v1.0 cadence | Existing anchors (link first; not proof) |
|---|---|---|---|
| I1 | any external effect (tool, network, file, sub-Spirit spawn) without a registry verify of an in-scope token | L-a · fuzz | `maos-kernel-core/tests/policy_per_capability_deny.rs`, `capability_invocation_audit.rs`, `scope_selector_entitlement_17_11.rs`; fuzz `cap_token_verify` |
| I2 | a delivered frame (incl. ACK/NACK) with no earlier Transparency Log row; delivery after a failed log write | L-a · runtime | `maos-iac/tests/iac_log_before_deliver_invariant.rs`, `maos-audit/tests/halt_i2_fail_stop_20_11.rs` |
| I3 | a Spirit auto-response not marked `[auto-sent]` on both sides; a Spirit-forged human origin | L-a · CI | none found by name scan; `FrameOrigin` `i3.rs:31-50`; `maos-audit/tests/fr4_classifier_16_2.rs` mentions auto-sent |
| I4 | an Approval Decision Log row missing actor/target/capability/intent/decision; a granted approval with no row | L-a · runtime | producers in the inventory; `approval_prompt_e2e.rs` is deleted by AC9 |
| I5 | a read of another Spirit's private memory; a write outside the declared namespace (FR31 here; FR13 by the quarantine closure) | L-b · runtime | `memory_i5_isolation.rs`, `nfr_sec_14_cross_spirit_isolation.rs`, `principal_identity_20_3b.rs` |
| I6 | a swap that drops an in-flight call's token, drops in-flight A2A frames, or accepts an incompatible state schema | L-b · runtime | `hot_swap_same_major_lifecycle.rs`, `swap_invariants_20_8.rs`, `spawned_swap_20_9.rs` |
| I7 | telemetry delivered without a per-Spirit subscription; a halt whose `scalar.tap` run-up was not observable | L-b · runtime | `self_telemetry_scope.rs`, `telemetry_firehose_17_4.rs`, `maos-bin/tests/hook_pump_17_4.rs` |
| I14 | a swap with a non-empty halt set that neither drains all nor carries all with resolution state; a successor without a matching compatibility declaration | L-b (after 21-9) · runtime | `routed_halt_swap_20_3b.rs`, `halt_carry_20_3c.rs`, `hot_swap_halt_continuity_corpus_integration.rs` + `maos-eval/fixtures/halt-continuity-corpus-v0`, the two spawned vectors |
| I8 | cross-host delivery without sender policy and receiver acceptance for the frame's typed intent | L-c · fuzz | `maos-a2a/tests/cross_host_consent_v1.rs`, `maos-a2a-core/tests/cross_host_consent_v1_5.rs`, `cohort_consent_chokepoint_12_2.rs`, `maos-bin/tests/consent_refusal_1b.rs` |
| I9 | persistent kernel state outside the three holders or unregistered exemptions; a secret persisted by the kernel; statistical state in kernel-core; plus AC10 | L-c · CI | `xtask/tests/empty_kernel_integration.rs`, `check-empty-kernel`, `i9-exemptions.md` |
| I10 | a lifecycle transition not journaled; a cold boot that does not rehydrate; crash detection > 2 s; `task.orphaned` > 5 s or without cause | L-c · runtime | `journal_append_16_5.rs`, `journal_fsync_assertion.rs`, `audit_spine_integration.rs` |
| I11 | a persisted digest with empty or non-flattened `source_log_ref` or missing `distillation_depth` not refused `EDigestAuditChainMissing` | L-c · fuzz | `distillation_i11_audit_chain.rs`, `maos-eval/fixtures/distillate-corpus-v0` |
| I12 | a `decision.*` frame whose `working_memory_digest_refs` omit a declared in-context digest or an inbound raw frame | L-c · runtime | `maos-audit/tests/i12_decision_audit_test.rs`, `i12_real_digest_provider_8_10.rs` |
| I13 | a digest lineage that is not the kernel-computed union; a consumer under Y accepting lineage outside `allowed-promotion-set(Y)` | L-c · runtime | `intent_lineage_corpus_v0.rs`, `maos-iac/tests/iac_bus_intent_lineage.rs`, `maos-eval/tests/intent_lineage_corpus_load.rs` + `intent-lineage-corpus-v0` |

Unqualified test files are under `crates/maos-kernel-core/tests/`. A predicate the real code fails is a defect to fix in that landing (contingent kernel growth below), never a reason to narrow the predicate.

## Tasks / Subtasks

- [ ] **T0 — apply the rulings.** Use the recorded D-21-10-01…07 dispositions and approved-option forecasts. Apply D-E20-6 before any implementation that adds a trust boundary (AC10).
- [ ] **T1 — controls first (AC11, D-21-10-03/04).** In L-a, fix the domain-path detection and the phase-commitment check in `invariant_lock.rs` with planted reds; amend ADR-037 per D-21-10-03 A. Add the CI job and wire `cap_token_verify` into the three fuzz lists. Make no ledger flip as part of this controls work.
- [ ] **T2 — predicates (AC2/AC8).** All 14 predicates and their observation/violation types at the D-21-10-02 home; one predicate sentence per `I*.md`; link existing anchors. Under the lock: `tests/coverage-matrix.yaml` touched in the same diff.
- [ ] **T3 — L-a: I1–I4 (AC3–AC5, AC9).** Property tests, corpora, plants; I1 fuzz leg; delete the auto-allow path per D-21-10-07 with its class rows, exemption entry and surface pins. Flip I1–I4.
- [ ] **T4 — L-b: I5 (FR31), I6, I7, I14 (AC6).** Starts after 21-9's default-on build lands. I14: D-21-10-05 A fixes with red-before vectors; name the spawned vectors. Flip I6, I7, I14 on proof; I5 waits for both this landing's FR31 proof and the quarantine closure's FR13 proof.
- [ ] **T5 — L-c: I8–I13 (AC3–AC5, AC10).** New I8 and I11 fuzz targets; AC10 caller per D-21-10-06 with a built-binary test. Flip I8–I13.
- [ ] **T6 — ledger hygiene (AC6/AC7).** Owners and README routed table; hand the orchestrator the `deferred-work.md:132,133,1660` closures and the tracker row update (not edited by this story's implementers).
- [ ] **T7 — landing measurement (each landing that touches the kernel).** `check-kernel-baseline --json` and `kloc-check --json`; re-pin the kernel exact pin with `scripts/repin.py` and write its HISTORY row in the same commit; `check-service-boundary` with the class rows removed first (`kernel-core-baseline.toml` EOF obligation a–d). No reverted prototype, no FLAG-Winston ask.
- [ ] **T8 — orchestrator verification.** Focused suites per `proof-contract.md:35-55` (add `cargo test -p maos-kernel-core --lib` when the kernel changes); one final-tree matrix run; one plant per AC on one image; hosted fuzz-cadence records for I1/I8/I11. Difficult-tier review; security review for AC10 under D-E20-6.

## Dependent proof work — owed by other keys or later runs

| Item | Owner | Effect here |
|---|---|---|
| I5 FR13 half (quarantine fences, durable quarantine, release verb, CRL→Worker) | `later-registry-quarantine-closure` (`sprint-status.yaml:503`; epic `:314-329`) | I5 stays `asserted` (owner = that key) until both FR31 here and FR13 under the closure's AC5/AC9 are proven; exit line 2 cannot be green before then. |
| WASM host on by default | `21-9-wasm-host-default-on` (in progress, `:421`) | The spawned I14 vectors compile in the default build only after it; L-b starts after it lands. |
| Hosted fuzz records for I1, I8, I11 | `fuzz-cadence.yml` scheduled runs | Enrolment lands here; the run records are evidence, not created by story creation. |
| Predicate consumer | `25-1-maos-verify` (`epic-25-proof-you-can-run-w10.md:42,56`) | Calls AC8's predicates; builds no second copy. |
| I9 exemption sign-offs | human maintainers (`deferred-work.md:1248`) | Not claimed by this story. The I9 predicate checks enumeration; it does not claim the sign-offs exist. |

## Kernel and kloc forecast — method, not a prototype

**Method:** read the files each change touches and count lines added or removed; `tokei` on `security/approval.rs` only. Baseline: latest HISTORY **20,428 tokei / 26,061 physical / 90 files** (`xtask/kernel-core-baseline.toml:505-509`, 27-1 row); ceiling row 20,429 (`xtask/kloc.toml:272`). Recorded values, not a fresh measurement.

| Kernel change | Net tokei | Basis |
|---|---|---|
| Delete `security/approval.rs` + `pub mod approval;` | −222 | 221 tokei code (measured) + 1 |
| D-21-10-07 B: `approval_class` impl | −10…−11 | `security/mod.rs:511-521` |
| D-21-10-05 A: two fixes in `hot_swap/coordinator.rs` | +10…+40 | `None` passed through instead of `[1]` (`:160-164`); one carried-set snapshot to the monitor (`:429-440`) |
| D-21-10-06 A | 0 | caller is bin-side (B: +5…+20) |
| Predicates (D-21-10-02 A/B) | 0 | outside the kernel crate |
| Contingent in-`src` test seams | 0…+60 | only if a property cannot be driven from `tests/` |
| **Total, approved options** | **−223…−133** | → **20,205…20,295**; pre-approval worst case across all options −92 → 20,336. No alarm. |

Tests under `crates/maos-kernel-core/tests/` are outside the pinned set (`check_kernel_baseline.rs:74`); fuzz directories are excluded from tokei (`kloc_check.rs:302`). A fix forced by a falsified property is real kernel growth, measured at landing.

Kloc (production Rust, approved options): maos-domain +120…+350 (14 predicates and their types; row `kloc.toml:435` = 10,483; −1 for D-21-10-07 B); xtask +25…+80 (`:349` = 46,753); maos-bin −3 plus 0…+120 (`:527` = 39,699); maos-manifest 0. Non-kernel ceilings are not per-landing exact re-pins (ADR-069 amendments).

## Cross-story dependencies and claim limits

- Runs beside 21-9; L-b, including I14, starts after its default-on build lands. I5 cannot flip until 21-10 proves FR31 and `later-registry-quarantine-closure` proves FR13. No edge to 21-6, 21-7, 21-12.
- 22-9 owns the transferred WASM swap-invariant witness (`deferred-work.md:1762`); this story does not take it.
- I14's v1.5 `fuzz` cadence and the J4-extended corpus are not claimed. Interactive approval is a v1.0 claim boundary (24-3).
- `guaranteed` means the bound property test and corpus passed in CI on the final tree; it is not a formal proof, a pen-test result or NFR-Sec-7.

## Re-measured cite corrections — parent-owned source updates

Only this file is written; the epic, tracker and deferred-work are not edited. OLD → NEW at `de16fa0a`:

| Source | OLD → NEW |
|---|---|
| Epic §21-10 Scope / preflight §4.7 | `discipline.yml:774-810` → `:933-972` (claim gate `:949`; lock run `:952-967`); preflight `:790-791` → `:948-949`, `:796-804` → `:952-967`. |
| Epic §21-10 AC6 / quarantine section | tracker `sprint-status.yaml:448` → `:503`. |
| Epic §21-10 Closes, AC6, split table | `deferred-work.md:1659` → `:1660`. |
| Epic §21-10 kernel forecast | `kloc_check.rs:294-317` → fuzz exclusion `:302` (excludes `:290-311`). `check_kernel_baseline.rs:74` holds. |
| Epic §21-10 Scope | "fuzz targets exist for capability tokens…" holds, but `cap_token_verify` is enrolled in no workflow (`fuzz-cadence.yml:24-35`, discipline `:4016-4021`). |
| Epic §21-10 note `:271` / SCP 2026-11-13 `:37` | `main.rs:3443` → `:3416` (import `:89`). |
| Epic header kernel figures `:82` | 20,429 tokei / 26,062 physical → HISTORY 20,428 / 26,061 (27-1); kloc row `:264` → `:272`. |
| Retro `epic-20-retro-2026-11-12.md:72,254` | `admission.rs:296-302` → `:300-305`; `security/mod.rs:362`, `cap_policy/mod.rs:225-247`, `i9.rs:90` hold. |
| `invariant_lock.rs` cites | `:430-445` → corpus `:432`, phase `:435-446`; `:505-513` holds; `:515-533` → `:515-562`. |
| Epic exit provenance `:42` | `invariant_claims_20_3b.rs:10,:56,:166` hold; README `:16-26`, `:46-50` hold; `success-criteria.md:38` holds. |
| `deferred-work.md:132,133` | `coordinator.rs:160-164` holds; monitor baseline `:380-389` → `:429-440`; gate `:158-188` holds. |
| Epic §21-10 Scope (new finding) | The gate refuses file-level `cfg` and `#[tokio::test]` (`invariant_lock.rs:168-178`): the named I5 and I14 anchors cannot be property tests as written. |

## Dev Agent Record

### Agent Model Used

Creation: **anthropic/claude-opus-5-5** (`DraftInvariantLedger`). No implementer or reviewer attribution is claimed.

### Completion Notes List — story creation

- Created at `de16fa0a` in `/home/lunarpulse/dev_ws-course28`; only this file written. No epic, tracker, deferred-work, code or doc edits; no staging, commit or push.
- Read `bmad-create-story`, the epic section and both correct-course notes, the tracker row and its archived note, the preflight §4.7 and rulings, SCP 2026-11-12/-13, the retro D-E20-1 record, the architecture §3.2 live definition, the ledger files, the claim and lock gate source, the CI and fuzz workflows, the kernel pin and kloc rows, and exemplars 21-9 and 21-6.
- At story creation, no partial 21-10 implementation evidence existed (no branch, commit or evidence directory); the existing partial evidence was the 20-3b ledger, the routed witnesses and the anchors listed above.
- No Cargo, tests, gates, formatters, remote jobs or prototypes run (assignment). The prebuilt-gate route in the role contract was not used for the same reason. No green, red-before, exact kernel delta or review is claimed.

### File List

- `_bmad-output/implementation-artifacts/21-10-invariant-ledger-proofs.md` — new, story only.

## §A6 independent review artifact — 2026-11-21 (static)

**Scope:** A light reviewer agent performed a static, read-only review; the reviewer did not run builds or tests. An eligible independent security review is still owed for the lock gate. The evidence below is local to `ga-21-10-2026-11-21`; no hosted success is claimed.

**Implemented**

- Added I1–I14 predicates as `check(&Observation)` in `maos-domain/src/invariants`, property tests with adversarial corpora, and ledger wiring to `invariant-ledger-proofs`, with one CI step per invariant (D-21-10-04).
- Enrolled the existing `cap_token_verify` fuzz target and added `i11_digest_chain` and `i8_consent_intent`.
- Deleted the unused auto-allow `ApprovalManager` path. Measured kernel change: −242 tokei, 90 → 89 files. Moved `stamp_as_spirit` to `maos-domain`'s `frame.rs`.
- Fixed the lock gate in `xtask/src/invariant_lock.rs`: it takes the base ref from `INVARIANT_LOCK_BASE` or the PR base instead of `HEAD~1`, compares claims and cadence in parsed YAML frontmatter, and detects edits under `crates/maos-domain/src/invariants/`. Amended ADR-037 per D-21-10-03 A: a validated `asserted` → `guaranteed` flip satisfies the phase commitment.

**Falsified properties fixed at origin**

- **I14:** A successor without a `halt_protocol_compatibility` declaration was treated as v1 and could carry pending halts. It is now refused when halts are pending; `HotSwapCoordinator::precheck` uses the successor's actual declaration. The cutover race for a halt registered after the gate snapshot was recorded, not fixed.
- **I4:** `insert_approval_decision` previously accepted a blank actor, target, capability or intent. It now refuses those records.

**Review findings and disposition**

| Severity | Finding | Disposition |
|---|---|---|
| HIGH | I14: a halt can be registered after the gate snapshot during cutover. | Open residual, explicitly stated alongside the I14 claim. |
| HIGH | I1: a live Worker launch path spawns after `KernelPolicyDenied` under host-grant authority (`crates/maos-bin/src/worker_spawn.rs`, approximately lines 1030–1041). | I1 reverted to `asserted`. |
| HIGH | Lock gate used string search and `HEAD~1`. | Fixed by the base-ref and YAML frontmatter changes above. |
| MEDIUM | I2, I3, I4, I6, I7, I8, I10, I11, I12, I13: the property tests do not all drive production entry points or would not all fail if enforcement were removed. | Downgraded to `asserted` with owner text `witness strengthening owed`. Tests, corpora and CI steps remain as non-claiming witnesses. |

No row is labeled `guaranteed` after the 2026-11-22 review (I14 downgraded, cutover race open, owner 21-9 hot-swap); I5 and I9 stay `asserted`. These are ledger dispositions, not evidence of a hosted pass.

**AC10 not done.** A production caller for the kernel T3 admission arm still requires a design decision: an additive kernel entry point or a throwaway adapter in `maos-bin`. D-E20-6 applies to the trust boundary.

**Local results (orchestrator-run):** I1–I14 invariant property tests green after the I7 seed fix; xtask `invariant_lock` 18/18; `invariant_claims_20_3b` 7/7; `invariant_i9_empty_kernel`, `halt_carry_20_3c`, kernel lib and `maos-domain` green.

**Measured ceilings:** kernel 20,429 → 20,187 (lowered); `maos-domain` 10,483 → 10,782 (measured +299, within the story forecast of +120…+350); xtask 46,754 → 46,762 (+8). After a kernel file was deleted, the kernel pin was updated by hand to `src_lines: 25764`, 89 files.

**Not run:** the named plants for each invariant, `cargo fuzz build`, hosted CI and `check-ship-gate-completeness`. No closures were made for `deferred-work.md:132`, `:133` or `:1660`.

## 2026-11-22 independent review (static, GPT-family: the non-GPT D-E20-6 B review is STILL OWED; no tests run by the reviewer)

Dispositions are as left by the fix pass; the orchestrator compiles, measures and runs the tests afterwards (none run by the fixer).

| # | Finding | Disposition |
|---|---|---|
| F1 HIGH | Review count accepted every APPROVED entry (same account twice, stale-head and superseded approvals). | **Fixed**: `eligible_approvals` requires two DISTINCT eligible maintainer identities (OWNER/MEMBER/COLLABORATOR) whose latest effective review is APPROVED on the current `headRefOid`; test `two_approvals_need_two_distinct_eligible_maintainers_on_the_current_head`. |
| F2 | Verifier runs from the PR's own revision. | **DEFERRED**: run it from a protected revision / required workflow (CI architecture decision for the operator). |
| F3 HIGH | I14 `guaranteed` while the production cutover can violate it. | **Fixed (honesty)**: I14 and its FR53 mapping downgraded to `asserted` with "witness strengthening owed; production cutover race open (owner: 21-9 hot-swap)" in `I14.md`, `docs/invariants/README.md` and the `invariant_claims_20_3b` pin (no row guaranteed). `hot_swap/coordinator.rs` untouched. |
| F4 HIGH | Lock scope omitted test, corpus and CI job of a guaranteed row; diagnostic-only corpus use counted as a check. | **Fixed**: `witness_touched` (base and candidate row; test file, corpus, CI job body) and `operands` (only compared assertion operands count); tests `witness_files_and_ci_job_of_a_guaranteed_row_are_lock_scope` and the two `diagnostic-only corpus` vectors in `guaranteed_refuses_nonexecuting_or_unbound_witnesses`. |
| F5 | Worker launch I1 (`worker_spawn.rs`). | **DEFERRED**: owner 21-10 AC10 design decision. |
| F6 MEDIUM | I14 witness passes without the monitor handoff / precheck fixes. | **Partly fixed**: the witness now asserts the typed `HaltContinuityViolation` refusal and drives the standalone `HotSwapPrecheck::check`; it still cannot fail on the coordinator cutover race (owner 21-9 hot-swap), hence the F3 downgrade. |
| F7 | Review-submission trigger. | **DEFERRED**. |
| F8 MEDIUM | New unrecognized candidate phase keys were not validated. | **Fixed**: `cadence_problem` validates candidate keys (canonical phase or already at the base) and values before comparing; test `candidate_cadence_cells_with_new_unrecognized_phase_keys_are_refused`. |
| F9 | Merge journal PR identity. | **DEFERRED**. |
| F10 | Predicate completeness (i2, i4, i6, i11; I9 predicate sentence). | **DEFERRED**. |
| F11 | T3 admission arm (AC10). | **DEFERRED**: design decision. |

AC verdicts: AC1 partial; AC2 partial; AC3 partial; AC4 enforced for predicate inputs; AC5 not enforced (mutation runs not performed); AC6 partial; AC7 enforced; AC8 API only; AC9 enforced in source; AC10 not enforced; AC11 partial.

## Change Log

- 2026-11-20: created at `de16fa0a`; ACs from the epic plus the cited SCP/D-E20-1 additions, per-invariant predicate coverage, gate-shape constraints, seven open D-rows, forecasts and OLD → NEW cites recorded; status backlog pending operator rulings; tracker untouched (`sprint-status.yaml:422` still `backlog`).
- 2026-11-21: recorded implementation on `ga-21-10-2026-11-21`, the §A6 static review and claim dispositions, and orchestrator-run local evidence only; story `in-progress`.

### Operator approval receipt — 2026-11-21 UTC

- The operator approved the recommended options via the current form, record-only: D-21-10-01 **A**, D-21-10-02 **A**, D-21-10-03 **A**, D-21-10-04 **A**, D-21-10-05 **A**, D-21-10-06 **A**, D-21-10-07 **B**.
- One tracker key (`21-10`), three independently green landings (L-a, L-b, L-c). L-b, including I14, waits for 21-9's default-on build; I5 waits for both FR31 and FR13. No guarantee flips on approval.
- Recorded by **openai-codex/gpt-6.2-sol** (`InvariantPolicyRecord`). At this approval-only step, only this story file changed; the story and tracker remained `backlog`, all tasks remained open, and the receipt claimed no implementation, review or verification.
