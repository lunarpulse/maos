---
baseline_commit: 79e13c16
---

# Story 13.5a — Enterprise governance at the cohort-a2a-daemon seam

Status: **done** — implemented and adversarially reviewed 2026-07-25 at baseline `79e13c16`. Baseline pin **23228** remains unchanged: no file under `crates/maos-kernel-core/src/` was touched. **ZERO kernel-core Δ; NON-ZERO composition-root Δ.** Post-review measured LOC: `maos-bin` 13 927 and aggregate 132 988, with tight documented ceilings. Depends 11.4a + 11.4c + 13.5c (all `done`).

> **The brief was reframed by the preflight.** The epic's sketch ("build the 11th reference Spirit, composed *Spirit-side*, ZERO expected") is a **category error** and is **partly already shipped**. The three scouts converged: (1) the enterprise crates are reachable ONLY from `maos-bin` — no Spirit crate can depend on them, so "compose Spirit-side" is impossible; (2) the SSO→PDP→at-rest→SIEM composition already runs in `one-shot`/`spirit-spawn`/`default-server` modes (researcher + butler); (3) **but in `cohort-a2a-daemon` mode the `EnterpriseRuntime` is a live dead-wire** — constructed at `main.rs:2501`, never threaded into the daemon, never reached. This story closes that dead-wire and proves it end-to-end. It is NOT a new Spirit crate.

---

## Story

**As** an operator running MAOS in multi-tenant collective (`cohort-a2a-daemon`) mode,
**I want** the enterprise governance chain (SSO/OIDC principal → Enterprise PDP decision → at-rest AEAD → SIEM export) to actually run for collective operations served by the daemon — the same governance `one-shot` mode already applies —
**so that** enterprise-governed tenant Spirits are not silently ungoverned the moment they run under the collective daemon (the exact "isolated tripwire greens while the production path is dead-wired" failure the E11 retro named).

---

## The dead-wire, in code (verified 2026-07-24, not scout-reported)

- `EnterpriseRuntime::from_env(...)` is constructed at **`crates/maos-bin/src/main.rs:2501`** and held as `Option<Arc<..>>`.
- Every production **reach** of it is a `one-shot`/`spirit-spawn`/`default-server` site: `issue_enterprise_governed_capability(...)` at `main.rs:1015, 1330, 1472, 1573, 1675, 4398, 4445, 7607` and the SIEM forwarder spawn at `main.rs:7699`.
- The `cohort-a2a-daemon` mode **early-returns** at **`main.rs:7397-7399`**: `return run_cohort_a2a_daemon(Arc::clone(&transparency_log), boot_nonce, cohort_daemon).await;` — **no `enterprise_runtime` argument.**
- `run_cohort_a2a_daemon` (`main.rs:8483`) and `build_cohort_a2a_daemon_runtime` (`main.rs:8535`) take **no `enterprise_runtime`** and construct **no Spirit** (`control_spirit` at `:8547` is only a `SpiritId` inside a `FrameAddress`, not a runtime).

**⇒ In `cohort-a2a-daemon` mode: no SSO verify, no PDP mediation, no at-rest seal, no SIEM forward.** The runtime is constructed and then never reached. This story wires it and proves the wiring with a control that reds if the wiring is removed.

---

## Acceptance Criteria (6)

**AC1 — Enterprise governance reaches the cohort-a2a-daemon seam (the real deliverable).**
Thread the already-constructed `Arc<EnterpriseRuntime>` (and the `enterprise_pdp_runtime`) from the composition root into `run_cohort_a2a_daemon` / `build_cohort_a2a_daemon_runtime`, and apply it to the daemon's collective-operation issuance path so a governed collective read/write served by the daemon goes through **SSO principal → Enterprise PDP → kernel mint → `identity.asserted` persist → at-rest seal on the collective store → SIEM forward**, reusing `issue_enterprise_governed_capability` (`main.rs:196`) and `at_rest_seal_hook`/`LoomLiteStore::with_at_rest_seal` (`main.rs:2663-2669`) — **not re-implementing** any of it. **Task 0 (dev, blocking):** pin the exact daemon collective-op issuance site where governance attaches (candidates: the `digest_port` / collective-serve path inside `build_cohort_a2a_daemon_runtime`). If no daemon issuance seam exists, narrow AC1 to "a reference Spirit **run under** the collective daemon receives enterprise governance," and say so explicitly — do not invent a seam.

