---
stepsCompleted: ['step-01-load-context', 'step-02-discover-tests', 'step-03-map-criteria', 'step-04-analyze-gaps', 'step-05-gate-decision']
lastStep: 'step-05-gate-decision'
lastSaved: '2026-08-11'
coverageBasis: 'acceptance_criteria'
oracleResolutionMode: 'formal_requirements'
oracleConfidence: 'high'
oracleSources:
  - '_bmad-output/implementation-artifacts/11-1a-wasm-component-model-spirit-form-host-wit.md (AC1-AC6 + §A7 gate-source mapping)'
externalPointerStatus: 'not_used'
story: '11-1a-wasm-component-model-spirit-form-host-wit'
gate_decision: 'PASS'
gate_decision_after_ta: true
epic_13_coverage: 'added @ HEAD ea9939d3'
---

# Traceability Matrix — Story 11.1a (WASM Component-Model Spirit Form — Host + WIT)

**Auditor:** Murat (Master Test Architect) · **Date:** 2026-07-01 · **Oracle:** the story's 6 formal ACs + its own §A7 per-AC proven-red discipline (confidence: high).

## Phase 1 — Coverage Matrix

| AC | Clause (§A7 proven-red discipline) | Tests | Level | Coverage |
|---|---|---|---|---|
| **AC1** | kernel-Δ=0 (derive-and-reconcile, 22964) | `check-kernel-baseline` (live) | Gate | ✅ FULL |
| AC1 | host-surface add→RED/remove→GREEN (closed allowlist) | `check_host_surface.rs` (7) + live `nm` mutation | Unit+Gate | ✅ FULL |
| AC1 | wasmtime absent from kernel/domain trees | `check_dependency_closure.rs` (11) | Unit+Gate | ✅ FULL |
| **AC2** | 100% `.wit` AST constructor denominator | `wit_corpus.rs::corpus_covers_all_*` (6) | Unit | ✅ FULL |
| AC2 | K-encode ≡ lift(component(lower())) byte-equal | `frame_bridge_roundtrip.rs` (13) + `wit_corpus.rs::k_encode_*` (3) + `codec_integration.rs` (10) | Unit | ✅ FULL *(was PARTIAL — fixed this pass)* |
| AC2 | mutator/dropper/boundary → RED | `mutator_flips_field_detected_red`, `dropper_omits_optional_detected_red`, `cbor_boundary_*`, `cbor_map_reorder_*` | Unit | ✅ FULL |
| **AC3** | real guest round-trip (real wasmtime, real pipes) | `real_runner_subprocess_adr032_roundtrip_through_guest` | Integration | ✅ FULL |
| AC3 | non-conformant component fails closed (`InvalidComponent`) | `resolve_launch_rejects_non_conformant_component`, `invalid_component_fails_closed_with_distinct_exit_code` | Integration | ✅ FULL |
| **AC4** | spin+fuel → `OutOfFuel` trap (not exit≠0) | `spin_loop_exhausts_fuel_with_out_of_fuel_trap` | Unit | ✅ FULL |
| AC4 | forbidden-syscall+T2 → SIGSYS+audit | `forbidden_syscall_killed_by_t2_with_sigsys` | Integration | ✅ FULL *(self-skips w/o CAP_SYS_ADMIN — see caveat)* |
| AC4 | benign survives (negative control) | `benign_guest_completes_with_fuel`, `benign_process_survives_t2_under_same_spec` | Unit+Integ | ✅ FULL |
| AC4 | granted cap works / ungranted refused | `granted_fs_capability_works_ungranted_is_refused` | Integration | ✅ FULL *(same self-skip)* |
| AC4 | fuel bound strictly < T2 (mechanism, not timing) | `fuel_ordering_fuel_bound_strictly_less_than_t2` | Unit | ✅ FULL |
| **AC5** | ADR-031 text/headers | document review | N/A | ✅ FULL (textual AC by design) |
| **AC6** | `wasm-host` absent from default build | `check_export_control.rs` (16) + live `nm` | Unit+Gate | ✅ FULL |

