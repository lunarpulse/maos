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
| **AC5.1/5.2 — ONE always-`Blocking` hermetic gate, registered in all five slots** | 2c | `check-j1-two-host-signed-run` (10 legs, every leg `LegAudit`ed) + `j1_crosshost_2c_proven_red.rs` (33 vectors incl. enrollment falsifiers for BOTH test-naming conventions and a `services:`-block refusal) | Gate | ✅ PROVEN |
| **AC5.3 — beat flipped by an EXECUTED leg** | 2c | `demo_j1.rs::apply_two_host_signed_run` — in-process judge, never the published-ledger route (structurally dead twice). Owner string VERIFIED, not edited | Unit | ✅ PROVEN |
| **AC5.5 — the bounded claim, in a capture that cannot overclaim** | 2c | `CaptureDoc::validate_two_host` + `capture_validation_refuses_the_two_host_overclaim_directions` (trust-anchor, shared-root key, free-prose shape, and the missing stranger check all refused) | Unit | ✅ PROVEN |
| **AC5.4 — `PROVEN_LIVE_SIGNED` under Reza's posture** | 2c | `check_j1_two_host_signed_run::verify_capture_signature` — `MAOS_AUDIT_KEY` (a PATH), `MAOS-EVIDENCE-V1` bound to commit+nonce, `verify_release_signature`. **CI holds no operator key by ratified design, so in CI this leg is `INDETERMINATE`; the operator lane produces the signed claim.** | Operator gate | ⚠ INDETERMINATE in CI — by design |
| **The paid two-host run itself** | 2c | Capture artifact under `_bmad-output/test-artifacts/j1-two-host-evidence/`, validated by the gate when present and REFUSED as a claim when absent | Operator gate | ⚠ ABSENT until the operator run |

**The two ABSENT/INDETERMINATE rows are the honest state, not gaps.** Their
substrate is an operator, two hosts and a funded API key — which CI has never had
and will never have. That is precisely why this lane has ONE always-`Blocking`
gate that validates a capture when present rather than a second
`AdvisorySubstrate` job that would take the ABSENT branch on every run for its
entire lifetime. A gate whose substrate cannot exist is a monument, not a control.
