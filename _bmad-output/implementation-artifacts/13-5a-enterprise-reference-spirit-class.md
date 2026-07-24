# Story 13.5a — Enterprise governance at the cohort-a2a-daemon seam

Status: **ready-for-dev** — preflight closed 2026-07-24 (party-mode Winston·Murat·Amelia·John·Mary·Grumbal + 3 disprove-the-brief scouts A/B/C). Baseline pin **23228** (`check-kernel-baseline`). **ZERO kernel-core Δ; NON-ZERO `maos-bin` composition-root Δ** — say both sentences. Depends 11.4a + 11.4c (both `done`); 13.5c (`done`, single composition root) is a hard prerequisite for the daemon seam.

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

*(to be completed by the dev)*

### Agent Model Used
*(frontier-class allowlist; record at dev start)*

### §A6 review net
*(record the multi-layer review artifact marker on completion — the `check-dev-model-tier` gate requires one of ["§A6","bmad-code-review","Blind Hunter","Acceptance Auditor","REVIEW COMPLETE"])*

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