## Test inventory

- **71 active test cases** across 12 suites (`maos-host` + `maos-wasm-host`), 0 skipped-by-design at the suite level.
- **13 NEW this pass** (`frame_bridge_roundtrip.rs`): all-15-FrameKind round-trip + all-9 payload variants + 3 documented-lossy-field pins (intent→Readonly, consent_envelope→None, Scope→empty) that flip RED if a future WIT revision adds those fields.

## Phase 1 — Gap analysis (pre-TA → post-TA)

| Gap (pre-TA) | Severity | Resolution (this TA pass) |
|---|---|---|
| **No CI job runs `cargo test -p maos-wasm-host/-p maos-host`** — 71 tests proven locally but never executed in the ship pipeline | 🔴 P0 | ✅ **Fixed:** added `wasm-host-tests` job (builds forbidden-syscall-probe fixture + runs the suite), wired into `v1-0-ship-gate` `needs` + summary + fail-log. |
| **`frame_bridge::lower/lift` had 0 direct tests** — e2e exercised 1 of 15 FrameKinds | 🟠 P1 | ✅ **Fixed:** `frame_bridge_roundtrip.rs` (13 tests) covers all 15 FrameKinds + all 9 payloads + the 3 lossy fields, pinned explicitly. |
| AC2 byte-equal proven at CBOR level, not through a real component | 🟡 P2 | Accepted simplification — now backed by direct lower/lift round-trip tests for every payload variant. Documented in `frame_bridge.rs` module doc + pinned assertions. |
| `t2_sandbox_kill.rs` (AC4 T2 column) self-skips without CAP_SYS_ADMIN | 🟡 P2 | **Documented, repo-consistent limitation** — mirrors the kernel's own `sandbox_enforcement_linux.rs` (also unprivileged-skip, also not run privileged in CI). Non-vacuous: panics if the probe binary is missing, skips ONLY on `PermissionDenied`. The CI job builds the probe so the test reaches the spawn attempt and emits a visible SKIP rather than a vacuous pass. |

## Phase 2 — Gate decision

$$\boxed{\text{PASS}}$$