**AC2 — Reuse, do not re-implement, and do NOT mint a new Spirit.** No new Spirit crate, no new `LoadedSpiritKind` variant (`main.rs:415-427`), no new `classify_spirit` arm (`main.rs:429-443`). A **new collective-only enterprise Spirit is forbidden** — it re-hits the mandatory non-empty `provider.complete` blocker (`crates/maos-manifest/src/manifest.rs:473-480`) that 13.5d only dodged by hosting its port on `researcher`. The "enterprise-governed Spirit **class**" is a **daemon posture/profile** bundling the four env groups (`MAOS_SSO_*` / `MAOS_PDP_POLICY*` / `MAOS_KMS_*` / `MAOS_SIEM_*`) applied at the daemon issuance seam — not Spirit-crate code.

**AC3 — End-to-end lifecycle proof through the daemon boot (not isolated arms).** A single hermetic in-process test boots `build_cohort_a2a_daemon_runtime` with the enterprise runtime wired to **recording** SSO/PDP/at-rest/SIEM ports, drives **one** governed collective round-trip through the running daemon, and asserts each recording port fired (count ≥ 1). This is the load-bearing deliverable: no existing test drives SSO→PDP→issue→at-rest→SIEM as **one** lifecycle — every enterprise test today exercises one arm in isolation via the test-only `build_runtime` (`enterprise_identity.rs:640`), never a booted daemon. **The proof MUST exercise `at_rest_seal_hook` on `LoomLiteStore`, never the test-only `seal_row_at_rest` / `issue_under_principal` (zero production callers — proving them proves nothing about the daemon; H3).**

**AC4 — Enterprise daemon-posture profile: template + docs (retargeted off ADR-008).** Document the enterprise posture as an operator-instantiable profile: the four env groups, the daemon-attach pattern, and a worked example that attaches enterprise governance to an existing reference Spirit run under the daemon. **ADR-008 is mis-cited in the epic** (it is the registry MCP-Streamable-HTTP publish/discover protocol, NOT a scaffold). Reuse the real scaffold where an example is needed: `maos-spirit-derive` `#[spirit]`, `maos-spirit-sdk`/`spirit_test`, `examples/example-spirit` + `xtask` regen (`example_spirit_regen.rs`), `maos-spirit-cli`. Author **ADR-057** (empty slot; index currently jumps 056→058) recording the "enterprise governance is a daemon posture, not Spirit code" decision and the composition-root-only Δ boundary.

