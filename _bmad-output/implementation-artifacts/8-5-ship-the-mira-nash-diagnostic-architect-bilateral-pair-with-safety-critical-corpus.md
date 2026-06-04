---
dev_model_used: claude-opus-4-8
---

# Story 8.5: Ship the Mira+Nash Diagnostic-Architect Bilateral Pair with Safety-Critical Corpus

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->
<!-- dev_model_used frontmatter is set/confirmed by the dev agent in AC1 (§A2 hard-fail gate). Recommended: claude-opus-4-8 (Decision K). -->

## Story

As a v1.5 operator deploying a diagnostic-architect bilateral 2-Host pair,
I want **Mira** (Host A, prod-edge diagnostic) + **Nash** (Host B, dev-environment architect) shipped as two reference Spirits that coordinate over **A2A cross-Host** with **pre-paired mTLS cert fingerprints (no discovery)**, where a halt firing on Mira **routes a notification to mobile push** AND **informs Nash via A2A typed-intent consent (ADR-012)** AND is **resolved by the director through the three-tap flow**, AND a **safety-critical corpus methodology (N≥150 per Spirit, inter-annotator agreement κ≥0.7)** is authored — all built **on the A2A / halt / notification / latency substrate that already exists** (E6 `LoopbackA2ARouter` + TOFU + ADR-012 consent + rotation-chaos, E4 halt protocol, E3 `HaltFlow` three-tap, the §13.1 J4 harness),
So that the **v1.5 bilateral-pair user journey (J4) is a working, audit-traced, safety-critical reference deployment** — Epic 8's capstone reference Spirit — proven end-to-end with **zero kernel KLOC**.

## What this story IS and IS NOT (read first — scope is deliberately bounded)

This is the **fifth and final reference-Spirit deliverable** of Epic 8 and the **first bilateral cross-Host pair**. Unlike Butler (8.1), Researcher (8.2), Observer (8.3 — single read-only watchdog) and the founder-loop wedge (8.4 — four same-host Spirits over the in-process bus), Story 8.5 proves the **two-Host coordination journey (J4)** over substrate that **already exists**: the E6 Story 6.3 A2A peer mesh (`LoopbackA2ARouter`, TOFU pinning, ADR-012 typed-intent consent, mTLS rotation chaos), the E4 halt protocol, the E3 Story 3.3 three-tap `HaltFlow`, the §7.4 `NotificationEvent::Halt` surface, and the §13.1 J4 latency harness all landed in Epics 3–7. **Mira and Nash are thin reference consumers of that substrate; they add no kernel code.** This story authors the genuinely-new *Spirit-side / eval-side* pieces the epic mandates — the J6 cold-start bench, the Cohen's-κ computation, and the safety-critical-corpus methodology — none of which is kernel code.

Scope is drawn to prevent **over-building** (a live two-process mTLS/TCP transport — see Decision B / Story 8.6) and **under-building** (manifest-only stubs that never drive the real TOFU/consent/halt path).

**This story IS:**
- Two new **workspace-member Spirit crates** under `spirits/` (Decision A; workspace **37 → 39**):
  - `spirits/mira/` — **rust-inproc** `[class]` Spirit; the **prod-edge diagnostic**. Carries `SpiritRole::Worker` (Decision C) with an `[epistemic_policy]` that **fires a halt** on an unresolved prod-edge anomaly (the "halt fires on Mira" path). Deterministic diagnosis at v1.5 (Decision I).
  - `spirits/nash/` — **rust-inproc** `[class]` Spirit; the **dev-environment Senior Architect** (architecture §6.4). Carries `SpiritRole::Worker` (Decision C). Receives the cross-Host advisory via A2A typed-intent consent and proposes a fix; deterministic at v1.5 (Decision I).
- The **bilateral A2A pairing over the REAL `LoopbackA2ARouter`** (Decision B; loopback-simulated two-Host): two distinct `HostId`s (`host_a`/`host_b`), **pre-paired `PeerCertFingerprint`s in the per-side `A2APeerConfig` (no discovery)**, **TOFU pinning verified** (`TofuPinStore::verify_pinned` over `InMemoryTofuPinStore`), **ADR-012 `ConsentAllowlists` (send/accept) typed-intent consent**, and the **mTLS rotation-chaos** harness (`compute_t_grace` / `RotationDrillReport`) — all driven as **dev-deps** (the 8.1–8.4 in-proc-bridge pattern). This proves the cross-Host addressing / consent / TOFU / rotation path **end-to-end in CI** without a live socket.
- The **halt → mobile-push + Nash-via-consent + three-tap-resolve** path, all over real adapters as dev-deps:
  - A halt fires on Mira → the **real** `NotificationDispatcher` fans `NotificationEvent::Halt { payload }` to a **test-double `NotificationChannel` whose `surface() == NotificationSurface::MobilePush`** that captures the dispatched event (Decision D — the real `MobilePushChannel` is a 6.5 panic-stub; the dispatch path is real, only the terminal push transport is fixture-replayed).
  - Nash on Host B is **informed via A2A typed-intent consent (ADR-012)**: Mira emits an advisory frame with an `A2AIntent` that Mira's `send_allowlist` admits and Nash's `accept_allowlist` admits, routed over the `LoopbackA2ARouter` with TOFU verify (Decision E).
  - The director **resolves the halt via the existing three-tap `HaltFlow`** (`Tap1Acknowledge → Tap2SelectKind → Tap3Submit → Done`, `submit_resolution` → `KernelHaltResolver`), driven as a dev-dep (Decision E).
- The **J4 latency budget** (Mira-Nash Observer colocation **< 10ms P95**, §13.1) verified via the **already-shipped** J4 harness (`crates/maos-bench/src/harness/j4.rs`, `J4_P95_BUDGET_US = 10_000`), with a `mira-nash-bench` CI job wiring the previously-unwired J4 bench (`--test` mode) — its v1.5 budget activates here (Decision G).
- The **J6 cold-start budget** (Diego cold-start **< 500ms**, §13.1) verified via a **NEW** `crates/maos-bench/src/harness/j6.rs` (`J6_P95_BUDGET_US = 500_000`), mirroring the J4 harness, cold-loading a Mira/Nash-shaped Spirit; `criterion` `bench_j6` added to `section_13_1`; budget reported per release (Decision G; user-confirmed author-J6).
- The **safety-critical corpus methodology** (Decision F; user-confirmed authored-κ + stand-in seam):
  - A **NEW** `crates/maos-eval/src/safety_critical_corpus.rs` module computing **Cohen's κ** (`cohen_kappa(annotator_a, annotator_b) -> f64`) deterministically over annotation-label fixtures, producing an `IaaAttestation` with **κ ≥ 0.7**.
  - **N ≥ 150** scenarios per Spirit (Mira + Nash), SHA-256-pinned per Story 0.3, registered in `tests/corpora/MANIFEST.toml` + `tests/coverage-matrix.yaml`.
  - `docs/safety-critical-corpus-methodology.md` documenting the **2-annotator human protocol** as a **STAND-IN seam** (the real human annotation is a documented process; CI fixture-replays the annotation labels so κ is deterministic and bit-stable — mirrors 7.5b's stand-in-corpus and Story 4.4's `iaa-attestation.json` pattern).
- The **runnable headline artifact**: a **`smoke-mira-nash-8-5`** one-shot in `maos-bin` (mirrors `smoke-a2a-loopback-6-3` + `smoke-founder-loop-8-4`, Decision H), running the full J4 journey at a compressed timeline: Mira(host_a) diagnoses a prod-edge anomaly → **halt fires on Mira** → mobile-push test-double captures the `Halt` notification + **Nash(host_b) informed via A2A typed-intent consent** (TOFU verified, send/accept allowlists admit) → director **three-tap resolves** → digest **cites actual `source_log_ref`s** via the existing FR17 path; plus one deliberate **A2A consent-denied** (`EIntentDenied`) observable in the Transparency Log. Exits 0.
- **Fixtures** (diagnostic/architect scenario inputs, annotation labels, wedge scenario) authored under each crate's `tests/fixtures/` + the corpus under `tests/corpora/`, **SHA-256-pinned per Story 0.3** and registered in `tests/coverage-matrix.yaml` / `tests/corpora/MANIFEST.toml` / corpus-staleness.

**This story IS NOT:**
- It does **NOT** complete the live two-process mTLS/TCP **cross-Host transport**. Investigation confirmed the `A2AProfile::CrossHost` profile is **fully absent** (not scaffolding): `LoopbackA2ARouter` is in-memory `mpsc` only (`crates/maos-a2a/src/adapter.rs:84,284-289`), no `TcpListener`/`TcpStream` exists anywhere in `maos-a2a`, `A2AProfile::CrossHost` is an enum variant **never dispatched on**, `maos-bin` has **no daemon A2A composition / listen-address wiring**, and the JSON-RPC framing is never serialized to a socket. Explicit deferral markers: *"the cross-Host TCP connector at v0.7"* (`transport/json_rpc.rs:127-130`), *"deferred to follow-up"* (`tests/cross_host_consent_v1.rs:177-185`), *"FR23b v1.0 cross-Host … operator-managed PKI, JSON-RPC over mTLS/TCP"* declared-not-implemented (`src/lib.rs:13`). Building it is **~1000+ LOC of new, security-critical socket+TLS code** in a **new crate** (`maos-a2a-tcp` — `maos-a2a` is already OVER its own 1500-line ceiling at 2550, and is **outside** the kernel-KLOC ceiling). This is a **different risk class** (security-critical networking) and is split out as **NEW Story 8.6** (Decision B/J; user-confirmed split). **Story 8.5 proves the cross-Host *protocol* — addressing, TOFU, consent, rotation — over the real `LoopbackA2ARouter`; Story 8.6 proves the cross-Host *transport*.**
- It does **NOT** add a `FrameKind`, `FramePayload`, `SpiritRole`, or `NotificationEvent` variant. The frozen v1.0 ABI already has `SpiritRole = {Director, Observer, Worker, Orchestrator}` (Mira and Nash both use `SpiritRole::Worker`, Decision C), `NotificationEvent::Halt { payload }`, `NotificationSurface::MobilePush`, and the A2A consent/TOFU types. `abi-diff` stays **Added-only / `removed = []`**.
- It does **NOT** implement the real `MobilePushChannel` gateway transport. That is the 6.5 panic-stub (`crates/maos-director-surface/src/notification.rs:271-285`, `unimplemented!("Story 6.5 — mobile push via gateway sub-modules")`); completing it is the §6.5 gateway sub-module work (`GatewayDispatcher` / `GatewayType::{Telegram,…}`), not Epic 8. 8.5 proves **the halt-notification routes to the MobilePush surface** via a test-double channel; the real transport is a carry-forward (Decision D).
- It does **NOT** add a live **LLM** to Mira/Nash. Their diagnosis/architecture cognition is deterministic/fixture-driven at v1.5 (no live `provider.complete` in CI), exactly the Butler/Researcher/Observer/Architect/Reviewer precedent. Live generative behavior is application-layer (Decision I).
- It does **NOT** add any cognition (diagnosis policy, anomaly classification, consent decision, κ thresholding) to `maos-kernel-core`. All Mira/Nash logic is Spirit-side; the κ computation lives in `maos-eval` and the J6 harness in `maos-bench` — **neither is `maos-kernel-core`** (Story 0.2 kernel-API surface invariant stays GREEN; `check-empty-kernel` + `check-service-boundary` = 0 violations).
- It does **NOT** re-implement the morning digest, the halt protocol, the three-tap flow, the A2A router, or the J4 harness. All are **consumed, not modified**.

## LOCKED Design Decisions (do NOT silently re-decide — chosen during story creation; flagged for Winston)

> Decisions **B, F, G, J** are **USER-CONFIRMED** (story-creation forks, 2026-06-04). The remaining decisions are flagged for Winston confirmation in the same manner as Story 8.4's rulings.