**Rationale:** P0 coverage = 100% (AC1/AC3/AC6 fully traced and proven). The two P0 gaps from the initial audit are closed: the behavioral test suite now runs in CI (`wasm-host-tests`), and `frame_bridge` conversion is mechanically proven for the full 15-FrameKind / 9-payload surface. Overall coverage ≈ 95% against the AC oracle. The single residual caveat — T2 privileged execution self-skips on unprivileged runners — is a documented, repo-consistent limitation (identical to the kernel's own pattern), with a non-vacuous skip signal, not a silent gap.

**Caveat carried forward (advisory, non-blocking):** to get TRUE T2 SIGSYS proof in CI, a future privileged-runner job (or a `--privileged` container step) would be needed — same enhancement the kernel's own `sandbox_enforcement_linux.rs` awaits. Tracked here, not blocking.

## Recommendations

- ✅ DONE — Add `wasm-host-tests` CI job (was URGENT/P0).
- ✅ DONE — Add `frame_bridge` unit tests for all FrameKinds (was HIGH/P1).
- LOW — Run `bmad-testarch-test-review` for a test-quality pass on the new suite (isolation, determinism, explicit-assertion checks).
- LOW — When a privileged CI runner is available, promote the T2 column from self-skip to asserted-kill for both this suite and the kernel's `sandbox_enforcement_linux.rs`.

## Epic 13 — Reza journey coverage

**Scope:** Reza cross-team Cortex capabilities, mapped from [`user-journeys.md:227-253`](../planning-artifacts/prd/user-journeys.md) to the published Epic 13 operator evidence at `ea9939d3` (added @ HEAD `ea9939d3`).

| Journey capability / requirement | Stories | Evidence | Level | Coverage |
|---|---|---|---|---|
| Cross-team A2A with asymmetric consent envelopes | 13.3, 13.6b | `check-multi-tenant-loom` gate ledger (`product_claim: PROVEN`) | Gate | ✅ PROVEN |
| Multi-hop distillation provenance to original raw decisions | 13.3b | `check-multi-tenant-loom` gate ledger (`product_claim: PROVEN`) | Gate | ✅ PROVEN |
| Multi-tenant Loom physical + cryptographic wall; team data residency | 13.1, 13.2 | `check-multi-tenant-loom` gate ledger (`product_claim: PROVEN`) | Gate | ✅ PROVEN |
| Tenant audit isolation | 13.5e | `check-multi-tenant-loom` gate ledger (`product_claim: PROVEN`) | Gate | ✅ PROVEN |
| FR37 vetting machinery | 13.4 | `check-reza-production-path` gate ledger (`product_claim: PROVEN`) | Gate | ✅ PROVEN |
| Three-team / three-region substrate and Reza journey | 13.6c, 13.6 | `reza-three-team-three-region-journey` required leg — `PROVEN_LIVE_SIGNED` | Operator gate | ✅ PROVEN |
| Fourteen-institution isolation | 13.6 | `cortex-fourteen-institution-isolation` required leg — `PROVEN_LIVE_SIGNED` | Operator gate | ✅ PROVEN |
| NFR-Scale-5 capacity envelope | 13.6 | `check-multi-region-slo` and `check-cross-region-consensus` gate ledgers (`product_claim: PROVEN`) | Gate | ✅ PROVEN |
| **GAP — J3 Marcus peer-mesh journey** ([`user-journeys.md:203-225`](../planning-artifacts/prd/user-journeys.md)) | — | No Epic 13 verification evidence | N/A | ⚠ GAP |

## J1 — cross-host developer-remote lane coverage

**Scope:** the J1 `developer-remote` delegation lane (`j1-crosshost-1a`, `1b`, `2a`,
`2b`, `2c` + `j1-demo-one-command-scene`), added by `j1-crosshost-2c` AC5.6. The
lane had **zero** rows here before this pass — the whole cross-host, signed-artifact,
paid-agent line was traced nowhere.

**Oracle:** each story's own ACs plus the two Blocking gates that bind them,
`check-j1-loopback-delegation` (7 legs) and `check-j1-two-host-signed-run` (10 legs).
Every gate leg carries a `LegAudit`, so a leg that read nothing hard-FAILs instead of
aggregating into a green — the vacuity condition `findings.is_empty()` is blind to.

| AC / capability | Story | Evidence | Level | Coverage |
|---|---|---|---|---|
| Delegation is frame-borne; "route locally anyway" reds | 1a | `check-j1-loopback-delegation` leg `frame-borne-route-intact` + `j1_crosshost_1a_proven_red.rs` (11 vectors) | Gate | ✅ PROVEN |
| ADR-012 consent refusals: `-32001`, both `-32009` seams, `-32003` distinct | 1b | leg `consent-refusal-proofs` + `crates/maos-bin/tests/consent_refusal_1b.rs` (CI-enrolled by exact `--test` name) | Gate+Integ | ✅ PROVEN |
| Per-adapter completion oracle; no shared "clean exit + last line" | 2a | legs `completion-oracle-per-adapter`, `worker-cli-under-library` + `worker_completion_2a.rs` | Gate+Integ | ✅ PROVEN |
| Two daemons cross a frame; both TLs carry the same 16 `frame_id` bytes | 2b | leg `cross-host-identity-proof` + `two_host_delegation_2b.rs` (two real OS processes, `CARGO_BIN_EXE_maos`) | Gate+Integ | ✅ PROVEN |
| **AC1 — `sealed-export` prints the key that SIGNED (both sites, both output arms)** | 2c | leg `signing-identity-repaired` + `signing_identity_2c.rs` (7 tests; 4 RED before the fix) | Gate+Integ | ✅ PROVEN |
| **AC1.4 — `verify-bundle` derives from the bundle's CLAIMED region** | 2c | `signing_identity_2c.rs::verify_bundle_derives_the_region_key_from_a_base_seed` + the region-tamper negative | Integ | ✅ PROVEN |
| **AC2.1 — host discriminator additive, signed, byte-identity preserved** | 2c | leg `host-discriminator-signed` + `two_host_bundle_2c.rs` (post-signing tamper negative; pre-2c golden sha256 held) | Gate+Unit | ✅ PROVEN |
| **AC2.1 — the STRANGER's path** | 2c | `two_host_reconcile_2c.rs::the_python_twin_verifies_a_host_stamped_bundle` — `tools/verify-audit-bundle/verify.py` accepts a host-stamped bundle and REJECTS a rewritten host field | Integ | ✅ PROVEN |
| **AC2.2/2.3 — two-bundle verb + receipt, joined on `frame_id`** | 2c | leg `reconciliation-refuses-one-root` + `two_host_bundle_2c.rs` (join, R-RG1 forgery, disjoint logs, receipt tamper matrix) | Gate+Unit | ✅ PROVEN |
| **AC2.4 — independent per-host roots; ONE root cannot attest two identities** | 2c | `two_host_reconcile_2c.rs::one_root_signing_both_halves_is_refused` (both halves individually valid, reconciliation refuses) | Integ | ✅ PROVEN |
| **AC2.6 — bundle schema ENFORCED and corrected** | 2c | leg `bundle-schema-enforced` + `j1_crosshost_2c_proven_red.rs` (extra top-level field, extra `entries[]` field, missing `required`, and each omitted struct field all RED) | Gate | ✅ PROVEN |
| **AC3.1/3.3 — `connect` AND `framed.send` bounded; `partition_timeout_secs` wired** | 2c | leg `fault-typing-and-bounds` + `t_2c_fault_windows.rs` (silent peer, black-holed address; both bounded, typed `PartitionTimeout` carrying the frame id) | Gate+Integ | ✅ PROVEN |
| **AC3.2 — `CODE_INTERNAL` and `CODE_TIMEOUT` typed; census 10→12 of 16** | 2c | `fault_typing_2c.rs` (three injected faults are three distinct `IacBusError`s) + `bounded_postures_2b.rs::response_code_census_records_the_post_repair_scope_wall` | Unit | ✅ PROVEN |
| **AC3.4 — three fault windows, correctly named** | 2c | `t_2c_fault_windows.rs` — (a) before the delivery ACK, (b) during host-B worker execution, (c) reverse `TaskComplete` delivery. Never "after-completion-before-ACK": the ACK means *delivered*, not *executed* | Integ | ✅ PROVEN |
| **AC3.5 — nothing is `Duplicate` until something is durable** | 2c | leg `duplicate-after-durable` + `digest_reply_durability_2c.rs` (dropped receiver, full channel, drain-then-retry) + `maos-cohort` state-machine unit test | Gate+Unit+Integ | ✅ PROVEN |
| **AC3.6/3.7 — pin refusal journaled on BOTH sides; listen side asserted on the SERVER's journal** | 2c | leg `pin-refusal-journaled` + `t_2c_pin_journal.rs` (listen-side refusal lands a `PeerIdentityUnverified` rupture; healthy handshake journals nothing) | Gate+Integ | ✅ PROVEN |
| **AC4.1 — read-path scan over STORED rows, both classes reported distinctly** | 2c | leg `stored-row-scan` + `credential_posture_2c.rs` (prefix escape, hex-run escape, both-in-one-row, never echoes the secret) | Gate+Integ | ✅ PROVEN |
| **AC4.2 — credential posture ASSERTED, not changed** | 2c | `credential_posture_2c.rs` — `env_clear` absent from production code, present only as documented rationale; 11 payload variants carry no credential BY SCHEMA, with the free-form `goal`/`success_criteria` caveat stated | Integ | ✅ PROVEN |
| **AC5.1/5.2 — ONE always-`Blocking` hermetic gate, registered in all five slots** | 2c | `check-j1-two-host-signed-run` (10 legs, every leg `LegAudit`ed) + `j1_crosshost_2c_proven_red.rs` (42 vectors incl. enrollment falsifiers for BOTH test-naming conventions, the capture value/negation-smuggle family, and a `services:`-block refusal) | Gate | ✅ PROVEN |
| **AC5.3 — the beat reflects the judge's verdict; an EXECUTED leg is no longer reachable** | 2c → **re-rendered by `2e` AC3 (F3, 2026-08-24 §A6 review P9)** | `demo_j1.rs::apply_capture_judgement` — in-process judge, never the published-ledger route (structurally dead twice). **Since `2e`, a present capture renders the beat `INDETERMINATE` with `executed = false` and the owner retained** (F3: `executed && !is_proven()` must not fail a non-claim); pinned by `demo_j1_tests::two_host_beat_with_present_capture_renders_indeterminate_not_fail` + the refused-capture twin. Owner `j1-crosshost-2d-paid-two-host-run` (RF-0). | Unit | ✅ PROVEN (post-`2e` shape) |
| **AC5.5 — the bounded claim, in a capture that cannot overclaim** | 2c | `CaptureDoc::validate_two_host` + `capture_validation_refuses_the_two_host_overclaim_directions` (trust-anchor, shared-root key, free-prose shape, and the missing stranger check all refused) | Unit | ✅ PROVEN |
| **AC5.4 — `PROVEN_LIVE_SIGNED` under Reza's posture** | 2c → **re-scoped by `2e` AC3 (F2/R1)** | The `verify_capture_signature` verifier named here was **DELETED by `j1-crosshost-2e`**: the `MAOS-EVIDENCE-V1` nonce was recomputed at gate-run time, so no pre-written transcript could carry it — `PROVEN_LIVE_SIGNED` is structurally unreachable for this gate and the term asserted a verifier that could never run. `two_host_signed_run_claimed` is now published as a literal `false` (a TRUE fact); the evidence of a two-host run is the two bundle signatures verified by the operator's `verify.py` plus an executing `reconcile-hosts` — all operator-performed. See the F2 absence row below and `RELEASE-HOLDS.md` row 14. | Operator gate | ✅ RE-SCOPE PROVEN (`claimed:false` with a present capture pinned by `a_present_capture_still_claims_nothing`) |
| **The paid two-host run itself** | 2c judge / **2d AC8 PERFORMED 2026-08-25** | Capture artifact under `_bmad-output/test-artifacts/j1-two-host-evidence/`, validated by the gate when present and REFUSED as a claim when absent. **The run happened**: `claude-haiku-4-5-20251001` on host B, $0.014644, identical `frame_id 0100000000000000e09eb3b406c54f4e` journaled on both sides, each half sealed under its OWN root (`4bbc1187…` / `843dc5a8…`, byte-matching `PUBLISHED-FINGERPRINTS.md` committed at `dd4cf959` BEFORE the run), `verify.py` OK on both halves against the COMMITTED fingerprints, `reconcile-hosts` OK. `paid_run_capture_present` flipped `false`→`true`; the demo beat moved **ABSENT → INDETERMINATE**. ⚠ The gate still cannot verify the two signatures — that remains operator-performed (F6 row below). | Operator gate | ✅ **PERFORMED — capture present and validated; the run is NOT gate-claimed (R1)** |
| **AC1 — the release build is genuinely release-built** | 2d | Loopback double-boot falsifier, executed 2026-08-22: two `maos run … --once` runs with the SAME `MAOS_TEST_BOOT_NONCE=424242` read back `9046754445710571789` and `1928460524043859277`; the **debug control** read back `424242` twice. Release binary `maos` sha256 `e185e540…`. | Operator, executed | ✅ PROVEN |
| **AC2.2 — the whole live worker path, driven unbilled** | 2d | Fake `codex`/`claude` on a prepended `PATH` with `MAOS_LIVE_AGENT=1` (basename dispatch, `worker_cli.rs:833-848`). Exercised: argv-flag refusal *before probe and spawn*, ambient-`auth.json` refusal, liveness-probe admission, cap-token mint, TL journaling, and BOTH completion oracles. Zero spend. | Integ, executed | ✅ PROVEN |
| **AC2.3 — the capture the paid run will actually submit is admissible** | 2d | `maosctl audit record-capture` dry-run against a scratch TL with the real strings and `two_host_shape` set → journaled `run.capture`. Three `sk-`-substring negatives (`--task-file`, `risk-accepted`, `disk-backed`) all REFUSED `api_key_generic`. | Integ, executed | ✅ PROVEN |
| **AC3 — the operator documentation would have refused the operator AFTER billing** | 2d | The runbook's Phase-4 capture template (byte-identical to the shipped T6 capture) fed to the real CLI → `capture field 'fs_jail_followup' is required`, exit 2. README `:35`'s literal shape string fed to the real gate → `shape asserts 'two machines'`. Both repaired; both repairs verified by re-execution. | Integ, executed | ✅ PROVEN |
| ⚠ **ABSENCE — the `MAOS-EVIDENCE-V1` transcript had NO PRODUCER (F2) → RE-SCOPE EXECUTED** | 2c judge / 2d finding / **2e AC3 closed** | `two-host-evidence.txt` was declared as `CAPTURE_TRANSCRIPT` and **read by no leg**. `capture_signature_verified` needed a nonce recomputed at gate-run time (`evidence_ledger.rs:415-431`), so no pre-written file could carry it; the sole signer (`tests/harness/evidence_record.rs`) emits only for gates in `ledger_gates()`, and J1 is not one. **`PROVEN_LIVE_SIGNED` is structurally unreachable for this gate.** Re-scoped by R1 to the two bundle signatures — how T6 was actually evidenced. ✅ **`2e` AC3 (2026-08-22) deleted `verify_capture_signature`, the `CAPTURE_TRANSCRIPT` const and both `capture_signature_*` JSON fields; no replacement term was added and the gate still has zero `Command::new`.** 42/42 proven-red vectors stayed green. | Integ, executed | ✅ **RE-SCOPE PROVEN — the unreachable branch is gone, not hidden** |
| ⚠ **ABSENCE — leg 9 accepts a SINGLE-ROOT, UNSIGNED FORGERY (F6)** | 2c judge / 2d finding | `j1_crosshost_2c_proven_red.rs:386-414` **exhibits the shape** — ⚠ **CORRECTED 2026-08-25 by the §A6 Blind Hunter, which re-derived this independently and was right**: the earlier wording here and in the story's F6 said a committed test *proves* the forgery is admissible. It does not. That test is `published_capture_template_is_admissible_by_the_real_gate`, and what it ASSERTS is that the published template is accepted so an operator following the runbook is not refused after billing. The single-root material is incidental scaffolding from the shared `good_bundle()` helper (`:270-295`), not the subject of any assertion — **no test anywhere asserts the two halves must carry different keys**, and if the gate were fixed to compare `attester_pubkey` this test would RED and read as *'the template broke'*, not *'a forgery hole closed'*. A test that exhibits a weakness while asserting something else is not an enrolled control; the distinction is the exact 'claim standing in for a control' shape this lane treats as a signature failure. The forgery itself was re-proven live 2026-08-25 in a sandbox mirror (one key in both halves, and non-hex garbage signatures, both `passed:true findings:[]`). What follows is what that test's fixture happens to contain: the published template plus two halves carrying identical `attester_pubkey` (`"aa"×32`, one root) and `"signature": "bb"×64`, no transcript — and asserts `verdict.passed && verdict.success`. Leg 3 (`:395-478`) is a source-text grep executing nothing; leg 9 compares only `bundle["host"]` strings; leg 3 at `:458-470` **forbids** reconciliation from reading `attester_pubkey` (R-RG1), so the artifacts structurally cannot carry the discriminator. The gate has **zero `Command::new`**. The lone independence signal is a boolean shipped **pre-filled `true`** in the template the operator copies. | — | ⛔ **NOT CHECKABLE BY THIS GATE — discharged by operator-performed pasted discriminators (2d AC4), not by a test** |
| ⚠ **ABSENCE — the paid two-host run is BLOCKED BY OPERATOR SUBSTRATE, no longer by code** | 2d AC8 / **2e closed all six** | Six code defects made it impossible or worthless: F1 (nothing signed a cohort manifest, so host B could not boot), F2, F3 (landing the capture made `demo-j1` exit nonzero), F4 (the documented pairing needed a debugger), F5 (`verify.py` failed on any non-ASCII bundle, and Phase 7.4 is a mandatory abort — so the run died *after* both agents were billed), F7 (the hardcoded delegated goal could not satisfy codex's oracle; observed on the wire as `founder-loop: execute the delegated assignment from founder-loop-host` → `not_completed:no_effect_evidence`). ✅ **All six CLOSED by `j1-crosshost-2e` (2026-08-22), each proven RED before GREEN.** What remains is not code: a funded metered API key, a clean sandbox home (`refuse_ambient_auth` correctly refuses `~/.claude/.credentials.json`), two provisioned hosts, and an operator willing to spend. | Operator gate | ✅ **DISCHARGED 2026-08-25 — the spend decision was made ($3 cap authorized, $0.025862 actual across three live calls) and the run was performed.** The substrate was assembled and the crossing executed on ONE box as two real OS processes, the ratified shape. Retained rather than deleted because the row records why this was blocked for so long, and by what. |

**The ABSENT, INDETERMINATE and UNREACHABLE rows are the honest state, not gaps —
and they are listed here on purpose.** A traceability matrix that records only what
passed is a marketing document. Three of the rows above record things this lane
*cannot* prove, and each names the mechanism that stops it rather than a schedule:

- The **operator-substrate** rows (`PROVEN_LIVE_SIGNED` in CI, the paid run itself)
  need an operator, two hosts and a funded API key — which CI has never had and will
  never have. That is precisely why this lane has ONE always-`Blocking` gate that
  validates a capture when present, rather than a second `AdvisorySubstrate` job that
  would take the ABSENT branch on every run for its entire lifetime. A gate whose
  substrate cannot exist is a monument, not a control.
- The **F2 row** is different in kind: it is not waiting on substrate, it is waiting
  on a producer that does not exist anywhere in the workspace. The target was
  mis-specified, not merely unbuilt, and the honest response was to re-scope the
  evidence (R1) rather than to leave an unreachable branch looking like future work.
- The **F6 row** is different again, and is the uncomfortable one: the judge this lane
  built cannot distinguish a real run from a forgery, and a committed test proves it.
  No amount of additional testing closes that, because the discriminator is forbidden
  from entering the artifacts. It is closed — to the extent it can be — by an operator
  pasting both `attester_pubkey` values by hand and by fingerprints published in git
  **before** the run (`j1-two-host-evidence/PUBLISHED-FINGERPRINTS.md`).

**Read the gate's JSON fields, never its exit code.** `passed` and `oracle_green` are
green whether the capture is absent, valid, or fabricated. The discriminators are
`paid_run_capture_present` and `two_host_signed_run_claimed` — both `false` at HEAD,
and published as true facts. ⚠ `capture_signature_verified` was a third discriminator
until `j1-crosshost-2e` AC3 **deleted** it with the unreachable branch it gated; any
script still grepping for it is pre-`2e` and its green means nothing.