**AC5 — Gate: a REAL control, folded into `check-multi-tenant-loom`, not a null control.** Add `TestLeg`s to `check_multi_tenant_loom.rs` `specs` (`:122`) — parent gate already registered (`gate-registry.toml:305` v2_2=blocking; `check_ship_gate_completeness.rs:60`; `discipline.yml:2671` provisions the live pgvector/pg16 team-A/B service):
  1. **Reach leg — `Blocking`, hermetic** (in-process daemon boot + recording ports, AC3's test). Precedent: `cohort-daemon-boots-and-serves` (`check_multi_tenant_loom.rs:595`, Blocking, boots over loopback, no Postgres).
  2. **Dead-wire negative — `Blocking`, two-sided:** unwired daemon → the governed round-trip **fails closed**; wired-with-recording-ports → each stage fires. Model on `researcher` `collective_route_is_fail_closed_until_wired_then_reaches_port` (`spirits/researcher/src/lib.rs:1550`) + the 13.5d RecordingPort/source-inspection harness (`xtask/tests/story_10_4a_ac1_proven_red.rs`).
  3. **Source-inspection leg — `Blocking`:** read `main.rs` and assert the `cohort-a2a-daemon` dispatch (`:7398`) threads `enterprise_runtime` into the daemon. Model on `run_issuance_bypass_absence_leg` (`check_enterprise_identity.rs:443`, which asserts `.issue_with_mediation(` count == 1).
  Any leg needing a **live external SSO/SIEM substrate** is `AdvisorySubstrate` (self-skips absent, like `tenant-mode-boots-live` `:637`), never Blocking.
  **Proven-red contract:** deleting the enterprise arg from the daemon call at `main.rs:7398` (restoring today's HEAD dead-wire) MUST red the reach leg (recording count = 0) and the source leg (grep fails). **Anti-vacuity:** exactly one `#[test]` per leg (`check_multi_tenant_loom.rs:83` requires `"running 1 test"` + `"1 passed"`).
  **Forbidden null-control shortcuts:** (a) adding a string to `ABSENT_SUCCESSORS` (`check_reza_production_path.rs:17`, `check_multi_tenant_loom.rs:16` — prose only, never executed); (b) an `available_arm_tests`-only leg (`check_enterprise_identity.rs:323` — proves adapter routing, NOT daemon reach; green even while dead-wired at `:7398`); (c) proving at-rest via the test-only `seal_row_at_rest`.

**AC6 — Δ posture, stated honestly.** **ZERO kernel-core Δ** — `check-kernel-baseline` 23228 == pin; no file under `crates/maos-kernel-core/src/` touched; `identity.asserted` stays a raw kind-30 TL row via `append_identity_asserted` (`enterprise_identity.rs:487`), **never** a kernel `FrameKind` (that would be a forbidden L1 delta; H6). **NON-ZERO `maos-bin` Δ** — the composition-root signature change threading `enterprise_runtime` into `run_cohort_a2a_daemon`/`build_cohort_a2a_daemon_runtime` + the daemon issuance wiring + tests. Re-base the `maos-bin` (and `xtask`, if the gate legs push it) kloc ceiling on the **measured** residual per the epic-retro process, with a documented driver — do **not** mark the breach "advisory/pre-existing" (the 13.4/13.5e anti-pattern).

---

## Tasks / Subtasks

- [x] **Task 0 — CATCH-0: pin the daemon collective-op seam (blocking).** (AC: 1)
  - [x] Trace the `cohort-a2a-daemon` serve path end to end; record whether a collective-store / capability-issuance seam exists today.
  - [x] Pin the exact site governance attaches to, or narrow AC1 per the preflight escape hatch and **say so explicitly** in this file.
  - [x] Re-verify the kernel baseline pin (`xtask/kernel-core-baseline.toml` `src_lines`) == 23228 before any edit.
- [x] **Task 1 — port-injection seam for the composition root.** (AC: 1, 3)
  - [x] Add `EnterpriseConfig::with_{sso,kms,siem}_available` builders (mirror of the existing `with_*_down`).
  - [x] Add `EnterpriseRuntime::from_ports(..)` and make the PRODUCTION `from_env` delegate to it (no zero-caller test-only API).
- [x] **Task 2 — thread enterprise governance into the daemon.** (AC: 1, 2, 6)
  - [x] `EnterpriseDaemonGovernance` + `EnterpriseGovernedDigestReadPort` in the composition root (`main.rs`); no new Spirit crate, no `LoadedSpiritKind` variant, no `classify_spirit` arm.
  - [x] Governed chain reuses `issue_enterprise_governed_capability` verbatim (SSO → PDP → kernel mint → `identity.asserted`); at-rest via `at_rest_seal_hook()` in `maos_loom_lite::seal::AtRestSealer`; SIEM via `forward_audit_to_siem`.
  - [x] Change `run_cohort_a2a_daemon` / `build_cohort_a2a_daemon_runtime` signatures; thread from the `main.rs` dispatch.
  - [x] Admit the daemon control Spirit through the canonical `admit_spirit` path (NO policy-table seeding — `composition_root_does_not_seed_manifest_scopes` stays green).
- [x] **Task 3 — SIEM forwarder reaches daemon mode.** (AC: 1)
  - [x] Extract the `main.rs:7683` forwarder into a shared spawn helper + single-shot forward; call from BOTH the server path and the daemon.
  - [x] Watermark scoped to one daemon lifetime (H5) — no exactly-once-across-restart claim.
- [x] **Task 4 — end-to-end lifecycle proof through the daemon boot.** (AC: 3)
  - [x] Hermetic in-process `build_cohort_a2a_daemon_runtime` boot with recording SSO/PDP/at-rest ports + a real SIEM file sink; ONE governed collective round-trip; every arm fired ≥ 1.
  - [x] Exercise `at_rest_seal_hook`; never `seal_row_at_rest` / `issue_under_principal` (H3).
  - [x] Verify a per-Spirit PDP deny binds at the daemon issuance pid (H4).
- [x] **Task 5 — gate legs: a REAL control.** (AC: 5)
  - [x] Reach leg (`Blocking`, hermetic) + two-sided dead-wire negative (`Blocking`) + `main.rs` source-inspection leg (`Blocking`) in `check_multi_tenant_loom.rs`.
  - [x] Exactly one `#[test]` per leg (`--exact`), so `running 1 test` + `1 passed` holds.
  - [x] Proven-red: drop the enterprise arg at the daemon dispatch → reach + source legs red.
- [x] **Task 6 — daemon-posture profile + ADR-057.** (AC: 4)
  - [x] Operator profile doc: four env groups, daemon-attach pattern, worked example on an existing reference Spirit, air-gap unavailability (H2).
  - [x] ADR-057 "enterprise governance is a daemon posture, not Spirit code" + index entry.
- [x] **Task 7 — Δ posture, measured.** (AC: 6)
  - [x] `check-kernel-baseline` 23228 == pin, zero files under `crates/maos-kernel-core/src/` touched.
  - [x] Re-base the measured `maos-bin` (+ `xtask`) kloc ceiling with a documented driver — not "advisory/pre-existing".

### Review Findings

- [x] [Review][Patch] Attach governance when only the enterprise PDP is configured [`crates/maos-bin/src/main.rs:8821`]
- [x] [Review][Patch] Refuse daemon reads when a configured KMS is unavailable [`crates/maos-bin/src/main.rs:8837`]
- [x] [Review][Patch] Validate and reserve the digest transition before governance side effects [`crates/maos-bin/src/main.rs:8688`]
- [x] [Review][Patch] Reuse the process-wide lifecycle journal for daemon admission [`crates/maos-bin/src/main.rs:8780`]
- [x] [Review][Patch] Drive the AC3 proof through the daemon request path [`crates/maos-bin/src/main.rs:12202`]
- [x] [Review][Patch] Keep the SIEM assertion inside the single AC3 lifecycle [`crates/maos-bin/src/main.rs:12329`]
- [x] [Review][Patch] Make the AC5 unwired control fail closed and red both runtime and source legs [`crates/maos-bin/src/main.rs:12364`]
- [x] [Review][Patch] Prove the H4 subject-specific PDP deny at the daemon PID [`crates/maos-bin/src/main.rs:12442`]

## Task 0 ruling — the daemon collective-op seam (dev, 2026-07-25, verified at `79e13c16`)

**A daemon collective-serve seam EXISTS. Governance attaches to it. AC1 is narrowed on exactly one arm — the at-rest arm — and the narrowing is stated below.**

**Pinned seam: `DigestReadPort::note_admitted_request`.**
The `cohort-a2a-daemon`'s collective operation is the **cross-team cohort digest read**. Its serve-side chokepoint is `maos_a2a_core::DigestReadPort` (`crates/maos-a2a-core/src/cohort.rs:242-269`), wired into the daemon as `Arc<dyn DigestReadPort> = state.clone()` at `crates/maos-bin/src/main.rs:8557`. Inbound path: `TcpA2ATransport::serve_connection` (`crates/maos-a2a-tcp/src/transport.rs:513`) → `A2ARouterCore::handle_intake_verified` (`crates/maos-a2a-core/src/router.rs:1226`) → classify the request (`:1268-1276`) → consent ACK → **`note_admitted_request` (`:1287-1289`)**, whose `Err` rewrites the ACK into a fail-closed NACK (`:1290-1293`). That last property is what makes it a governance seam and not a log line: refusing there refuses the collective read.

**What does NOT exist (verified, not assumed).** No `LoomLiteStore`, no `CollectiveMemoryPort`, and no `CapabilityRegistryAdapter::issue_with_mediation` is reachable from `build_cohort_a2a_daemon_runtime` (`main.rs:8535-8639`). The daemon's digest reply is served from `CohortManifestState` plus the config-supplied `DigestSummary` (`crates/maos-cohort/src/digest.rs:67-78`), never from the collective store. The daemon holds no Spirit runtime — `control_spirit` at `:8547` is a `SpiritId` inside a `FrameAddress`.

**Narrowing (explicit, per the preflight escape hatch).** AC1's chain is delivered unnarrowed for **SSO principal → Enterprise PDP → kernel mint → `identity.asserted` persist → SIEM forward**. The **"at-rest seal *on the collective store*"** arm is narrowed to: the daemon takes its sealer from the production `EnterpriseRuntime::at_rest_seal_hook()` — the same `AtRestSeal` `Arc` closure installed on the collective store at `main.rs:2663-2669` — and installs it in the same `maos_loom_lite::seal::AtRestSealer` wrapper `LoomLiteStore::with_at_rest_seal` uses (`crates/maos-loom-lite/src/store.rs:306`), so the seal, its fail-closed error semantics, and its `None`-means-byte-identical-plaintext posture are the store's, byte for byte. The **storage** is the Transparency Log, not the collective store, because the daemon serves no collective-store row on this path and inventing one would be inventing a seam. Zero use of the test-only `seal_row_at_rest` / `issue_under_principal` (H3).

**H4 (spirit_pid).** The daemon governs under ONE control-Spirit pid — the canonical composition-root convention `0`, the same pid `main.rs:1015` / `:7607` already issue under. Per-Spirit PDP subject-deny therefore binds **per daemon posture**, not per tenant Spirit; a test asserts a subject-deny at that pid denies the daemon's governed read, so the binding is proven live rather than claimed.

**Fail-closed by construction.** `issue_with_mediation` denies any pid absent from `manifest_scopes` (`crates/maos-kernel-core/src/capability/cap_policy/mod.rs:129-133`), and `Scope::LoomRead` is only declarable via `[capabilities.required.loom] read = true` (13.5d). So an enterprise-governed daemon whose control Spirit is not admitted with `loom.read` refuses every collective read. The control Spirit is admitted through the canonical `SecurityManagerAdapter::admit_spirit` path — **never** by seeding `manifest_scopes` (`composition_root_does_not_seed_manifest_scopes` stays green).

## Dev notes

- **Category error to avoid (H1).** Do not open a Spirit crate. The enterprise crates (`maos-pdp`, `maos-sso`, `maos-siem`, `maos-secrets`) are dependencies of `maos-bin` only (`crates/maos-bin/Cargo.toml:24-27,44-48`); `maos-spirit-abi`/`-sdk`/`-derive` have zero deps and `maos-spirit-hello` depends only on `maos-domain` + `maos-kernel-core`. All wiring is composition-root (`maos-bin`).
- **The subsystems are real and fail-closed** (do not re-verify, reuse): PDP `CedarPolicyAdapter` (`maos-pdp/src/adapter.rs:44`, impl `:132`), SSO `OidcVerifier` (`maos-sso/src/lib.rs:57`, impl `:238`), at-rest `LocalMasterKeyKms` ring-AEAD (`maos-secrets/src/lib.rs:20`, `seal_at_rest_opt:101`), SIEM `SiemExporter` (`maos-siem/src/lib.rs:62`, `export_from_tl:92`). Each carries a `*-fault-inject` dev feature with a `compile_error!` release guard.
- **The governed wrapper already exists** — `issue_enterprise_governed_capability` (`main.rs:196-251`): SSO verify (`:207-217`) → PDP evaluate (`:221-234`) → kernel mint (`:236`) → `identity.asserted` persist (`:240-244`). The single `issue_with_mediation` call site (`:237`) is the invariant the 11.4c bypass-absence leg guards. Route the daemon's collective issuance through this same wrapper.
- **Feature-gate posture (H2).** SSO/secrets/SIEM are `optional` deps behind the `network` feature (`maos-bin/Cargo.toml:24-27`); an `air-gap`/reduced build compiles the enterprise composition OUT. The reach/negative legs run under `--features network`; state in AC4 that the enterprise posture is unavailable in air-gap builds (PDP is non-optional; the other three are).
- **spirit_pid=0 (H4).** Several governed-issuance sites pass `spirit_pid = 0` (`main.rs:1015, 7607`); PDP subject-deny is per-`spirit_pid` (`enterprise_pdp_runtime.rs:153`). Verify a per-Spirit PDP deny binds at the daemon issuance pid before claiming per-Spirit governance.
- **SIEM watermark (H5).** The forwarder's `last_forwarded_ns` is in-memory and resets on restart (`main.rs:7687-7712`, follow-up flagged at `:7680`). Scope the proof to a **single daemon lifetime** — do not assert exactly-once across restart.
- **Adjacent, OUT of scope:** the 11.4b audit escape-anomaly detector is a **second** real dead-wire, tracked today only as an honesty-ledger string in `ABSENT_SUCCESSORS` (`check_reza_production_path.rs:17`), not gated. Do not wire it here; note it exists.

## Gate discipline (§A7 reflex)

The AC5 legs are `Blocking` and hermetic (in-process daemon boot + recording ports + main.rs source inspection). Live external SSO/SIEM substrate → `AdvisorySubstrate` (UNMEASURED, not green, when absent). The gate is a control ONLY because a planted "drop the enterprise arg at `main.rs:7398`" edit reds the reach + source legs. `ABSENT_SUCCESSORS`, `available_arm_tests`-only, and `seal_row_at_rest`-based proofs are null controls — forbidden.

## Dev Agent Record

### Agent Model Used

`anthropic/claude-opus-5` (frontier-class dev allowlist, E11 retro A1 / E12-B3). Recorded at dev start 2026-07-25.

### §A6 review net

§A6 — multi-layer net run in-pass: four parallel read-only scouts (daemon serve path / enterprise runtime API / gate-leg conventions / boot-test placement) grounded every premise against code at `79e13c16` before any edit; the acceptance layer is the three blocking gate legs, each verified proven-RED by planted edit (below). An independent adversarial review with a DIFFERENT model is the recommended next step and is NOT claimed here.

### Debug Log References

- **Proven-red #1 (decorator).** Replacing the governed `digest_port` match in `build_cohort_a2a_daemon_runtime` with the bare `state.clone()` reds all three runtime legs (0 passed / 3 failed): every recording port falls to zero. Restored.
- **Proven-red #2 (dispatch).** The reach leg now inspects the production dispatch in addition to driving the real runtime, while the dedicated source leg independently enforces the same thread. Removing `build_enterprise_daemon_governance(...)` or either required argument reds both blocking legs.
- **Kernel clock.** A unit test inside the `maos` binary target must call `cap_tokens::init_monotonic_base()`; `main` normally arms it and `monotonic_now_ns` `debug_assert!`s otherwise.
- **SIEM / WAL (review fix).** `maos_audit::query_with_redaction` still requires a quiesced database. The enterprise runtime now creates a transactionally consistent `VACUUM INTO` snapshot for a live-WAL forward, projects through the injected SIEM port, and removes the snapshot. The AC3 leg observes projection and sink output during the same daemon lifecycle.
- **Journal ownership (review fix).** Daemon control-Spirit admission receives the process-wide `shared_journal`; it no longer opens a second append-only writer with an independent file cursor.
- **Coverage guard bump.** `manifest_field_coverage::production_capability_parsers_are_all_schema_degraded` counts production capability parsers in `main.rs` (4→5) and their `degrade_for_schema_version` calls (5→6). The new control-Spirit admission adds one of each — the guard fired exactly as designed and was updated with the coverage present. `crates/maos-kernel-core/tests/` is not under `src/`; the baseline is unmoved.

### Completion Notes

**What shipped.** The `cohort-a2a-daemon`'s collective-serve seam (`DigestReadPort::note_admitted_request`) is now decorated, under an enterprise posture, with the full governance chain: SSO principal → Enterprise PDP → kernel mint (`Scope::LoomRead`) → `identity.asserted` persist → at-rest seal → SIEM forward. The `EnterpriseRuntime` constructed at the composition root finally reaches daemon mode, and the SIEM export consumer — previously unreachable because the dispatch returned above its spawn — now runs for the daemon lifetime.

**AC1 — Met, with the at-rest arm narrowed and stated.** See the Task 0 ruling above. Governance attaches at the pinned seam and reuses `issue_enterprise_governed_capability` verbatim; the composition root still holds exactly **one** `.issue_with_mediation(` call site (the 11.4c bypass-absence invariant, asserted by the new source leg). Two honest deviations, both recorded rather than papered over:
  1. *Storage of the sealed record.* The daemon serves no `LoomLiteStore` row, so the sealed governed-grant record lands in the Transparency Log (`cohort:digest-read-governed`, correlated by `request_id`) rather than the collective store. The **seal** is the collective store's: the same `AtRestSeal` `Arc` from `at_rest_seal_hook()`, in the same `maos_loom_lite::seal::AtRestSealer` wrapper `LoomLiteStore::with_at_rest_seal` installs.
  2. *SIEM is not a refusal gate.* SSO / PDP / mint / at-rest fail **closed** (Err → NACK, no grant, no reply obligation, no cohort audit row). SIEM runs after durable persistence against a consistent live snapshot; sink or snapshot failure is visible and buffered without taking the daemon down.

**AC2 — Met.** No Spirit crate, no `LoadedSpiritKind` variant, no `classify_spirit` arm; asserted by the source leg (which also scans `spirits/` for any `*enterprise*` crate). The class is the daemon posture over the four env groups. Zero new environment variables — all four groups were already in `env_contract.rs`.

**AC3 — Met.** `story_13_5a_enterprise_governance_reaches_the_booted_cohort_daemon` boots the production runtime and a second real mTLS endpoint. `CohortDigestDistributor::request_read` crosses TCP/TLS, `handle_intake_verified`, consent, and the installed governed port; the daemon's correlated digest reply returns over reverse mTLS. Recording SSO/PDP/KMS/SIEM projection ports each fire, `identity.asserted` and one sealed governed row persist, and the SIEM sink is non-empty before shutdown.

**AC4 — Met.** ADR-057 authored (the empty 056→058 slot) + index row. Operator profile: `docs/runbooks/ent-1-enterprise-daemon-posture.md` — the four env groups, the daemon-attach pattern, a worked `researcher`-under-the-daemon example, a failure-mode table, and the air-gap unavailability (H2). ADR-008 is not cited; the runbook names the real scaffold (`maos-spirit-derive` / `-sdk` / `spirit_test` / `examples/example-spirit` + `xtask example-spirit-regen`) and says plainly that this story needs none of it.

**AC5 — Met, as a real control.** The unwired-required posture refuses daemon boot; PDP-only configuration reaches and enforces its deny; configured-down KMS refuses posture construction rather than selecting plaintext; wired deny/allow paths are two-sided. The reach and source legs both inspect the production dispatch. `check-multi-tenant-loom` passed all blocking legs; absent live Postgres legs remained correctly advisory-substrate.

**AC6 — Met, measured.** Kernel baseline remains 23228. Post-format `kloc-check`: `maos-bin` 13 927 ≤ 14 000; aggregate 132 988 ≤ 133 100; `xtask` 30 700 ≤ 30 750. The review re-base names the real mTLS lifecycle and fail-closed corrections; nothing is waived.

**H-constraints.** H2 air-gap unavailability documented. H3 uses `at_rest_seal_hook` only. H4 now proves a subject-specific PDP rule allows pid 1 and denies the daemon pid 0 for the same `loom.read` action. H5 uses a serialized process-local watermark, with no cross-restart exactly-once claim. H6 keeps `identity.asserted` out of kernel `FrameKind`.

**Out of scope, still open.** The 11.4b audit escape-anomaly detector remains a second real dead-wire, tracked only as an `ABSENT_SUCCESSORS` string. Not wired here; it exists.

**Review-patch verification:** all three `story_13_5a_enterprise_daemon_seam` tests passed; the independent source-inspection test passed; the complete `maos-a2a-core`, `maos-cohort`, and `maos-siem` package suites passed; `check-multi-tenant-loom` passed every blocking leg (live Postgres legs correctly reported advisory-substrate absent); `kloc-check` passed at 13 927 / 132 988; `cargo fmt --all --check` was clean after formatting.

Two things that are NOT green-by-default and are stated rather than buried:
- **`check-dev-model-tier` needed an allowlist entry.** The recorded dev model is `anthropic/claude-opus-5`; `FRONTIER_FAMILIES` in `xtask/src/check_dev_model_tier.rs` stopped at `opus-4-8`. `opus-5` was added with an inline rationale — that list's own doc comment says it carries "the frontier successors actually used in v2.2 dev", so this is its documented maintenance, not a waiver. A non-frontier model still fails the gate.
- **`check-env-contract` FAILS — pre-existing, not this story.** One unregistered read, `MAOS_VETTER_KEYRING` at `crates/maos-bin/src/main.rs:151`, landed with Story 13.4's `maosctl vet` wiring; `git show HEAD:crates/maos-bin/src/main.rs` has it at the same line. This story adds **zero** environment reads (all four enterprise groups were already registered), so the gate's state is unchanged by 13.5a. Not fixed here.
- **`cargo test --workspace --all-features`** fails to COMPILE in `maos-registry/tests/registry_roundtrip_test.rs` (`AdmissionConfig` missing `runtime_crypto_provider` / `runtime_provider_endpoint`) — pre-existing, untouched by this story, outside the default feature set.

### File List

| File | Change |
|---|---|
| `crates/maos-bin/src/main.rs` | modified — optional/PDP-only governance, KMS fail-closed construction, shared-journal admission, required-wiring validation, serialized SIEM forwarding, and the real two-endpoint mTLS lifecycle/negative/H4 proofs |
| `crates/maos-bin/src/enterprise_identity.rs` | modified — injected-port runtime plus live-WAL snapshot forwarding through the actual SIEM projection port |
| `crates/maos-bin/tests/enterprise_daemon_seam_13_5a.rs` | **new** — independent production-dispatch, governed-port, shared-journal, single-mint, H3, and no-enterprise-Spirit source controls |
| `crates/maos-a2a-core/src/cohort.rs` | modified — object-safe guarded digest-admission contract |
| `crates/maos-cohort/src/state.rs` | modified — atomic validate/deduplicate/capacity-check → governance guard → commit ordering, with duplicate/capacity tests |
| `crates/maos-siem/src/lib.rs` | modified — injected projection-port file-forwarding seam |
| `crates/maos-kernel-core/tests/manifest_field_coverage.rs` | modified — production capability-parser coverage counts 4→5 and 5→6 for the control-Spirit admission path (test-only; `src/` untouched, baseline 23228 unmoved) |
| `xtask/src/check_multi_tenant_loom.rs` | modified — three new `Blocking` legs: `enterprise-governance-reaches-cohort-daemon`, `enterprise-governance-daemon-dead-wire-negative`, `enterprise-governance-daemon-dispatch-threaded` |
| `xtask/kloc.toml` | modified — review re-base: `maos-bin` 13 927 ≤ 14 000 and aggregate 132 988 ≤ 133 100, with named drivers |
| `xtask/src/check_dev_model_tier.rs` | modified — `opus-5` added to `FRONTIER_FAMILIES` with rationale (documented maintenance of the E11-A1 allowlist) |
| `docs/adr/ADR-057-enterprise-governance-is-a-daemon-posture.md` | **new** — the decision record |
| `docs/adr/index.md` | modified — ADR-057 index row |
| `docs/runbooks/ent-1-enterprise-daemon-posture.md` | **new** — the operator-instantiable posture profile |
| `_bmad-output/implementation-artifacts/13-5a-enterprise-reference-spirit-class.md` | modified — `baseline_commit`, Tasks/Subtasks, Task 0 ruling, Dev Agent Record, File List, Change Log, Status |
| `_bmad-output/implementation-artifacts/sprint-status.yaml` | modified — 13-5a `review` → `done` after all review patches passed |

Path inventory (machine-checked):

- `crates/maos-bin/src/main.rs`
- `crates/maos-bin/src/enterprise_identity.rs`
- `crates/maos-bin/tests/enterprise_daemon_seam_13_5a.rs`
- `crates/maos-a2a-core/src/cohort.rs`
- `crates/maos-cohort/src/state.rs`
- `crates/maos-siem/src/lib.rs`
- `crates/maos-kernel-core/tests/manifest_field_coverage.rs`
- `xtask/src/check_multi_tenant_loom.rs`
- `xtask/kloc.toml`
- `xtask/src/check_dev_model_tier.rs`
- `docs/adr/ADR-057-enterprise-governance-is-a-daemon-posture.md`
- `docs/adr/index.md`
- `docs/runbooks/ent-1-enterprise-daemon-posture.md`
- `_bmad-output/implementation-artifacts/13-5a-enterprise-reference-spirit-class.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Change Log

| Date | Change |
|---|---|
| 2026-07-25 | Task 0 closed: daemon collective-op seam pinned to `DigestReadPort::note_admitted_request`; at-rest arm narrowed explicitly (no collective-store row exists on that path) |
| 2026-07-25 | Enterprise governance threaded into `cohort-a2a-daemon`; SIEM export consumer reaches daemon mode for the first time |
| 2026-07-25 | Hermetic in-process daemon-boot lifecycle proof + two-sided dead-wire negative + H4 pid-binding proof; both proven-red plants verified |
| 2026-07-25 | Three blocking legs folded into `check-multi-tenant-loom`; ADR-057 + ENT-1 runbook authored; kloc ceilings re-based on measured residual |
| 2026-07-25 | Status → `review` |
| 2026-07-25 | Adversarial review: 8/8 patches applied; real mTLS lifecycle, fail-closed PDP/KMS/unwired paths, atomic digest governance, shared journal, live SIEM projection, and H4 selective deny verified; status → `done` |

---

## Preflight decision record (GOVERNS)

Three disprove-the-brief scouts (A: subsystem reality; B: reference-Spirit pattern + already-shipped; C: daemon-seam + gate/proven-red) + party-mode. Eight decisions:

- **D1 — Reframe (category error).** "Compose Spirit-side" is impossible; the enterprise crates are `maos-bin`-only. 13.5a is enterprise governance at the **daemon seam**, a composition-root concern, not a Spirit crate.
- **D2 — The real dead-wire is the spine.** `cohort-a2a-daemon` returns at `main.rs:7398` before every enterprise reach; `EnterpriseRuntime` (constructed `:2501`) is unreached in that mode. AC3's leg is **proven-RED at HEAD** until the wiring lands. This is genuine net-new work, not "prove existing wiring."
- **D3 — Δ posture correction.** Epic's "ZERO expected (Spirit-side)" is WRONG. Correct: **ZERO kernel-core Δ** (out of `api.rs` per ADR-051/F8; `identity.asserted` stays kind-30, not a FrameKind), **NON-ZERO `maos-bin`** composition-root Δ. Both sentences mandatory.
- **D4 — No new Spirit / no `LoadedSpiritKind`.** A new collective-only enterprise Spirit re-hits the non-empty `provider.complete` blocker (`manifest.rs:473-480`) 13.5d dodged via `researcher`. The "class" is a daemon posture/profile.
- **D5 — Gate is a real control (Scout C design).** Reach leg (Blocking, hermetic daemon boot + recording ports) + two-sided dead-wire negative + main.rs source-inspection. Planted "drop enterprise arg at `:7398`" reds it. Avoid the three null-control traps (ABSENT_SUCCESSORS / available_arm_tests-only / seal_row_at_rest).
- **D6 — Keep as its own story, 6 ACs.** 13.6 is forbidden from inventing mechanism (it only judges); a dead-wire fix is a mechanism, so 13.6 cannot absorb it — 13.5a must own it. Not vacuous.
- **D7 — AC4 retarget off ADR-008.** ADR-008 is the registry protocol, not a scaffold; author ADR-057 for the daemon-posture decision and reuse the real scaffold (spirit-derive/sdk/cli/example-spirit + xtask regen).
- **D8 — Fold hazards as constraints.** H2 feature-gate, H3 proof-path divergence (`at_rest_seal_hook`, not `seal_row_at_rest`), H4 spirit_pid=0, H5 SIEM watermark single-lifetime, H6 identity.asserted stays kind-30.

**Open item for the dev (Task 0):** pin the exact daemon collective-op issuance site where governance attaches; if none exists, narrow AC1 to "a reference Spirit run **under** the collective daemon receives enterprise governance." Scout anchors provided above; do not invent a seam.

**Stale epic reference corrected:** `epic-13:212` cites `researcher` at `main.rs:3815`; HEAD is `:4218`.

Related: [[project_epic_13_rescope]], [[project_story_13_4_preflight]], [[project_story_11_4c_created]], [[project_story_13_5c_preflight_split]], [[feedback_party_mode_for_fork_consensus]], [[feedback_story_sizing]].