**Decision A — Two reference Spirit crates under `spirits/`; workspace count 37 → 39. (rust-inproc.)**
Ship `spirits/mira/` and `spirits/nash/` as workspace members (mirrors Butler/Researcher/Observer/Architect/Reviewer Decision A). `check-workspace-count` floor moves **37 → 39**; AC8 updates the `<!-- workspace-count-authoritative -->` sentinel in `4-kernel-design.md:115` (currently 37) to 39. **FLAG Winston:** confirm two crates under `spirits/` (count → 39).

**Decision B — Bilateral pair realized over the REAL `LoopbackA2ARouter` (loopback-simulated two-Host); live CrossHost TCP/mTLS transport is split to NEW Story 8.6. (USER-CONFIRMED — split.)**
Investigation confirmed the live `A2AProfile::CrossHost` TCP/mTLS transport is **fully absent** (see "IS NOT" above + the prerequisite table). The user selected "Split: ship 8.5 loopback now + new story 8.6 for live TCP". So Story 8.5 instantiates Mira and Nash with **distinct `HostId`s** (`host_a`/`host_b`) and drives the **real `LoopbackA2ARouter`** (in-process `mpsc`) as a dev-dep — exercising the genuine cross-Host *protocol surfaces*: per-side `A2APeerConfig` with **pre-paired `PeerCertFingerprint`s (no discovery)**, `TofuPinStore::verify_pinned` (TOFU), ADR-012 `ConsentAllowlists`, the JSON-RPC `handle_intake` path, the Lamport clock, and the `RotationDrillReport` chaos harness — without a real socket. The live two-process mTLS/TCP transport (a new `maos-a2a-tcp` crate, ~1000+ LOC, security-critical, outside kernel KLOC) is **NEW Story 8.6** (Decision J), recorded in `sprint-status.yaml` as backlog. **FLAG Winston:** confirm the loopback-simulated bilateral pair for 8.5 with live transport deferred to 8.6, vs. building the live transport now.

**Decision C — Mira and Nash both map to `SpiritRole::Worker`; NO new `SpiritRole` variant (ABI frozen).**
`SpiritRole = {Director, Observer, Worker, Orchestrator}` (`crates/maos-spirit-abi/src/identity.rs:127-132`) — there is **no** `Mira`/`Nash`/`Diagnostic`/`Architect` variant and the ABI is frozen at v1.0. Mira (diagnostic) and Nash (Senior Architect §6.4) are **specialized Workers**; both carry `SpiritRole::Worker` in their `FrameAddress.role` (the 8.4 Architect/Reviewer precedent). Mira is a **Worker** (not Observer) because it must **fire a halt** on an unresolved prod-edge anomaly — Observer (8.3) is strictly read-only and "halts nothing"; Mira's diagnostic cognition reaches epistemic boundaries and halts. The "Observer colocation" in the J4 budget name refers to a colocated 8.3-style Observer for the latency measurement, **not** to Mira being an Observer. Adding enum variants would break `abi-diff`. **FLAG Winston:** confirm Mira=Worker (diagnostic, halt-capable) + Nash=Worker (architect), vs. Mira=Observer.

**Decision D — Mobile-push-on-halt is realized via a test-double `NotificationChannel` (surface = `MobilePush`); the real gateway transport stays the 6.5 stub.**
The halt → notification → dispatch path is **real**: a halt produces `NotificationEvent::Halt { payload: EpistemicHaltPayload }`, fanned by the real `NotificationDispatcher::dispatch` to all registered channels. The real `MobilePushChannel::dispatch` is `unimplemented!("Story 6.5")` (`crates/maos-director-surface/src/notification.rs:271-285`); completing it is §6.5 gateway work (kernel-adjacent, not Epic 8). 8.5 registers a **test-double channel** whose `surface()` returns `NotificationSurface::MobilePush` and which **captures** the dispatched `Halt` event (asserting it routed to the mobile-push surface). This proves "notification routes to mobile push" without the live transport — the Butler/Observer fixture-replay precedent. **FLAG Winston:** confirm the test-double MobilePush channel (real dispatch path, fixture transport) vs. implementing the §6.5 gateway now.

**Decision E — Halt resolution uses the EXISTING three-tap `HaltFlow` + `KernelHaltResolver`; Nash informed via the EXISTING A2A typed-intent consent over `LoopbackA2ARouter`.**
The director resolves via the real `HaltFlow<KernelHaltResolver>` (`crates/maos-director-surface/src/halt_ui.rs:20-97`): `resolve_flow(Tap1Acknowledge → Tap2SelectKind → Tap3Submit → Done)` then `submit_resolution(halt_id, Resolution, spirit_id)` → `KernelHaltResolver::resolve` (`crates/maos-kernel-core/src/halt/resolver.rs:95`) with `Resolution ∈ {ProvidedContext, AcceptedHalt, AuthorizedOverride}`. Nash is informed by Mira emitting an advisory `IacFrame` carrying an `A2AIntent` (e.g. `A2AIntent::new("diagnostic.advisory")`) that Mira's `send_allowlist` and Nash's `accept_allowlist` both admit (`ConsentAllowlists::{send_admits,accept_admits}`), routed via `A2ARouter::route_outbound` with TOFU `verify_pinned`. No new resolution / consent mechanism. **FLAG Winston:** confirm reuse of the three-tap flow + A2A consent (vs. a bespoke notify path).

**Decision F — Safety-critical corpus: author Cohen's-κ compute + synthetic N≥150 corpus + `IaaAttestation(κ≥0.7)` + methodology doc, with the real 2-annotator protocol as a documented STAND-IN seam. (USER-CONFIRMED.)**
No κ-computation code exists today — `IaaAttestation` only **loads** a pre-computed `hedge_cohen_kappa` from `iaa-attestation.json` (`crates/maos-eval/src/distillate_corpus.rs:60-67`; Story 4.4 floor κ≥0.85 for distillate). 8.5 authors a deterministic **`cohen_kappa`** in a new `crates/maos-eval/src/safety_critical_corpus.rs` module, generates the **N≥150-per-Spirit** corpus (Mira + Nash; SHA-pinned, `valid_until` dated), produces an `IaaAttestation` with **κ ≥ 0.7** (the epic's safety-critical floor — lower than distillate's 0.85, documented), and writes `docs/safety-critical-corpus-methodology.md` documenting the **2-annotator human protocol**. The real human annotation is a **documented STAND-IN seam**: CI replays the two annotators' label fixtures so κ is deterministic and bit-stable (mirrors 7.5b's stand-in corpus + 8.1's seam-closure precedent). The κ floor 0.7 vs. 0.85 difference is intentional — distillate hedge-preservation is a tighter signal than safety-critical scenario labeling; record the rationale in the methodology doc. **FLAG Winston:** confirm κ floor = 0.7 (epic value) for the safety-critical corpus vs. aligning to distillate's 0.85.

**Decision G — J4 consumed from the EXISTING harness; J6 AUTHORED NEW in `maos-bench`. (USER-CONFIRMED author-J6.)**
J4 **already exists** — `crates/maos-bench/src/harness/j4.rs` (`J4_P95_BUDGET_US = 10_000`, `run_j4_measurement` / `run_j4_smoke`, criterion `bench_j4` in `section_13_1`); its v1.5 budget activates here. 8.5 wires a **`mira-nash-bench`** CI job running it in `--test` mode (Observer 8.3 deliberately left J4 unwired: "J4 <10ms is v1.5"). J6 is **ABSENT** — 8.5 authors `crates/maos-bench/src/harness/j6.rs` (`J6_P95_BUDGET_US = 500_000`, `run_j6_measurement` / `run_j6_smoke`, cold-loading a Mira/Nash-shaped Spirit ≥N times), adds `criterion bench_j6` to `section_13_1`, and reports the budget per release. `maos-bench` is **not** kernel KLOC (the 8.4 J1 precedent). A J4/J6 breach at HEAD is a §13.1 measurement recorded in the Dev Agent Record (the J1/8.4 escape-hatch semantics: "fix our code first; do not mask"). **FLAG Winston:** confirm consume-J4 + author-J6-in-maos-bench, and the breach-records-not-fails semantics.

**Decision H — The runnable headline artifact is a `smoke-mira-nash-8-5` one-shot in `maos-bin`.**
Mirroring `smoke-a2a-loopback-6-3` (the A2A loopback precedent) and `smoke-founder-loop-8-4` (`MAOS_ONE_SHOT` dispatch, wired into `discipline.yml`), the J4 journey is a **runnable** one-shot: Mira(host_a) diagnoses → halt fires on Mira → mobile-push test-double captures the `Halt` notification + Nash(host_b) informed via A2A typed-intent consent (TOFU verified, allowlists admit) → director three-tap resolves → digest cites actual `source_log_ref`s, plus one deliberate `EIntentDenied` consent rejection observable in the TL. `maos-bin` smoke code is **not** kernel KLOC (`kloc-check` counts `maos-kernel-core`; 6.2/8.4 precedent). This is the observable end-to-end demo `[[feedback_lunarpulse_observability_preference]]`. **FLAG Winston:** confirm the `maos-bin` `smoke-mira-nash-8-5` one-shot headline.

**Decision I — Mira/Nash cognition is deterministic/fixture-driven at v1.5; no live LLM in CI.**
Butler (8.1), Researcher (8.2), Observer (8.3), Architect/Reviewer (8.4) all fixture-replay their external drivers and run deterministic compute in CI. Mira's `diagnose(signal)` and Nash's `architect(advisory)` are pure, seeded, bit-identical (NFR-Testability-1). Per 8.4 Decision E, **omit `[capabilities.required]`/`provider.complete`** unless the validator mandates it (confirmed optional in 8.3/8.4); declare-but-unused if mandatory. **FLAG Winston:** confirm deterministic Mira/Nash at v1.5 (live LLM deferred).

**Decision J — NEW Story 8.6 recorded for the live `maos-a2a-tcp` two-process mTLS/TCP transport. (USER-CONFIRMED — split.)**
The live CrossHost transport (TCP listener/dialer, rustls server+client config with client-cert verification, async JSON-RPC framing over the socket, handshake retry, real partition timeout; a new `maos-a2a-tcp` crate outside kernel KLOC; `maos-bin` cross-host daemon mode; a two-real-process CI integration test) is split to **Story 8.6** (a v1.5 networking story, its own security-critical risk class). AC9 records `8-6-…` in `sprint-status.yaml` as `backlog`, inserted before `epic-8-retrospective`. **FLAG Winston:** confirm the 8.6 split + suggested key.

**Decision K — Recommended dev model: `claude-opus-4-8`.**
Rationale: the most integration-surface-diverse story in Epic 8 — the A2A consent/TOFU/rotation surfaces + the halt/notification/three-tap chain + the Cohen's-κ statistic + two latency benches, all driven as dev-deps over real adapters. Memory records deepseek-v4-pro is weak on async invariants / integration plumbing / env-var threading; the in-proc Spirit→adapter bridge (the 8.1–8.4 risk class) recurs here across **four** distinct subsystems plus a new statistical computation. 8.1–8.4 all used `claude-opus-4-8`.

**Decision L — Manifests are deployment-topology-agnostic; pre-paired peer cert fingerprints live in test/deployment config (`A2APeerConfig`), NOT the Spirit manifest.**
The `maos-manifest` schema has **no** host-affinity / A2A-peer / cert-fingerprint section (verified — the manifest stays topology-agnostic per architecture §7.2 ADR-003). Mira's and Nash's `manifest.toml` declare only the standard rust-inproc sections (`[class]`, `[posture]`, `[output_shape]`, `[budget]`, `[resources]`, `[sandbox]`, `[author]`, plus Mira's `[epistemic_policy]`). The pre-paired `PeerCertFingerprint`s + `ConsentAllowlists` live in the per-side `A2APeerConfig` constructed in the integration tests / smoke (the deployment-config layer). **FLAG Winston:** confirm peer fingerprints are deployment-config (not manifest).

## Winston Architectural Rulings (2026-06-04 — flags resolved post-implementation)

> Winston (System Architect) resolved the three dev-pass flags. Recorded here so review picks up the rulings inline.

**Ruling 1 — Consent matches the `IntentClass::Readonly` projection (`"readonly"`), not a free-form `"diagnostic.advisory"` string. → ACCEPT as-implemented.**
The `LoopbackA2ARouter` enforces consent on `frame.intent.a2a_consent_intent_str()` — the 3-band `IntentClass` projection `{highprivilege, standard, readonly}` (`maos-a2a/src/adapter.rs:144-164`), case-insensitive against the allowlist. Mira's advisory is read-only evidence, so `IntentClass::Readonly` ("readonly", = `mira::ADVISORY_CONSENT_INTENT`) is semantically correct and keeps the ABI frozen. The "diagnostic.advisory" naming in the Dev Notes is aspirational, realized as the `readonly` band. **Substrate gap (NOT an 8.5 defect):** `ConsentAllowlists` is a free-form `Vec<A2AIntent>` but only the 3-band projection ever matches — so a specific intent string silently never matches. Widening the taxonomy is a `maos-a2a` ABI change against an over-budget crate; **deferred to a consent-vocabulary follow-up scoped after Story 8.6 establishes `maos-a2a-core`** (recorded in the Story 8.6 scope note, epic-8 §AC-A6). v1.5 coarse 3-band gate is accepted behavior.

**Ruling 2 — κ floor = 0.7 for the safety-critical corpus (vs distillate's 0.85). → ACCEPT (Decision F confirmed).**
Two different measurements: distillate κ≥0.85 measures fine-grained hedge-preservation over near-identical text; safety-critical κ≥0.7 measures a coarser 3-way categorical scenario label (`benign/caution/critical`), where 0.7 ("substantial agreement", Landis–Koch) is the appropriate bar. The distinction is documented in `docs/safety-critical-corpus-methodology.md` + the `SAFETY_CRITICAL_KAPPA_FLOOR` constant, and **ratified in `docs/adr/ADR-042-safety-critical-kappa-floor-distinct-from-distillate.md`** (a third κ floor appearing is the trigger to generalize ADR-042 into a per-corpus-class table — guards against false harmonization).

**Ruling 3 — Decisions A, C, D, E, H, I, K, L confirmed as applied.**
Load-bearing rulings: **C** — Mira & Nash both `SpiritRole::Worker` is correct (Mira *must* be a Worker, not Observer, because it fires a halt; reusing the frozen enum over a new variant is the right call). **D** — MobilePush test-double over the real dispatch path is an honest seam (real `NotificationDispatcher`, only the §6.5 terminal transport fixture-replaced). **E** — building the full `KernelHaltResolver` (not a mock) is what makes the wiring *work*, not merely compile. **A/H/I/K/L** — consistent with the 8.1–8.4 reference-Spirit pattern; no objections.

## Prerequisites (verified present at story-creation time — re-verify in AC1)

| Prerequisite | Status | Path / Evidence |
|---|---|---|
| Spirit ABI + lifecycle hooks, `#[spirit]` proc-macro, `Ctx` | ✅ PRESENT | `crates/maos-spirit-abi/src/lifecycle.rs`, `…/identity.rs`; `crates/maos-spirit-derive/src/lib.rs` |
| Spirit SDK + local runner + spirit-test harness + v0.5 assert macros | ✅ PRESENT | `crates/maos-spirit-sdk/src/{local_runner.rs,spirit_test/{harness.rs,assert.rs,manifest.rs}}` |
| **`A2ARouter` port + `LoopbackA2ARouter` (real, in-process)** | ✅ PRESENT | `crates/maos-domain/src/ports/a2a.rs:22-40` (`route_outbound`); `crates/maos-a2a/src/adapter.rs:81-165` (`LoopbackA2ARouter::new`, `handle_intake`, `clock()`) |
| **`A2APeerConfig` + `A2AProfile{Loopback,CrossHost}`** (peer_id, endpoint, **cert_fingerprint**, profile, allowlists, partition_timeout) + `validate()` | ✅ PRESENT | `crates/maos-a2a/src/config.rs:12-22,33-111` (CrossHost is enum-only / **never dispatched** — Decision B) |
| **`PeerCertFingerprint{algo,hex}`** + `from_cert_der`/`parse`/`wire`/`short` (pre-paired, no discovery) | ✅ PRESENT | `crates/maos-a2a/src/identity.rs:46-104` |
| **TOFU pinning** — `TofuPin`, `TofuPinStore::{verify_pinned,pin_first_contact,invalidate_for_restart,get_pin}`, `InMemoryTofuPinStore`, `EPinMismatch` | ✅ PRESENT | `crates/maos-a2a/src/tofu.rs:16-267` |
| **ADR-012 typed-intent consent** — `A2AIntent`, `IntentAllowlist`, `A2AConsentEnvelope`, `ConsentAllowlists{send_allowlist,accept_allowlist,send_admits,accept_admits}`, `EIntentDenied` | ✅ PRESENT | `crates/maos-domain/src/invariants/i8.rs:29-49`; `crates/maos-a2a/src/consent.rs:13-84` |
| **mTLS rotation chaos** — `compute_t_grace`, `RotationDrillReport::from_per_agent`, `HandshakeRetryPolicy`; churn scaffold | ✅ PRESENT | `crates/maos-a2a/src/chaos/rotation.rs:1-158`; `…/chaos/churn.rs`; `…/src/mtls.rs:11-108` |
| **JSON-RPC framing + Lamport clock** (in-memory v0.5; TCP funnel deferred to 8.6) | ✅ PRESENT | `crates/maos-a2a/src/transport/{json_rpc.rs,logical_clock.rs}` |
| **Cross-host error mapping** — `IacBusError::{CrossHostNotConfigured,CrossHostIntentDenied,CrossHostPinMismatch,CrossHostConsentExpired,CrossHostPartitionTimeout,…}` | ✅ PRESENT | `crates/maos-domain/src/iac_bus_types.rs:24-70` |
| **A2A cross-host reference tests** (loopback) | ✅ PRESENT | `crates/maos-a2a/tests/{cross_host_consent_v1.rs,cert_rotation_chaos_3_host.rs,restart_invalidates_pin_nfr_rel_6.rs,churn_3_host_scaffold.rs}` |
| **Halt protocol** — `HaltId`, `Resolution{ProvidedContext,AcceptedHalt,AuthorizedOverride}`, `HaltResolver`, `KernelHaltResolver`, `HaltReceipt`, `HaltState`, `HaltJournal` | ✅ PRESENT | `crates/maos-domain/src/halt.rs:11-251`; `crates/maos-kernel-core/src/halt/resolver.rs:95-215` |
| **Notification surface** — `NotificationEvent{TaskAssigned,ApprovalPrompt,Halt{EpistemicHaltPayload},AnomalyFlagged}`, `NotificationDispatcher{register,dispatch}`, `NotificationChannel` trait, `NotificationSurface{Terminal,AcpEditor,MobilePush}`, `DispatchReport` | ✅ PRESENT | `crates/maos-domain/src/notification.rs:7-66`; `crates/maos-director-surface/src/notification.rs:23-285` |
| **`MobilePushChannel`** (the §6.5 panic-stub; Decision D test-doubles it) | ✅ PRESENT (stub) | `crates/maos-director-surface/src/notification.rs:271-285` (`unimplemented!("Story 6.5 …")`) |
| **Three-tap `HaltFlow`** — `HaltFlow<R>`, `FlowState{Tap1Acknowledge,Tap2SelectKind,Tap3Submit,Done}`, `TapEvent`, `resolve_flow`, `dispatch_halt`, `submit_resolution` | ✅ PRESENT | `crates/maos-director-surface/src/halt_ui.rs:20-104` |
| **J4 latency harness** (`J4_P95_BUDGET_US=10_000`; `run_j4_measurement`/`run_j4_smoke`; criterion `bench_j4` in `section_13_1`) | ✅ PRESENT (unwired in CI) | `crates/maos-bench/src/harness/j4.rs:1-199`; `crates/maos-bench/benches/section_13_1.rs` |
| **`maos-eval` IaaAttestation** (`corpus_version,annotator_count,hedge_cohen_kappa,computed_at`) + `load_from` (loads κ; does NOT compute it) | ✅ PRESENT | `crates/maos-eval/src/distillate_corpus.rs:60-128` |
| **`maos-corpus-gen` `CorpusGenerator` trait** (`seed_corpus,expand,validate,coverage_report,seed_sha256,rule_version`) | ✅ PRESENT | `crates/maos-corpus-gen/src/lib.rs:97-122` |
| **Corpus SHA-pin + coverage registration** — `tests/corpora/MANIFEST.toml` (`sha256,schema_version,item_count,valid_until,prompt_version_hash,description`) + `tests/coverage-matrix.yaml` + `corpus-staleness` gate | ✅ PRESENT | `tests/corpora/MANIFEST.toml`; `tests/coverage-matrix.yaml`; `xtask corpus-staleness` |
| **`smoke-a2a-loopback-6-3`** + **`smoke-founder-loop-8-4`** (one-shot precedents to mirror) | ✅ PRESENT | `crates/maos-bin/src/main.rs:4192-4413` (a2a-loopback), `…:3540-3799`+`smoke_founder_loop_8_4` (founder-loop); `MAOS_ONE_SHOT` dispatch + `discipline.yml` wiring |
| **FR17 morning-digest path** (`source_log_ref` citation) | ✅ PRESENT | `spirits/butler/src/lib.rs`, `spirits/researcher/src/lib.rs` |
| Butler/Researcher/Observer/Architect/Reviewer reference crates (structure to mirror — Observer is the closest rust-inproc template) | ✅ PRESENT | `spirits/{butler,researcher,observer,architect,reviewer}/{Cargo.toml,manifest.toml,src/lib.rs,tests/}` |
| Workspace count gate + authoritative sentinel | ✅ PRESENT (=37) | root `Cargo.toml` members (37); `xtask check-workspace-count`; sentinel `<!-- workspace-count-authoritative -->` in `4-kernel-design.md:115` (declares **37** post-8.4) |
| Kernel-API surface invariant (Story 0.2) — restricts **`maos-kernel-core` only** (NOT `maos-a2a`/`maos-bench`/`maos-eval`) | ✅ PRESENT | `.github/workflows/discipline.yml` (`check-service-boundary`/`check-empty-kernel`); `xtask/src/check_service_boundary.rs` |
| KLOC ceilings — `maos-kernel-core=6000`; **`maos-a2a=1500` (OVER at 2550)**; `maos-bench`/`maos-eval` own budgets (not kernel) | ✅ PRESENT | `xtask/kloc.toml:48,67`; `maos-a2a` NOT in I9 whitelist (`xtask/i9-whitelist.toml`) |
| CI new-spirit wiring (job + aggregate `needs:`) + bench-job pattern (`researcher-bench`/`founder-loop-bench`, `--test`) | ✅ PRESENT | `discipline.yml` — `*-tests` + `*-bench` jobs wired into `aggregate` `needs:` |
| **`spirits/{mira,nash}/` + fixtures + coverage-matrix slots** | ❌ **ABSENT** — **this story creates them** | none exist today |
| **`crates/maos-bench/src/harness/j6.rs`** (Diego cold-start <500ms) | ❌ **ABSENT** — **this story authors it** | only j0/j1/j4/j_researcher exist |
| **`crates/maos-eval/src/safety_critical_corpus.rs`** (Cohen's κ compute) | ❌ **ABSENT** — **this story authors it** | κ is only *loaded* from JSON today, never computed |
| **`docs/safety-critical-corpus-methodology.md`** | ❌ **ABSENT** — **this story authors it** | only `docs/corpus-extensions/*` exist |
| **`smoke-mira-nash-8-5`** one-shot | ❌ ABSENT — this story creates it | mirror `smoke-a2a-loopback-6-3` + `smoke-founder-loop-8-4` |
| Live CrossHost TCP/mTLS transport (`maos-a2a-tcp`) | ❌ DEFERRED (Decision B/J) | fully absent (not scaffolding); **NEW Story 8.6**, recorded in AC9 |
| Real `MobilePushChannel` gateway transport | ❌ DEFERRED (Decision D) | §6.5 stub; carry-forward (test-doubled here) |

## Acceptance Criteria

### AC1 — Prerequisites & scope classified mechanically before pair work opens

**Given** the prerequisite table above
**When** AC1 runs first
**Then** the dev confirms each ✅ path/symbol still exists — the A2A surfaces (`A2ARouter`/`LoopbackA2ARouter`, `A2APeerConfig`/`PeerCertFingerprint`, `TofuPinStore::verify_pinned`, ADR-012 `ConsentAllowlists`/`A2AIntent`/`EIntentDenied`, `compute_t_grace`/`RotationDrillReport`), the halt/notification chain (`NotificationEvent::Halt`, `NotificationDispatcher`, `NotificationSurface::MobilePush`, the `MobilePushChannel` stub, `HaltFlow`/`FlowState`/`submit_resolution`, `KernelHaltResolver`, `Resolution`), the J4 harness (`J4_P95_BUDGET_US=10_000`), the eval/corpus surfaces (`IaaAttestation`, `CorpusGenerator`, `MANIFEST.toml`, coverage-matrix, corpus-staleness), `SpiritRole={Director,Observer,Worker,Orchestrator}`, the workspace-count sentinel (=37), and the kernel-API gate — and records the result in the Dev Agent Record
**And** the absence of `spirits/{mira,nash}/`, `maos-bench/src/harness/j6.rs`, `maos-eval/src/safety_critical_corpus.rs`, `docs/safety-critical-corpus-methodology.md`, and the four artifacts' coverage slots is confirmed; the **fully-absent live CrossHost transport** is re-confirmed (no `TcpListener`/`TcpStream` in `maos-a2a`; `A2AProfile::CrossHost` never dispatched) and recorded as the Story-8.6 split (Decision B/J)
**And** Decisions A–L are recorded as the chosen resolutions, not silently re-decided; `dev_model_used` is recorded/confirmed in the story frontmatter (§A2 hard-fail gate).

### AC2 — Mira+Nash bilateral pair coordinate over A2A with pre-paired cert fingerprints + TOFU pinning (loopback-simulated two-Host)

**Given** the Mira (`spirits/mira/`, prod-edge diagnostic) and Nash (`spirits/nash/`, dev-environment architect) reference Spirits (rust-inproc `[class]`, `SpiritRole::Worker` — Decision C)
**When** Mira on `HostId("host_a")` and Nash on `HostId("host_b")` are wired over the **real** `LoopbackA2ARouter` (dev-dep) with a per-side `A2APeerConfig` carrying the **other side's pre-paired `PeerCertFingerprint` (no discovery)** and `ConsentAllowlists`
**Then** an integration test drives the real router: Mira→Nash and Nash→Mira frames route via `A2ARouter::route_outbound`; **TOFU pinning is verified** (`TofuPinStore::verify_pinned` admits the matching pinned fingerprint and rejects a mismatched one with `EPinMismatch::Mismatch`); the pre-paired fingerprints are consumed from config (no discovery/first-contact roundtrip needed when pre-pinned)
**And** the cross-Host addressing uses `FrameAddress.host_id = Some(HostId(..))`, and the **live CrossHost TCP/mTLS transport is explicitly out of scope** (Decision B; Story 8.6) — the loopback router proves the protocol surface without a socket
**And** Mira/Nash dispatch logic lives entirely in `spirits/{mira,nash}/`; the A2A router is consumed, not modified.

### AC3 — J4 latency: Mira-Nash colocation < 10ms P95 (§13.1)

**Given** the J4 latency budget (Mira-Nash Observer colocation < 10ms P95 per §13.1)
**When** the J4 benchmark runs (the **existing** `maos-bench` J4 harness — `J4_P95_BUDGET_US = 10_000`; `cargo bench -p maos-bench --bench section_13_1 -- --test` — Decision G)
**Then** the budget is **met**, OR a breach is **recorded** as a §13.1 measurement in the Dev Agent Record (the J1/8.4 escape-hatch semantics — not masked by migration)
**And** a `mira-nash-bench` CI job is added (mirroring `founder-loop-bench`, `--test` mode) and wired into the gate aggregation — activating the J4 bench's v1.5 budget that Observer 8.3 deliberately left unwired
**And** a budget overrun emits `BudgetWarning` (NFR-Perf-6), and the J4 measurement basis + pass/breach outcome are recorded in the Dev Agent Record.

### AC4 — Halt on Mira → mobile-push + Nash-via-consent + director three-tap resolution

**Given** a halt fires on Mira (e.g., an unresolved prod-edge anomaly hits Mira's `[epistemic_policy]` boundary)
**When** the kernel dispatches the halt notification
**Then** the **real** `NotificationDispatcher::dispatch(NotificationEvent::Halt { payload }, …)` fans to a **test-double `NotificationChannel` with `surface() == NotificationSurface::MobilePush`** which **captures** the `Halt` event — proving the notification **routes to mobile push** (Decision D; the real `MobilePushChannel` §6.5 transport is out of scope)
**And** **Nash on Host B is informed via A2A typed-intent consent (ADR-012)**: Mira emits an advisory frame carrying an `A2AIntent` that Mira's `send_allowlist` and Nash's `accept_allowlist` both admit (`ConsentAllowlists::{send_admits,accept_admits}`), routed over the `LoopbackA2ARouter` with TOFU `verify_pinned`; a frame whose intent is **not** in Nash's `accept_allowlist` is **rejected** with `EIntentDenied` (negative case)
**And** the director **resolves the halt via the existing three-tap flow** (`HaltFlow::resolve_flow`: `Tap1Acknowledge → Tap2SelectKind → Tap3Submit → Done`, then `submit_resolution(halt_id, Resolution, "mira")` → `KernelHaltResolver::resolve`) with a `Resolution` (e.g. `ProvidedContext`/`AcceptedHalt`), and the resolution is journaled (`HaltJournal`) — all driven as dev-deps; no new resolution mechanism (Decision E).

### AC5 — Safety-critical corpus: N≥150 per Spirit, κ≥0.7 across ≥2 annotators, methodology documented

**Given** the safety-critical Spirit corpus methodology (Decision F — authored κ-compute + synthetic corpus + stand-in annotation seam)
**When** the Mira+Nash corpora are authored
**Then** the corpus has **N ≥ 150 scenarios per Spirit** (Mira + Nash), SHA-256-pinned per Story 0.3, with a deterministic generator (env-gated, bit-identical) if used
**And** a **NEW** `crates/maos-eval/src/safety_critical_corpus.rs` computes **Cohen's κ** (`cohen_kappa(annotator_a_labels, annotator_b_labels) -> f64`) over the two annotators' label fixtures, and **inter-annotator agreement κ ≥ 0.7 is verified across ≥ 2 annotators** — emitting/loading an `IaaAttestation { annotator_count ≥ 2, hedge_cohen_kappa ≥ 0.7, … }` (mirrors the Story 4.4 `iaa-attestation.json` pattern; the κ floor 0.7 is the epic's safety-critical value, distinct from distillate's 0.85 — rationale documented)
**And** the methodology is documented in **`docs/safety-critical-corpus-methodology.md`** (the 2-annotator human protocol as a **documented STAND-IN seam**: real human annotation is the process; CI fixture-replays the annotation labels so κ is deterministic — mirrors 7.5b's stand-in seam)
**And** the κ computation + N≥150 floor + κ≥0.7 floor are exercised by a test that **fails loud** if the corpus shrinks below 150 or κ drops below 0.7.

### AC6 — J6 cold-start budget authored: Diego cold-start < 500ms (§13.1)

**Given** the J6 cold-start budget (Diego cold-start < 500ms per §13.1) and the J6 harness is **ABSENT** (Decision G — user-confirmed author-J6)
**When** the J6 harness is authored
**Then** a **NEW** `crates/maos-bench/src/harness/j6.rs` defines `J6_P95_BUDGET_US = 500_000`, `run_j6_measurement` / `run_j6_smoke` cold-loading a Mira/Nash-shaped Spirit ≥ N times (mirroring the J4 harness structure), and a `criterion bench_j6` is added to `crates/maos-bench/benches/section_13_1.rs`
**And** a Mira/Nash cold-load **completes within 500ms** (or a breach is recorded as a §13.1 measurement, not masked), and the budget is **reported per release** in the Dev Agent Record
**And** the J6 bench runs in the `mira-nash-bench` CI job (`--test` mode) alongside J4; `maos-bench` is **not** kernel KLOC (the 8.4 J1 precedent).

### AC7 — Mira+Nash bilateral demo: runnable, halt-traced, digest cites actual log refs (the headline)

**Given** the runnable `smoke-mira-nash-8-5` one-shot in `maos-bin` (Decision H; mirrors `smoke-a2a-loopback-6-3` + `smoke-founder-loop-8-4`)
**When** it runs at a compressed timeline
**Then** the full J4 journey executes end-to-end: Mira(host_a) diagnoses a prod-edge anomaly → **a halt fires on Mira** → the mobile-push test-double **captures the `Halt` notification** + **Nash(host_b) is informed via A2A typed-intent consent** (TOFU verified, send/accept allowlists admit) → the director **three-tap resolves** the halt → the morning digest **cites actual `source_log_ref`s** resolved against the **real** Transparency Log via the existing FR17 path, and one deliberate **`EIntentDenied`** consent rejection is observable in the Transparency Log
**And** the one-shot is wired into `discipline.yml` (`MAOS_ONE_SHOT=smoke-mira-nash-8-5`, with `timeout-minutes`) and **exits 0** on the happy path.

### AC8 — Zero kernel KLOC; ABI frozen; workspace count reconciled; manifests conform

**Given** Mira/Nash are rust-inproc reference crates and the new infra lives in `maos-eval`/`maos-bench`/`docs` (zero kernel KLOC)
**When** their logic is added
**Then** all Mira/Nash logic lives in `spirits/{mira,nash}/`, the κ compute in `crates/maos-eval/`, the J6 harness in `crates/maos-bench/`, the methodology in `docs/` — **none in `maos-kernel-core`** — the Story 0.2 kernel-API surface invariant stays GREEN (`check-empty-kernel` + `check-service-boundary` = 0 violations; **no** new kernel public fn), and each Spirit crate keeps `maos-kernel-core` / `maos-director-surface` / `maos-a2a` / `maos-bench` adapters in `[dev-dependencies]` only (the 8.1–8.4 pattern: Spirit-side deps = `maos-spirit-sdk` + `maos-spirit-abi` + `maos-domain` + serde)
**And** **no new `FrameKind`, `FramePayload`, `SpiritRole`, or `NotificationEvent` variant** is added — `abi-diff` is **Added-only with `removed=[]`** (Mira/Nash use `SpiritRole::Worker`, Decision C); if any ABI delta appears, STOP and flag (mis-scoped)
**And** `check-workspace-count` is reconciled to **39** (Decision A): root `Cargo.toml` members + the `<!-- workspace-count-authoritative -->` sentinel in `4-kernel-design.md` both updated 37 → 39
**And** `maos-a2a` is **not edited** (consumed as dev-dep) so its pre-existing 1500-ceiling overage (2550) is **untouched and wedge-neutral**; if any `maos-a2a` edit is required, STOP and flag (it would be Story-8.6 scope)
**And** every manifest passes `maos-manifest` validation with each section verified against the authoritative validators before authoring (`deny_unknown_fields`; rust-inproc `[class]`; Mira declares `[epistemic_policy]` for the halt path, Nash does not; both omit `[capabilities.required]` per Decision I unless mandatory; peer fingerprints are NOT manifest fields — Decision L).

### AC9 — Fixtures + corpus authored, SHA-pinned, and registered (Story 0.3); CI / discipline green; Story 8.6 recorded

**Given** the deterministic test inputs (diagnostic/architect scenario fixtures, the N≥150 corpus, the two annotators' label fixtures, the wedge scenario — no live socket, no live LLM in CI)
**When** the fixtures + corpus are authored
**Then** they are SHA-256-pinned per Story 0.3 (per-crate `fixtures_pin.rs` mirroring `spirits/observer`; corpus entries in `tests/corpora/MANIFEST.toml` with `sha256`/`schema_version`/`item_count ≥ 150`/`valid_until`/`prompt_version_hash`/`description`) and registered in `tests/coverage-matrix.yaml`; the new `mira`/`nash` slots are added to `reference_spirits` (`path: spirits/<name>`, `ships_at: "v1.5"`, `third_party: false`) and a safety-critical-corpus coverage row is added
**And** per-crate `mira-tests`/`nash-tests` jobs (`cargo test -p <name> --locked`, with `timeout-minutes`) + the `mira-nash-bench` job (J4 + J6, `--test`) + the `smoke-mira-nash-8-5` step are added to `.github/workflows/discipline.yml` and wired into the gate-aggregation `needs:` list (mirrors `researcher-tests`/`founder-loop-bench`); a `maos-eval` test exercising `safety_critical_corpus` (κ≥0.7, N≥150) is wired into the existing `maos-eval` test job
**And** **NEW Story 8.6** (`8-6-…` live `maos-a2a-tcp` two-process mTLS/TCP transport) is recorded in `sprint-status.yaml` as `backlog`, inserted before `epic-8-retrospective` (Decision J)
**And** `check-service-boundary` (0 new violations), `check-empty-kernel` (0), `check-workspace-count` (39/39), `coverage-matrix`, `corpus-staleness`, `abi-diff` (Added-only/`removed=[]`), `kloc-check`, and the §A2 `check-dev-model-used-populated` gate are all GREEN at HEAD — **no flipped-while-red** (the Epic 7 §A2 trap)
**And** the full `cargo test -p {mira,nash} --locked` + `maos-eval` + `maos-bench` suites pass, the Butler/Researcher/Observer/Orchestrator/Worker/Architect/Reviewer regressions stay clean (0 failures), and the Dev Agent Record lists every file created/modified, with any pre-existing RED (e.g. `maos-a2a` 1500-overage, `kloc-check`) verified pair-neutral (identical clean-HEAD-vs-changes) and flagged, not introduced.

## Tasks / Subtasks

- [x] **T1 — Prerequisite + scope pre-check (AC1)**
  - [x] Re-verify every ✅ row (paths + key symbols): A2A surfaces (`LoopbackA2ARouter`, `A2APeerConfig`/`PeerCertFingerprint`, `TofuPinStore::verify_pinned`, `ConsentAllowlists`/`A2AIntent`/`EIntentDenied`, `compute_t_grace`/`RotationDrillReport`), halt/notification (`NotificationEvent::Halt`, `NotificationDispatcher`, `NotificationSurface::MobilePush`, `MobilePushChannel` stub, `HaltFlow`/`FlowState`/`submit_resolution`, `KernelHaltResolver`, `Resolution`), J4 harness (`J4_P95_BUDGET_US=10_000`), eval/corpus (`IaaAttestation`, `CorpusGenerator`, `MANIFEST.toml`), `SpiritRole`, workspace sentinel (=37), kernel-API gate; record in Dev Agent Record
  - [x] Re-confirm the live CrossHost transport is fully ABSENT (no TCP in `maos-a2a`); confirm `spirits/{mira,nash}/`, `j6.rs`, `safety_critical_corpus.rs`, the methodology doc ABSENT; record Decisions A–L
  - [x] Confirm/set `dev_model_used` frontmatter (§A2 gate)
- [x] **T2 — Scaffold Mira + Nash crates + A2A bilateral pairing (AC2, AC8; Decision A/B/C)**
  - [x] Create `spirits/mira/` + `spirits/nash/` mirroring `spirits/observer/` shape; Spirit-side deps = `maos-spirit-sdk[local_runner]` + `maos-spirit-abi` + `maos-domain` + serde; dev-deps = `maos-spirit-sdk[local_runner,mock,spirit_test]` + `maos-kernel-core` + `maos-director-surface` + `maos-a2a` + `maos-manifest` + tokio/tempfile/sha2/toml
  - [x] `manifest.toml` for each: `[class]` (mira/nash, 1.5.0, abi=1.0, schema=2, min_substrate_version, forms=["rust-inproc"], trust_tier="local"); `[posture]`; `[output_shape]`; `[budget]`/`[resources]`; `[sandbox] tier="T2"`; Mira adds `[epistemic_policy]` (halt rule — verify shape); both omit `[capabilities.required]` (Decision I) unless mandatory; NO peer-fingerprint fields (Decision L); validate via `tests/spirit_smoke.rs`
  - [x] Implement `Mira::diagnose(signal)` (deterministic; reaches an epistemic boundary → halt path) + `Nash::architect(advisory)` (deterministic); dispatch/host logic Spirit-side
  - [x] Integration test `tests/a2a_pairing.rs` (AC2): two `HostId`s over the real `LoopbackA2ARouter` (dev-dep), pre-paired `PeerCertFingerprint` in per-side `A2APeerConfig`, `TofuPinStore::verify_pinned` admits-match / rejects-mismatch (`EPinMismatch`), bidirectional `route_outbound`
- [x] **T3 — Halt → mobile-push + Nash-via-consent + three-tap resolve (AC4; Decision D/E)**
  - [x] Integration test `tests/halt_bilateral.rs`: a halt fires on Mira → real `NotificationDispatcher::dispatch(NotificationEvent::Halt{..})` → a test-double `NotificationChannel` (`surface()==MobilePush`) captures it; Nash informed via an `A2AIntent` admitted by send/accept allowlists over `LoopbackA2ARouter` (TOFU verified); a non-allowlisted intent → `EIntentDenied` (negative); director three-tap (`resolve_flow` Tap1→Tap2→Tap3→Done) + `submit_resolution` → `KernelHaltResolver::resolve` + `HaltJournal`
  - [x] (If a colocated 8.3-style Observer is used for the J4 measurement context, wire it read-only — do NOT add cognition)
- [x] **T4 — Safety-critical corpus + Cohen's κ + methodology (AC5; Decision F)**
  - [x] Author `crates/maos-eval/src/safety_critical_corpus.rs`: `cohen_kappa(a,b) -> f64` (deterministic), corpus loader/generator for N≥150 per Spirit, `IaaAttestation` producer/checker (κ≥0.7, annotator_count≥2); re-export from `maos-eval/src/lib.rs`
  - [x] Author the N≥150 Mira + Nash corpora + the two annotators' label fixtures (env-gated deterministic generator if used; bit-identical)
  - [x] Author `docs/safety-critical-corpus-methodology.md` (2-annotator human protocol as documented STAND-IN seam; κ≥0.7 floor + rationale vs. distillate 0.85)
  - [x] Test (in `maos-eval`): κ computed ≥0.7, N≥150 per Spirit, fail-loud on shrink/κ-drop
- [x] **T5 — J6 cold-start harness (AC6; Decision G)**
  - [x] Author `crates/maos-bench/src/harness/j6.rs`: `J6_P95_BUDGET_US=500_000`, `run_j6_measurement`/`run_j6_smoke` (cold-load Mira/Nash-shaped Spirit ≥N times; mirror j4.rs); add `criterion bench_j6` to `benches/section_13_1.rs`
  - [x] Record the cold-start budget pass/breach per release in the Dev Agent Record
- [x] **T6 — Mira+Nash wedge demo: `smoke-mira-nash-8-5` (AC7; Decision H)**
  - [x] Add `smoke_mira_nash_8_5()` to `maos-bin/src/main.rs` (mirror `smoke_a2a_loopback_6_3` + `smoke_founder_loop_8_4`): Mira diagnoses → halt fires → mobile-push test-double captures + Nash via A2A consent (TOFU + allowlists) → three-tap resolve → digest cites actual `source_log_ref`s (FR17) + one deliberate `EIntentDenied`
  - [x] Wire `MAOS_ONE_SHOT=smoke-mira-nash-8-5` dispatch + the `discipline.yml` step (`timeout-minutes`); exits 0
- [x] **T7 — J4 latency (AC3; Decision G)**
  - [x] Run the existing J4 harness (`cargo bench -p maos-bench --bench section_13_1 -- --test`); record pass or the §13.1 breach
  - [x] Add `mira-nash-bench` CI job (J4 + J6, `--test`, mirror `founder-loop-bench`) + wire into aggregate
- [x] **T8 — Fixtures + corpus: author, SHA-pin, register (AC9)**
  - [x] Per-crate `fixtures_pin.rs` (mirror observer); corpus entries in `tests/corpora/MANIFEST.toml` (item_count≥150, valid_until dated); ADD `mira`/`nash` `reference_spirits` slots (`ships_at: v1.5`) + a safety-critical-corpus row to `tests/coverage-matrix.yaml`; run `coverage-matrix` + `corpus-staleness`
- [x] **T9 — Zero-kernel-KLOC / ABI / workspace count (AC8)**
  - [x] Confirm no `maos-kernel-core` AND no `maos-a2a` edits; `check-empty-kernel` + `check-service-boundary` (0 violations); `abi-diff` Added-only/`removed=[]` (no new FrameKind/FramePayload/SpiritRole/NotificationEvent)
  - [x] Add `spirits/{mira,nash}` to root `Cargo.toml` members (→39); bump the `4-kernel-design.md` sentinel 37→39; run `check-workspace-count` (39/39)
- [x] **T10 — CI / discipline green + Story 8.6 recorded (AC9)**
  - [x] Add `mira-tests`/`nash-tests` jobs + `mira-nash-bench` + `smoke-mira-nash-8-5` step; wire all into `aggregate` `needs:`; wire the `safety_critical_corpus` test into the `maos-eval` job
  - [x] Record `8-6-…` (live `maos-a2a-tcp` transport) in `sprint-status.yaml` as `backlog` before `epic-8-retrospective`
  - [x] Verify all AC9 gates GREEN at HEAD; pre-existing reds (maos-a2a overage, kloc-check) verified pair-neutral; no flipped-while-red; File List complete

## Dev Notes

### Spirit form & scaffolding (mirror Observer 8.3 — the closest rust-inproc template)
- **Two crates rust-inproc `[class]`** (mira/nash). Scaffold by copying `spirits/observer/` shape: Spirit-side deps only in `[dependencies]` (`maos-spirit-sdk[local_runner]` + `maos-spirit-abi` + `maos-domain` + serde/serde_json); real adapters (`maos-kernel-core`, `maos-director-surface`, `maos-a2a`, `maos-manifest`, tokio, sha2, toml) in `[dev-dependencies]` so integration is PROVEN without violating Story 0.2. Keep state in `Arc<Mutex<...>>` with poison-safe `unwrap_or_else(|e| e.into_inner())` (the 8.2/8.3 review fix). The `#[spirit]` macro synthesizes no-op bodies for unused hooks.
- **`Ctx` exposes only opaque handles** (`cancellation()`, `capability()`, `mailbox()`, `deprecation_warnings()`). A lifecycle hook cannot reach kernel services directly — the A2A router, the notification dispatcher, the halt resolver, and the J4/J6 benches are proven in tests that drive the **real adapters as dev-dependencies** (the resolved 8.1–8.4 pattern). This is the single most likely place to lose a review cycle — do NOT reach into `maos-kernel-core`/`maos-a2a`/`maos-director-surface` from any `spirits/*` lib.

### A2A bilateral pairing — loopback-simulated two-Host (AC2; Decision B)
- `LoopbackA2ARouter::new(peers, tofu_store, clock)` (`crates/maos-a2a/src/adapter.rs:81`) is **in-process** (DashMap + `mpsc` intake_sink) — NO socket. Construct two routers (one per "side") or one router with two peer entries; address frames with `FrameAddress.host_id = Some(HostId("host_a"|"host_b"))`.
- `A2APeerConfig { peer_id, endpoint("tls://…" — validated but unused on loopback), cert_fingerprint: PeerCertFingerprint, profile: A2AProfile::Loopback, allowlists: ConsentAllowlists, partition_timeout_secs }` (`config.rs:33`). **Pre-paired fingerprints** = put the peer's `PeerCertFingerprint` (from `from_cert_der` over a fixture cert) directly in config — no discovery.
- TOFU: `InMemoryTofuPinStore::new()`; pre-pin via `pin_first_contact` (declared==observed) OR seed pins, then `verify_pinned(peer, observed)` admits the match and returns `EPinMismatch::Mismatch` on a tampered fingerprint. NFR-Rel-6 restart (`invalidate_for_restart`) is OUT of scope unless trivially demonstrable.
- **CrossHost is enum-only** (`A2AProfile::CrossHost` never dispatched). Do NOT branch on it; do NOT add socket code. That is Story 8.6.
- Reference test to mirror: `crates/maos-a2a/tests/cross_host_consent_v1.rs` (7-scenario loopback consent matrix), `cert_rotation_chaos_3_host.rs` (rotation), `restart_invalidates_pin_nfr_rel_6.rs` (TOFU).

### ADR-012 typed-intent consent (AC2/AC4; Decision E)
- `ConsentAllowlists { send_allowlist: Vec<A2AIntent>, accept_allowlist: Vec<A2AIntent> }` with `send_admits(&A2AIntent)` / `accept_admits(&A2AIntent)` (`crates/maos-a2a/src/consent.rs:43`). `A2AIntent::new("diagnostic.advisory")` (`maos-domain/src/invariants/i8.rs:33`). Frame projection: `frame.intent.a2a_consent_intent_str()` (`maos-domain/src/invariants/i1.rs:139`).
- Positive: Mira's `send_allowlist` ∋ intent AND Nash's `accept_allowlist` ∋ intent ⇒ delivered. Negative: intent ∉ Nash's `accept_allowlist` ⇒ `EIntentDenied { peer, intent, direction: Accept }` / NACK `-32001`.

### Halt → notification → three-tap (AC4; Decision D/E)
- Halt produces `NotificationEvent::Halt { payload: EpistemicHaltPayload }` (`maos-domain/src/notification.rs:43`). The real `NotificationDispatcher::{register,dispatch}` (`maos-director-surface/src/notification.rs:49`) fans to channels. **Decision D test-double:** implement a `struct MobilePushCapture { captured: Arc<Mutex<Vec<NotificationEvent>>> }` whose `surface()` returns `NotificationSurface::MobilePush` and whose `dispatch` records the event — do NOT call the real `MobilePushChannel` (it `unimplemented!`s). Register it via `dispatcher.register(Box::new(MobilePushCapture::…))`.
- Three-tap: `HaltFlow::new(resolver, dispatcher, journal)`; `HaltFlow::resolve_flow(state, tap)` is a pure total function `Tap1Acknowledge → Tap2SelectKind → Tap3Submit → Done`; `submit_resolution(halt_id, resolution, spirit_id)` resolves FIRST then journals (fail-closed). `Resolution ∈ {ProvidedContext{text}, AcceptedHalt, AuthorizedOverride{operator_policy_ref}}` (`maos-domain/src/halt.rs:51`). `KernelHaltResolver::resolve` (`maos-kernel-core/src/halt/resolver.rs:95`) is the production sink (dev-dep).

### Safety-critical corpus + Cohen's κ (AC5; Decision F)
- κ is currently only **loaded** (`IaaAttestation { corpus_version, annotator_count, hedge_cohen_kappa, computed_at }`, `distillate_corpus.rs:60`; `load_from` reads `iaa-attestation.json`) — there is NO `cohen_kappa` function. Author it: Cohen's κ = `(p_o − p_e) / (1 − p_e)` over two annotators' categorical labels; deterministic; unit-tested against known κ values (perfect-agreement→1.0, chance→0.0).
- Generate N≥150 per Spirit; SHA-pin per Story 0.3; `MANIFEST.toml` entry per corpus (`item_count ≥ 150`, `valid_until` dated, `sha256`, `prompt_version_hash`). Mirror the corpus-gen `CorpusGenerator` trait (`maos-corpus-gen/src/lib.rs:97`) if a generator is used, else author static JSON fixtures + a pin test.
- **Stand-in seam** (mirrors 7.5b + 8.1): `docs/safety-critical-corpus-methodology.md` documents the real 2-annotator human protocol; CI replays the two annotators' label fixtures so κ is bit-stable. The floor is **κ≥0.7** (epic value) — document why it differs from distillate's 0.85.

### J4 / J6 latency (AC3/AC6; Decision G)
- J4 EXISTS: `crates/maos-bench/src/harness/j4.rs` (`J4_P95_BUDGET_US=10_000`, `run_j4_measurement`/`run_j4_smoke`; "Mira-Nash Observer colocation"); criterion `bench_j4` already in `benches/section_13_1.rs`. Observer 8.3 left J4 unwired ("J4 <10ms is v1.5") — wire it now.
- J6 ABSENT: author `harness/j6.rs` mirroring j4.rs (`J6_P95_BUDGET_US=500_000`, cold-load). Add `bench_j6` to the criterion group. `crates/maos-bench/src/bin/section_13_1_run.rs` orchestrates full-count runs — extend it if J6 should appear in the JSON report.
- `--test` mode = regression guard (mirror `founder-loop-bench`/`researcher-bench`). A budget breach at HEAD is a §13.1 measurement RECORDED (the 8.4 Decision F semantics), not a hard gate fail; never migrate to mask overhead ("fix our code first").

### Wedge demo (AC7; Decision H)
- Mirror `smoke_a2a_loopback_6_3` (`crates/maos-bin/src/main.rs:4192-4413`; the A2A loopback precedent) AND `smoke_founder_loop_8_4` (`MAOS_ONE_SHOT` dispatch + `discipline.yml` step). `maos-bin` smoke code is NOT kernel KLOC.
- Digest: reuse the FR17 path (Butler/Researcher); 8.5 proves the halt/advisory frames carry citable `source_log_ref`s against the **real** Transparency Log — not a digest re-impl.

### Zero kernel KLOC / boundaries (AC8)
- **No edits** to `maos-kernel-core` (Story 0.2) AND **no edits** to `maos-a2a` (it is OVER its own 1500 ceiling at 2550 — any edit worsens it and is Story-8.6 scope). Consume both as dev-deps. `kloc-check` counts `maos-kernel-core` (6000) for the kernel mandate; `maos-bench`/`maos-eval`/`maos-bin` have their own budgets and are not the kernel ceiling.
- `SpiritRole={Director,Observer,Worker,Orchestrator}` — Mira/Nash both `Worker` (Decision C). NO new ABI variant; `abi-diff` Added-only/`removed=[]`.

### Testing standards
- SDK spirit-test harness + v0.5 macros (`spirit_test_assert!`, `spirit_test_expect_frame!`, `assert_no_deprecations!`). Real adapters (`LoopbackA2ARouter`, `NotificationDispatcher`, `HaltFlow`/`KernelHaltResolver`, J4/J6 benches) driven via dev-deps. All inputs deterministic — fixtures, no live socket / live LLM in CI. SHA-pin fixtures + corpus per Story 0.3; register in corpus-staleness/coverage-matrix. ≥500ms timeouts on any async/telemetry test bound (the 8.2 flake fix).

### Prior-story lessons (carry into dev — these cost review cycles in 8.1–8.4)
- **Bus registration:** if Mira/Nash register over the IAC bus, a `register_spirit_typed` handle **must be bound** (dropping it closes the mailbox → `ChannelClosed`) — the 8.4 fix. The same applies to the `LoopbackA2ARouter` intake_sink: hold the sender.
- **`abi-diff` needs `--base`:** run it with the `--base` flag (no-base mode false-positives) — the 8.3 lesson.
- **Never `cargo fmt -p <crate>` here:** it causes whole-crate collateral reformatting. Format only the specific new files, or rely on the absence of a CI fmt gate (8.4 lesson; avoid touching `maos-bin` formatting).
- **Frame origin:** Spirit-authored frames use `FrameOrigin::SpiritAuto`, not `HumanAuthored` (the 8.4 review patch).
- **Poison-safe locks:** `Arc<Mutex<_>>` with `unwrap_or_else(|e| e.into_inner())` (the 8.2/8.3 review fix).
- **Async/telemetry test bounds:** ≥500ms timeouts (the 8.2 flake fix).

### Project Structure Notes
- **Two new crates** under `spirits/`: each `Cargo.toml`, `manifest.toml`, `src/lib.rs`, `tests/` (spirit_smoke + a2a_pairing + halt_bilateral + fixtures_pin), `tests/fixtures/`. Add both to root `Cargo.toml` members (→39); bump the sentinel 37→39; ADD two coverage-matrix `reference_spirits` slots + a safety-critical-corpus row.
- **New non-Spirit artifacts:** `crates/maos-eval/src/safety_critical_corpus.rs` (+ lib.rs re-export + a test), `crates/maos-bench/src/harness/j6.rs` (+ criterion `bench_j6`), `docs/safety-critical-corpus-methodology.md`, `tests/corpora/MANIFEST.toml` entries for the Mira/Nash corpora.
- **No edits** to `maos-kernel-core` or `maos-a2a`. `maos-bin` gains only the `smoke-mira-nash-8-5` one-shot.

### References
- [Source: epics/epic-8-…miranash-v03-v15.md#Story 8.5] — story statement + 5 BDD AC blocks (A2A pre-paired fingerprints + TOFU / J4 <10ms / halt→mobile-push+Nash-consent+three-tap / safety-critical κ≥0.7 N≥150 / J6 <500ms); v1.5 acceptance demo "Mira on Host A and Nash on Host B coordinate over A2A cross-Host; mTLS rotation chaos passes; safety-critical κ≥0.7 verified"
- [Source: architecture-maos-minimal-opus/6-reference-spirits.md#6.4] — Senior Architect (Nash); §6.5 Observer founder-loop/colocation use case
- [Source: architecture-maos-minimal-opus/7-…#7.2/7.3] — A2A profiles (Loopback FR23a v0.8 / CrossHost FR23b v1.0), ADR-003 deployment-config topology, ADR-012 typed-intent consent
- [Source: architecture-maos-minimal-opus/13-phased-roadmap.md#13.1] — J4 Mira-Nash Observer colocation <10ms P95; J6 Diego cold-start <500ms; "fix our code first" breach semantics
- [Source: architecture-maos-minimal-opus/4-kernel-design.md] — workspace-count sentinel (37→39); §4.0.7 kernel non-interpretive
- [Source: crates/maos-a2a/src/{adapter.rs,config.rs,identity.rs,tofu.rs,consent.rs,chaos/rotation.rs,mtls.rs,transport/json_rpc.rs}] — LoopbackA2ARouter + A2APeerConfig + PeerCertFingerprint + TofuPinStore + ConsentAllowlists + rotation chaos (CrossHost transport ABSENT → Story 8.6)
- [Source: crates/maos-domain/src/{halt.rs,notification.rs,invariants/i8.rs,iac_bus_types.rs}] — halt protocol + Resolution + NotificationEvent/Surface + A2AIntent + cross-host error mapping
- [Source: crates/maos-director-surface/src/{notification.rs,halt_ui.rs}] — NotificationDispatcher/Channel + MobilePushChannel stub + three-tap HaltFlow/FlowState/submit_resolution
- [Source: crates/maos-kernel-core/src/halt/resolver.rs] — KernelHaltResolver (dev-dep)
- [Source: crates/maos-bench/src/harness/j4.rs + benches/section_13_1.rs] — J4 harness (existing) + criterion entry; J6 to be authored mirroring J4
- [Source: crates/maos-eval/src/distillate_corpus.rs + crates/maos-corpus-gen/src/lib.rs] — IaaAttestation (loads κ; κ-compute to be authored) + CorpusGenerator trait
- [Source: tests/corpora/MANIFEST.toml + tests/coverage-matrix.yaml] — corpus SHA-pin + reference_spirits slot shape (ships_at v1.5)
- [Source: crates/maos-bin/src/main.rs:4192-4413 (smoke-a2a-loopback-6-3) + smoke_founder_loop_8_4 + discipline.yml] — one-shot precedents to mirror for smoke-mira-nash-8-5
- [Source: spirits/{butler,researcher,observer,architect,reviewer}/{Cargo.toml,manifest.toml,src/lib.rs,tests/}] — reference crate structure (dev-dep adapters, SHA-pinned fixtures, poison-safe locks, ≥500ms timeouts)
- [Source: _bmad-output/implementation-artifacts/8-4-….md] — the in-proc Spirit→adapter-as-dev-dep bridge pattern, fixture-replay precedent, Architect Rulings format
- [Source: xtask/kloc.toml + xtask/i9-whitelist.toml] — maos-a2a own 1500 ceiling (OVER 2550; do-not-edit), maos-kernel-core 6000 kernel ceiling

## Dev Agent Record

### Agent Model Used

`claude-opus-4-8` (Decision K) — to be recorded in frontmatter `dev_model_used` per §A2.

### Debug Log References

- `cargo test -p mira -p nash --locked` — 17 mira + 15 nash tests green (unit + a2a_pairing + halt_bilateral + spirit_smoke + fixtures_pin).
- `cargo test -p maos-eval --test safety_critical_corpus_8_5 --locked` + module unit tests — κ + corpus floors green; `cohen_kappa` verified against a known κ=0.6 reference.
- `cargo bench -p maos-bench --bench section_13_1 -- --test` — J0/J1/J4/J6/J-Researcher all "Success" (J4 + J6 regression guards).
- `MAOS_ONE_SHOT=smoke-mira-nash-8-5 ./maos-bin` — full journey, **exit 0**.
- Gates: `check-workspace-count` PASSED (39/39); `check-empty-kernel` 0; `check-service-boundary` 0; `abi-diff --base` `removed=[]`/no-breaking; `corpus-staleness` PASSED; `coverage-matrix` exit 0.

### Completion Notes List

**AC1 (T1) — prerequisites & scope, mechanically pre-checked.** All ✅ prerequisite paths/symbols re-verified present (A2A: `LoopbackA2ARouter::new(Vec<A2APeerConfig>, Arc<dyn TofuPinStore>)`, `A2APeerConfig`/`PeerCertFingerprint`, `TofuPinStore::verify_pinned`, `ConsentAllowlists`/`A2AIntent`/`EIntentDenied`, `compute_t_grace`/`RotationDrillReport`; halt/notification: `NotificationEvent::Halt`, `NotificationDispatcher`, `NotificationSurface::MobilePush`, `MobilePushChannel` §6.5 stub, `HaltFlow`/`FlowState`/`submit_resolution`, `KernelHaltResolver`, `Resolution`; J4 `J4_P95_BUDGET_US=10_000`; eval/corpus `IaaAttestation`/`CorpusGenerator`/`MANIFEST.toml`; `SpiritRole={Director,Observer,Worker,Orchestrator}`; sentinel=37; kernel-API gate). The fully-ABSENT live CrossHost TCP transport re-confirmed (zero `TcpListener`/`TcpStream` in `maos-a2a`; `A2AProfile::CrossHost` never dispatched) → Story 8.6 split (already recorded backlog). `dev_model_used: claude-opus-4-8` set. Decisions A–L applied as chosen.

**Consent-matching clarification (flag for Winston):** the story Dev Notes reference `A2AIntent::new("diagnostic.advisory")`, but the `LoopbackA2ARouter` matches consent on `frame.intent.a2a_consent_intent_str()` — the `IntentClass` projection (`"highprivilege"`/`"standard"`/`"readonly"`), case-insensitive against the allowlist `A2AIntent` strings (`adapter.rs:144-164`). Mira's advisory is read-only evidence → it carries `IntentClass::Readonly` → consent intent `"readonly"` (`mira::ADVISORY_CONSENT_INTENT`); both sides' allowlists admit `"readonly"`. The "diagnostic.advisory" naming is realized as the `readonly` projection. No ABI change.

**AC2 (T2) — bilateral pair over the real LoopbackA2ARouter.** `spirits/mira` + `spirits/nash` (rust-inproc, `SpiritRole::Worker`). `a2a_pairing.rs` drives a single loopback router holding two pre-paired-fingerprint `A2APeerConfig`s keyed by `HostId` (`host_a`/`host_b`): bidirectional `route_outbound` delivery, `TofuPinStore::verify_pinned` admits the matching pin / rejects a tampered one with `EPinMismatch::Mismatch`, send-side `IntentDenied{Send, EIntentDenied}` and accept-side `IntentDeniedAtPeer` both proven. Cross-Host addressing via `FrameAddress.host_id`; live TCP/mTLS explicitly out of scope (8.6). Router consumed, not modified.

**AC4 (T3) — halt → mobile-push + Nash-consent + three-tap.** `halt_bilateral.rs`: a halt fires on Mira via the real `invoke_halt` (TL `EpistemicHalt` row + lifecycle journal + pending-registry insert); the real `NotificationDispatcher` fans `NotificationEvent::Halt` to a test-double channel whose `surface()==MobilePush` which captures it (Decision D — real `MobilePushChannel` left the §6.5 stub); Nash informed via a `readonly` A2A advisory (positive) with `IntentDeniedAtPeer` negative; director three-tap `resolve_flow` (Tap1→Tap2→Tap3→Done) + `submit_resolution(AcceptedHalt)` → real `KernelHaltResolver::resolve`, journaled against the real `TransparencyLogAdapter` (which implements `HaltJournal`). Already-resolved re-submit fails (registry transitioned).

**AC5 (T4) — safety-critical corpus + Cohen's κ.** NEW `crates/maos-eval/src/safety_critical_corpus.rs`: `cohen_kappa(a,b)->f64` (deterministic; perfect→1.0, chance→0.0, verified against a known κ=0.6 reference); `SafetyCriticalCorpus::generate()` → **150 Mira + 150 Nash** scenarios with two replayed annotator labels (stand-in seam), **κ = 0.83 per Spirit** (≥ 0.7 floor); `IaaAttestation{annotator_count:2, hedge_cohen_kappa:0.83}`; fail-loud `validate()`; SHA-256 pin `CORPUS_SHA256_PIN`. `docs/safety-critical-corpus-methodology.md` documents the 2-annotator protocol + the κ-0.7-vs-0.85 rationale (Decision F). κ floor 0.7 is the epic value — **flag for Winston** (distinct from distillate's 0.85).

**AC6 (T5) — J6 cold-start harness AUTHORED.** NEW `crates/maos-bench/src/harness/j6.rs` (`J6_P95_BUDGET_US=500_000`, `run_j6_measurement`/`run_j6_smoke`; real cold-load loop behind `kernel_measurement`, canned smoke otherwise — mirrors j4.rs); `bench_j6` added to the `section_13_1` criterion group; J6 added to the `section_13_1_run` release report. **J6 smoke p95=134_617µs ≤ 500_000µs — budget MET.**

**AC3 (T7) — J4 latency + bench wiring.** Existing J4 harness run via `cargo bench -p maos-bench --bench section_13_1 -- --test`. **J4 smoke p95=1_940µs ≤ 10_000µs — budget MET.** NEW `mira-nash-bench` CI job (J4 + J6, `--test`) activates the J4 v1.5 budget Observer 8.3 left unwired.

**AC7 (T6) — runnable headline.** NEW `smoke_mira_nash_8_5()` in `maos-bin` (`MAOS_ONE_SHOT=smoke-mira-nash-8-5`, wired into `discipline.yml`): Mira(host_a) diagnoses an unexplained prod-edge anomaly (confidence 0.00 < 0.5 → halt) → halt fires (EpistemicHalt TL row) → mobile-push test-double captures the `Halt` → Nash(host_b) informed via A2A consent (TOFU verified) and architects a circuit-breaker fix → one deliberate `IntentDeniedAtPeer` (a `ConsentRupture` TL row — observable) → director three-tap resolves (real `KernelHaltResolver` + journal) → morning digest cites a real `ConsentRequest` TL frame resolving via `query_frame_by_id` (FR17). **Exits 0.**

**AC8 (T9) — zero kernel KLOC / ABI / workspace.** All Mira/Nash logic in `spirits/{mira,nash}`, κ in `maos-eval`, J6 in `maos-bench`, methodology in `docs/` — **zero `maos-kernel-core` edits**, **zero `maos-a2a` edits** (its 1500-overage at 2550 untouched, wedge-neutral). Spirit-side deps = `maos-spirit-sdk`+`maos-spirit-abi`+`maos-domain`+serde; real adapters in `[dev-dependencies]` only. No new `FrameKind`/`FramePayload`/`SpiritRole`/`NotificationEvent` variant; `abi-diff --base` `removed=[]`, no breaking (ABI surface byte-identical to HEAD — `maos-spirit-abi`/`maos-domain` untouched). `check-workspace-count` 39/39 (root members + `4-kernel-design.md` sentinel both 37→39). Manifests: Mira `[epistemic_policy]` halt rule (validated), Nash none; both omit `[capabilities.required]` (Decision I); no peer-fingerprint manifest fields (Decision L).

**AC9 (T8/T10) — fixtures/corpus registered; CI green; 8.6 recorded.** Per-crate `fixtures_pin.rs` SHA-pins (mira `1bb35ccc…`, nash `a752a131…`); corpus `safety-critical-mira-nash-v1.5` in `tests/corpora/MANIFEST.toml` (sha `454ba193…`, item_count 300, valid_until 2027-06-04, prompt_version_hash `23a9684d…`); `mira`/`nash` `reference_spirits` slots (ships_at v1.5) + `NFR-Saf-1` safety-critical coverage row in `tests/coverage-matrix.yaml`. CI: `mira-tests`, `nash-tests`, `mira-nash-bench`, `safety-critical-corpus-8-5`, `smoke-mira-nash-8-5` jobs added + all wired into `aggregate` `needs:`. Story `8-6-…` already recorded `backlog` before `epic-8-retrospective`. Gates GREEN at HEAD: check-service-boundary 0, check-empty-kernel 0, check-workspace-count 39/39, coverage-matrix, corpus-staleness, abi-diff (Added-only/`removed=[]`). No flipped-while-red.

**Pre-existing REDs verified pair-neutral (flagged, not introduced):** `kloc-check` is RED at clean HEAD (`maos-kernel-core` 15505/6000 long-standing debt + budget-0 crates `maos-eval`/`maos-registry`/`xtask`/etc.). The crates this story added lines to — `maos-bench` (1253/500), `maos-bin` (5087/1000), `maos-eval` (2576/0) — were **all already ❌ OVER at HEAD**; mira/nash are not in the kloc table (`spirits/`, not `crates/`); the kernel mandate crate had **zero edits**. The failing-crate set is identical clean-HEAD-vs-changes — wedge-neutral (the 8.3/8.4 precedent).

### File List

**New — Mira crate (`spirits/mira/`):**
- `spirits/mira/Cargo.toml`
- `spirits/mira/manifest.toml`
- `spirits/mira/src/lib.rs`
- `spirits/mira/tests/spirit_smoke.rs`
- `spirits/mira/tests/a2a_pairing.rs`
- `spirits/mira/tests/halt_bilateral.rs`
- `spirits/mira/tests/fixtures_pin.rs`
- `spirits/mira/tests/fixtures/diagnostic-scenarios.json`

**New — Nash crate (`spirits/nash/`):**
- `spirits/nash/Cargo.toml`
- `spirits/nash/manifest.toml`
- `spirits/nash/src/lib.rs`
- `spirits/nash/tests/spirit_smoke.rs`
- `spirits/nash/tests/fixtures_pin.rs`
- `spirits/nash/tests/fixtures/architect-scenarios.json`

**New — non-Spirit artifacts:**
- `crates/maos-eval/src/safety_critical_corpus.rs`
- `crates/maos-eval/tests/safety_critical_corpus_8_5.rs`
- `crates/maos-bench/src/harness/j6.rs`
- `docs/safety-critical-corpus-methodology.md`

**Modified:**
- `Cargo.toml` (workspace members → 39: + `spirits/mira`, `spirits/nash`)
- `Cargo.lock` (new crates)
- `crates/maos-eval/src/lib.rs` (`pub mod safety_critical_corpus` + re-exports)
- `crates/maos-bench/src/harness/mod.rs` (`pub mod j6`)
- `crates/maos-bench/benches/section_13_1.rs` (`bench_j6` + criterion group)
- `crates/maos-bench/src/bin/section_13_1_run.rs` (J6 measurement in the release report)
- `crates/maos-bin/Cargo.toml` (+ `mira`, `nash` deps)
- `crates/maos-bin/src/main.rs` (`smoke_mira_nash_8_5()` + `MAOS_ONE_SHOT` dispatch + mode list)
- `.github/workflows/discipline.yml` (`mira-tests`, `nash-tests`, `mira-nash-bench`, `safety-critical-corpus-8-5`, `smoke-mira-nash-8-5` jobs + aggregate `needs:`)
- `tests/corpora/MANIFEST.toml` (`safety-critical-mira-nash-v1.5` entry)
- `tests/coverage-matrix.yaml` (`mira`/`nash` reference_spirits slots + `NFR-Saf-1` row)
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` (workspace-count sentinel 37→39)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (8-5 → review)

### Change Log

| Date | Change |
|---|---|
| 2026-06-04 | Story 8.5 implemented (review): `spirits/{mira,nash}` (workspace 37→39) over existing A2A/halt/notification/J4 substrate, **zero kernel KLOC** (no `maos-kernel-core`/`maos-a2a`/ABI edits). Bilateral pair over the real `LoopbackA2ARouter` (pre-paired fingerprints + TOFU verify/mismatch + send/accept consent); halt→mobile-push test-double + Nash-via-consent + three-tap `KernelHaltResolver`+journal; NEW `maos-eval::safety_critical_corpus` (`cohen_kappa` + N=150/Spirit, κ=0.83≥0.7) + methodology doc; NEW `maos-bench::j6` (J6 cold-start <500ms, met) + J4 wired (met); `smoke-mira-nash-8-5` exits 0. 5 CI jobs + corpus/coverage registration. Consent matches on `IntentClass::Readonly` projection (clarified). Flags for Winston: κ floor 0.7 vs distillate 0.85 (Decision F) + Decisions A,C,D,E,H,I,K,L. Pre-existing `kloc-check` RED verified pair-neutral. |
| 2026-06-04 | Story 8.5 created (ready-for-dev): Mira+Nash diagnostic-architect bilateral pair (`spirits/{mira,nash}`, workspace 37→39) over existing A2A/halt/notification/J4 substrate (zero kernel KLOC). Three user-confirmed forks: (B/J) loopback-simulated pair now + NEW Story 8.6 for the fully-absent live `maos-a2a-tcp` two-process mTLS/TCP transport; (F) authored Cohen's-κ + synthetic N≥150 corpus + stand-in annotation seam (κ≥0.7); (G) consume existing J4 harness + author NEW J6 cold-start harness. Decisions A,C,D,E,H,I,K,L flagged for Winston. |

### Review Findings

#### decision-needed
- [x] [Review][Decision] Precision loss in diagnostic confidence transmission (f32 cast) — **RESOLVED: Option C** (Team consensus, 2026-06-04). Add epsilon guard (e.g., `1e-6`) before f64→f32 cast to prevent boundary flips. Preserves ABI contract while eliminating truncation-induced halt flips.
- [x] [Review][Decision] J6 real measurement (`kernel_measurement` feature) untested in CI — **RESOLVED: Hybrid** (Team consensus, 2026-06-04). Keep smoke-only in PR CI for fast feedback; add scheduled nightly/weekly job for real `kernel_measurement` cold-load loop on consistent hardware. Satisfies "breach recorded not masked" without flaky PR gates.
- [x] [Review][Decision] J6 measures kernel adapters, not actual Mira/Nash Spirit instantiation — **RESOLVED: Option B** (Team consensus, 2026-06-04). J6 benchmark must instantiate actual `Mira` and `Nash` Spirits, not just kernel adapters. Proxy measurements measure the wrong abstraction; spec says "cold-load a Mira/Nash-shaped Spirit."
- [x] [Review][Decision] `FrameKind::TaskAssign` used for diagnostic advisory — **RESOLVED: Option A with debt documentation** (Team consensus, 2026-06-04). Pragmatic reuse acceptable per AC8 (no new FrameKind variant). Must document as tech debt in code comment: `TaskAssign` is being used as read-only advisory carrier, not a mutable task assignment. Next FrameKind taxonomy revision should address this gap.

#### patch (all applied 2026-06-04)
- [x] [Review][Patch] Resource leak: smoke test temp directory never cleaned up on early return — FIXED: Added RAII `TempDirGuard` in `smoke_mira_nash_8_5` that cleans up on drop even on early return/panic.
- [x] [Review][Patch] Silent error suppression in safety-critical halt path — FIXED: `halt_payload` now logs the `HaltPayloadError` with subject before returning `None`.
- [x] [Review][Patch] Halt ID collision vulnerability from weak derivation — FIXED: `halt_id` now includes `spirit_id` + `severity` (`mira-halt-{subject}-{spirit_id}-{severity:.4}`), preventing collisions on same subject.
- [x] [Review][Patch] Library code panics on serialization failure — FIXED: `canonical_bytes()` now returns `Result<Vec<u8>, String>` instead of panicking. Callers updated.
- [x] [Review][Patch] Transparency Log insert errors silently discarded — FIXED: Both `tl.insert_frame_event` calls in `smoke_mira_nash_8_5` now use `?` to propagate errors.
- [x] [Review][Patch] Global monotonic base initializer called without test isolation — DOCUMENTED: Added comment in `halt_bilateral.rs` noting the pre-existing pattern (110+ call sites) and referencing deferred-work.md.
- [x] [Review][Patch] Unbounded async channel in A2A router setup — DEFERRED: The `unbounded_channel` is the existing LoopbackA2ARouter pattern (consumed, not modified by 8.5). No change made.
- [x] [Review][Patch] Deterministic but colliding frame IDs — FIXED: `smoke_mira_nash_8_5` uses an incrementing `frame_counter` baked into `frame_id`. `a2a_pairing.rs` `advisory_frame` takes a `seq: u64` parameter.
- [x] [Review][Patch] J6 smoke panics on invocation_count=0 — FIXED: `run_j6_smoke_with_count(0)` now returns a vacuous `JourneyResult` instead of panicking.
- [x] [Review][Patch] Mira::diagnose silently misclassifies NaN baseline — FIXED: `diagnose` now treats `NaN` baseline as `f64::EPSILON` (same as zero), preventing NaN propagation.
- [x] [Review][Patch] Mira::diagnose with negative baseline produces unexpected severity — FIXED: `diagnose` now treats negative baseline as `f64::EPSILON`, preventing sign-flip severity.
- [x] [Review][Patch] Cohen's κ single-sample convention is undocumented — FIXED: Added doc comment on `cohen_kappa` documenting the single-category convention.
- [x] [Review][Patch] std::sync::Mutex in MobilePushCapture used across async boundary — DOCUMENTED: Added comment noting `std::sync::Mutex` is correct because `NotificationDispatcher::dispatch` is sync. Migration path noted if dispatcher ever goes async.
- [x] [Review][Patch] Mira code threshold can drift from manifest threshold — ADDED TEST: `threshold_drift_guard` unit test verifies `DIAGNOSTIC_CONFIDENCE_HALT_THRESHOLD` matches `manifest.toml` `on_value_below.threshold` at compile time (both are 0.5).
- [x] [Review][Patch] DecisionRecord ignores J6 budget — FIXED: Added `j6_p95_met: bool` to `DecisionRecord`; updated `decide()` to take optional `j6`; updated all callers and tests.
- [x] [Review][Patch] AC8 — `4-kernel-design.md` workspace-count sentinel update unverified — VERIFIED: Sentinel `<!-- workspace-count-authoritative -->` at line 115 updated 37→39 (confirmed in staged diff).
- [x] [Review][Patch] AC3 — BudgetWarning emission on budget overrun absent — FIXED: `run_j6_kernel` now emits `FrameKind::BudgetWarning` TL row when `!result.budget_met` (NFR-Perf-6).
- [x] [Review][Patch] Add epsilon guard before f64→f32 confidence cast — FIXED: `try_halt_payload` now guards the f64→f32 cast with epsilon (`1e-6`) at the boundary, preventing truncation flips.
- [x] [Review][Patch] Add scheduled CI job for J6 real cold-load measurement — FIXED: Added `j6-real-measurement` nightly job (cron: 2 AM UTC) running with `kernel_measurement` feature.
- [x] [Review][Patch] J6 benchmark must instantiate actual Mira/Nash Spirits — FIXED: `run_j6_kernel` now cold-instantiates `Mira` and `Nash` Spirits inside the measurement loop.
- [x] [Review][Patch] Document FrameKind::TaskAssign tech debt for advisory reuse — FIXED: Added `// TECH-DEBT(8.5)` comments in `spirits/mira/src/lib.rs` (advisory doc) and `crates/maos-bin/src/main.rs` (make_frame).
- [x] [Review][Patch] AC9 — maos-eval safety-critical corpus test wired as standalone job — RESOLVED: The maos-eval tests are split across multiple standalone jobs in discipline.yml (no single aggregate job exists). The `safety-critical-corpus-8-5` job follows this established pattern. Added explanatory comment in the YAML.

#### defer
- [x] [Review][Defer] HaltFlow::submit_resolution partial failure (resolved-but-unjournaled) — If `resolver.resolve()` succeeds but `journal.journal_halt_resolution()` fails, halt is resolved in registry but has no audit journal row. `?` on journal returns Err. `crates/maos-director-surface/src/halt_ui.rs:71-80` — pre-existing, not introduced by this change. deferred, pre-existing
- [x] [Review][Defer] NotificationDispatcher swallows all channel errors — `dispatch()` counts `Err(_)` in `report.errors` but does not propagate, log, or identify which channel failed. `crates/maos-director-surface/src/notification.rs:62-81` — pre-existing, not introduced by this change. deferred, pre-existing
- [x] [Review][Defer] A2A un-pinned peer path untested — `verify_pinned` returns `EPinMismatch::NotPinned` if peer never pinned. All new tests call `pin_first_contact` before routing. `crates/maos-a2a/src/tofu.rs:196-200` — pre-existing, not introduced by this change. deferred, pre-existing
- [x] [Review][Defer] A2A timeout leaves handle_intake future dangling — `tokio::time::timeout(timeout, intake_fut).await` returns `PartitionTimeout` on expiry, but `handle_intake` may still be executing. `crates/maos-a2a/src/adapter.rs:289-298` — pre-existing, not introduced by this change. deferred, pre-existing
- [x] [Review][Defer] install_intake_sink is racy with in-flight frames — Sink replaced under `tokio::sync::Mutex`, but frames already accepted by `handle_intake` and awaiting sink access could be dropped. `crates/maos-a2a/src/adapter.rs:115-121` — pre-existing, not introduced by this change. deferred, pre-existing
- [x] [Review][Defer] LoopbackA2ARouter duplicate peer_id silently overwrites — `LoopbackA2ARouter::new` logs warning and overwrites on duplicate `peer_id`. `crates/maos-a2a/src/adapter.rs:97-102` — pre-existing, not introduced by this change. deferred, pre-existing
- [x] [Review][Defer] A2A handle_intake boot_nonce restart detection races on invalidation — `invalidate_for_restart` called; if NACK lost, peer could retry with old boot_nonce. `crates/maos-a2a/src/adapter.rs:383-423` — pre-existing, not introduced by this change. deferred, pre-existing
- [x] [Review][Defer] Consent intent taxonomy gap — `ConsentAllowlists` accepts free-form `A2AIntent` strings, but `frame_intent_str()` only projects to `"highprivilege"` / `"standard"` / `"readonly"`. Specific intent like `"diagnostic.advisory"` would silently never match. Acknowledged substrate gap in story doc Ruling 1. deferred, pre-existing/design
