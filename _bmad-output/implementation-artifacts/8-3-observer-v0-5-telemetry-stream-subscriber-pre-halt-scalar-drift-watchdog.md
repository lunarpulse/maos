---
dev_model_used: claude-opus-4-8
---

# Story 8.3: Observer v0.5 — Telemetry Stream Subscriber + Pre-Halt Scalar Drift Watchdog

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->
<!-- dev_model_used frontmatter is set/confirmed by the dev agent in AC1 (§A2 hard-fail gate). Recommended: claude-opus-4-8 (Decision H). -->

## Story

As an operator at v0.5 watching for pre-halt instability,
I want the **Observer reference Spirit** shipped in `spirits/observer/` as a **broad Telemetry-Stream subscriber** that watches `scalar.tap` for **pre-halt scalar drift** AND surfaces **structural-anomaly suspects** (sandbox-escape syscall-pattern divergence, fd-table growth, unexpected outbound IAC connections), where the drift early-warning and the anomaly suspect are **both surfaced to the operator via the existing `NotificationEvent::AnomalyFlagged` path** (the variant Story 3.4 shipped *for* Observer — its doc literally reads "full Observer wiring at Story 8.3"),
So that the **"kernel raises a structural alarm; interpretation is Spirit-side"** pattern (§4.0.7 / NFR-Sec-3) is operationalized — the operator can intervene **before** a halt fires — and the kernel itself stays non-interpretive (zero kernel KLOC; no anomaly-classification function ever enters `maos-kernel-core`).

## What this story IS and IS NOT (read first — scope is deliberately bounded)

This is the **third reference Spirit** and the **first read-only, write-restricted perceptual Spirit** (architecture §6.5: `passive-observer` — silent allow on reads, no exec, no mutate, no control-plane; the only thing Observer "emits" is an operator-facing notification). Unlike Butler (8.1, anticipatory compute) and Researcher (8.2, scoped distillation), Observer is a **watchdog**: it subscribes, detects drift/anomaly, classifies Spirit-side, and surfaces. Scope is drawn to prevent over-building (a generic anomaly-detection engine; live syscall/fd/network monitors that the v0.5 substrate does not emit) and under-building (a manifest-only stub that never subscribes to the real `scalar.tap` stream nor dispatches a real `AnomalyFlagged` notification).

**This story IS:**
- A real **Observer Spirit crate** at `spirits/observer/` (rust-inproc form, zero kernel KLOC) — `passive-observer` cognitive posture realized Spirit-side, autonomy `[posture] = cautious`, read-only, write-restricted to the operator-notification surface.
- A **broad `scalar.tap` subscription** over the real `TelemetryStreamPort` (I7 is `runtime`-operational at v0.5): Observer subscribes to the `scalar.tap.<metric>` topics it watches and receives peer `ScalarTapEvent`s, with **principal-namespace filtering applied CLIENT-SIDE** on `ScalarTapEvent.spirit_id` (FR31 — the substrate has no namespace-filtered subscription; Decision C).
- A **pre-halt scalar-drift watchdog** (the v0.5 headline acceptance demo): Observer maintains a rolling per-`(spirit_id, tag)` trajectory, detects the value **entering a watch-band approaching the halt threshold BEFORE the predicate fires**, and surfaces an early-warning so the operator can intervene. This is REAL substrate end-to-end (real `TelemetryStreamAdapter`, real `NotificationDispatcher`), because `scalar.tap` ships operational at v0.5.
- A **structural-anomaly suspect path**: Observer consumes **fixture-replayed** NFR-Sec-3 divergence inputs (syscall-pattern divergence / fd-table growth / unexpected outbound IAC — shaped as the real `FrameKind::SandboxBlock = 8` discriminator the v2.0 wiring will use), **classifies them Spirit-side**, and surfaces a `structural_anomaly_suspect`. The *interpretation of malice* is Observer-/operator-side, never kernel-side (§4.0.7).
- **Both surfaces emitted via the real `NotificationEvent::anomaly_flagged(observer, subject, summary, confidence)` constructor**, dispatched through the real `maos-director-surface` `NotificationDispatcher` (driven as a dev-dep, the 8.1/8.2 kernel-adapter-as-dev-dep pattern) into a test `NotificationChannel` that captures the event. **No new `FrameKind`, no new `FramePayload`, ABI stays frozen** (Decision B).
- **Observer fixtures** (drift-trajectory scenarios + structural-anomaly scenarios) authored under `spirits/observer/tests/fixtures/`, SHA-pinned (Story 0.3) and registered in the corpus-staleness / coverage-matrix surfaces.

**This story IS NOT:**
- It does **NOT** add live syscall / seccomp / Landlock / fd-table / outbound-network monitors. **NFR-Sec-3 is v2.0 (ADR-024), explicitly deferred** — the kernel observes *outcomes* (`FrameKind::SandboxBlock`), not individual syscalls, at v0.5. Observer's three divergence inputs are **fixture-replayed** at v0.5, exactly as Butler's calendar/comms (8.1 Decision B) and Researcher's web/arXiv (8.2 Decision B) inputs are fixture-replayed. (Decision E.)
- It does **NOT** add a `FrameKind::StructuralAnomalySuspect` or a new `FramePayload` variant. The frozen ABI has no such kind; the epic's "emits a `structural_anomaly_suspect` IAC frame" is realized through the purpose-built `AnomalyFlagged` notification (§6.5: Observer may emit *only* the operator-notification surface). Adding an ABI variant is OUT of scope and would break the v1.0 ABI freeze. (Decision B.)
- It does **NOT** add a `subscribe_namespace`/prefix method to `TelemetryStreamPort`. The port has only exact-topic `subscribe_topic`; FR31 namespace scoping is client-side at v0.5 (Decision C). A port/ABI change is its own future story.
- It does **NOT** add a §13.1 latency journey or budget. The J4 "Mira-Nash Observer colocation <10ms P95" budget is a **v1.5** obligation (Mira+Nash, Story 8.5); epic Story 8.3 carries no latency block. (Decision F.)
- It does **NOT** add distillation, drift-statistics, anomaly-classification, threshold-comparison-against-peer-policy, or any cognitive logic to `maos-kernel-core`. All Observer cognition is Spirit-side; the kernel-API surface invariant (Story 0.2) stays GREEN (any new kernel public fn = class `other` → build-break).
- It does **NOT** halt itself or declare an `[epistemic_policy]`. Observer emits no claims and never fires a halt of its own; the drift *watch* thresholds it monitors are Observer-side config, NOT the manifest epistemic policy (Decision G).

## LOCKED Design Decisions (do NOT silently re-decide — chosen during story creation; flagged for Winston)

**Decision A — Observer home `spirits/observer/` + workspace count bump 32 → 33.**
Observer lives at **`spirits/observer/`** as a new workspace member crate (mirrors Butler/Researcher Decision A). Rationale: epic-8 mandate ("Zero kernel KLOC — all subprocess Spirit code in `spirits/`"). This bumps `check-workspace-count` **32 → 33** (AC6 updates the `<!-- workspace-count-authoritative -->` sentinel in `4-kernel-design.md` 32→33). Unlike Researcher, there is **no pre-existing `observer` slot** in `tests/coverage-matrix.yaml` (8.2's `researcher` slot had a wrong path to fix; Observer is a fresh ADD). **FLAG Winston:** confirm Observer is a workspace member at `spirits/observer` (count → 33).

**Decision B — Drift early-warning AND structural-anomaly suspect BOTH emit via `NotificationEvent::AnomalyFlagged`; no new `FrameKind`/`FramePayload`.**
The epic AC3 says Observer "emits a `structural_anomaly_suspect` IAC frame," but (1) `FrameKind` has no such variant and the ABI is frozen at v1.0, and (2) architecture §6.5 says Observer "cannot send IAC frames except `notification.surface`." Both are reconciled by the **`AnomalyFlagged` notification variant** (`crates/maos-domain/src/notification.rs:50-65`), which Story 3.4 shipped expressly for Observer (its doc: "Story 3.4 — anomaly surfaced to the director by an Observer-class Spirit (full Observer wiring at Story 8.3)"). Drift early-warning (AC3) and structural-anomaly suspect (AC4) are BOTH surfaced as `AnomalyFlagged { observer, subject, summary, confidence }` via the validated constructor `NotificationEvent::anomaly_flagged(...)`, dispatched through the real `NotificationDispatcher`. The `summary` carries the classification (`"drift early-warning: …"` / `"structural_anomaly_suspect: <divergence_kind> …"`). **No new ABI surface; `abi-diff` stays Added-only/removed=[].** **FLAG Winston:** confirm `AnomalyFlagged`-notification is the v0.5 emit vehicle vs. a new IAC `FrameKind` (which would break the frozen ABI). Note the variant's `observer`/`subject` are `String` at v0.3-β; the doc invites a future typed-`SpiritId` promotion — keep `String` at v0.5 (ABI-neutral), record promotion as carry-forward.

**Decision C — FR31 principal-namespace subscription filtering is CLIENT-SIDE at v0.5.**
`TelemetryStreamPort` exposes only `publish_event` + `subscribe_topic(spirit_id, &TelemetryTopic) -> bool` (exact-topic). There is **no namespace-filtered / prefix / wildcard subscription** in the port or the `TelemetryStreamAdapter`. Observer subscribes to the set of `scalar.tap.<metric>` topics it watches and applies **principal-namespace filtering client-side** on the received `ScalarTapEvent.spirit_id` (drops emitters outside Observer's configured principal scope per FR31). This mirrors Researcher's resolved precedent ("`Scope::LogRecall` is kernel-granted, documented in manifest + comment, enforced via the adapter," 8.2). **FLAG Winston:** confirm client-side namespace filtering for v0.5 vs. adding a `subscribe_namespace`/prefix method to `TelemetryStreamPort` (a port/ABI change, out of scope).

**Decision D — Drift detection uses Observer-side watch thresholds + the scalar trajectory, NOT a delivered peer threshold.**
`ScalarTapEvent` carries `{ spirit_id, tag, value: f64, timestamp }` — **value only, no threshold** (the halt threshold lives in the *peer's* manifest `[epistemic_policy]` and in `EpistemicHaltPayload`, neither delivered over telemetry). Observer maintains a rolling per-`(spirit_id, tag)` window and fires the early-warning when the trajectory **enters a configured watch-band approaching the halt threshold** (the watch thresholds + direction are Observer-side config/fixture, since the v0.5 substrate delivers no peer epistemic-policy threshold to a colocated Observer at runtime). This realizes the I7 obligation: "Observer witnesses the runup, not just the alarm." **FLAG Winston:** confirm Observer-side watch thresholds vs. a future kernel API exposing peer `[epistemic_policy]` thresholds to colocated Observers.

**Decision E — NFR-Sec-3 v2.0 structural-anomaly SOURCES are fixture-replayed at v0.5; the real signal `FrameKind::SandboxBlock = 8` is the production target.**
NFR-Sec-3 (sandbox-escape structural anomaly detection) is **v2.0/ADR-024 — deferred**. The kernel does NOT emit syscall-pattern-divergence / fd-table-growth / unexpected-outbound-IAC telemetry at v0.5; the one real structural alarm that exists is `FrameKind::SandboxBlock = 8` (kernel-emitted exit-time seccomp/Landlock block + `emit_t3_escape_block`), written to the **Transparency Log** (not the Telemetry Stream). Because SandboxBlock frames are about the *subject* Spirit (not Observer's own emitter frames), a participant-scoped `LogRecallPort` read would `ScopeViolation`, and there is no broadcast topic for them — so **live cross-Spirit consumption is itself a v2.0 concern** (needs either a broadcast topic or a kernel-mediated broad-observer read capability). At v0.5 Observer's three divergence inputs are **fixture-replayed, shaped as the real `FrameKind::SandboxBlock` discriminator** so the v2.0 source-swap is a wiring change, not a rewrite. Observer's **detect → classify (Spirit-side) → `AnomalyFlagged` → operator-actionable** path is REAL and tested against the real `NotificationDispatcher`. **FLAG Winston:** confirm v0.5 satisfies the NFR-Sec-3-shaped obligation via fixture-replayed v2.0 sources + the real classify/emit path (live monitors + broad-observer SandboxBlock delivery deferred to v2.0; first live source ships its own conformance corpus — the 8.2 carry-forward pattern).

**Decision F — No §13.1 latency AC at v0.5.**
The §13.1 J4 "Mira-Nash Observer colocation <10ms P95" budget is a **v1.5** obligation shipped with the Mira+Nash bilateral pair (Story 8.5). Epic Story 8.3 has no latency block. Observer v0.5 carries no latency SLO and adds **no `maos-bench` journey** (unlike 8.2's J-Researcher). Drift-detection receipt latency is bounded only by the existing telemetry-test timeout convention (use ≥500ms in CI per 8.2's flake fix). The J4 colocation bench lands at 8.5.

**Decision G — Observer declares NO `[epistemic_policy]` and (likely) no inference capability; the `passive-observer` cognitive posture is Spirit-side.**
Observer emits no claims and never halts itself, so it has **no `[epistemic_policy]` rules** (Butler/Researcher declared them because they halt; Observer does not). The manifest `[posture]` autonomy = `cautious` (the most restrictive enum value — matches `passive-observer`: silent allow on reads, no exec/mutate/control-plane); the `passive-observer` *cognitive* posture is realized Spirit-side as an `ObserverPosture` marker (mirrors Researcher's `ResearcherPosture`, since `deny_unknown_fields` rejects a cognitive posture-set in `[posture]`). Observer does deterministic classification, so it needs **no live LLM**. **Verify** whether `[capabilities.required]` / `provider.complete` is mandatory-non-empty in the validator: if optional, OMIT it (Observer does no inference at v0.5); if mandatory, declare a minimal `provider.complete` documented as unused-at-v0.5. Declare **no MCP servers** (Observer has no external drivers). **FLAG Winston:** confirm `[posture]=cautious` + no `[epistemic_policy]` + no-inference manifest for the read-only Observer.

**Decision H — Recommended dev model: `claude-opus-4-8`.**
Rationale: integration-heavy story spanning the real `TelemetryStreamPort` subscription (broadcast channels + client-side namespace filter), the rolling-trajectory drift watchdog, the structural-anomaly classify path, and the real `NotificationDispatcher` emit — three real kernel/director adapters driven as dev-deps. Memory records deepseek-v4-pro is weak on async invariants / integration plumbing / port-injection threading — the in-proc Spirit→port bridge (the same risk class 8.1/8.2 navigated) recurs here. 8.1 and 8.2 both used claude-opus-4-8.

## Prerequisites (verified present at story-creation time — re-verify in AC1)

| Prerequisite | Status | Path / Evidence |
|---|---|---|
| Spirit ABI + lifecycle hooks, `#[spirit]` proc-macro, `Ctx` | ✅ PRESENT | `crates/maos-spirit-abi/src/lifecycle.rs`, `…/identity.rs` (FrameKind); `crates/maos-spirit-derive/src/lib.rs` |
| Spirit SDK + local runner + spirit-test harness + v0.5 assert macros | ✅ PRESENT | `crates/maos-spirit-sdk/src/{local_runner.rs,spirit_test/{harness.rs,assert.rs}}` |
| **`scalar.tap` Telemetry Stream** — port + concrete adapter + reference test (I7 = `runtime` at v0.5) | ✅ PRESENT | `crates/maos-domain/src/ports/telemetry.rs:12-25` (`TelemetryStreamPort`: `publish_event`, `subscribe_topic`); `crates/maos-domain/src/invariants/i7.rs:23-54` (`InvariantI7`/`TelemetryTopic`/`ScalarTapEvent`); `crates/maos-kernel-core/src/telemetry/mod.rs:78` (`TelemetryStreamAdapter`, broadcast channels, `.subscribe(&topic) -> broadcast::Receiver`); **reference test** `crates/maos-kernel-core/tests/scalar_tap_subscriber.rs` |
| Scalar emission path (peers publishing `scalar.tap.<tag>`) | ✅ PRESENT | `crates/maos-kernel-core/src/capability/mod.rs:208` — `set_scalar(...)` publishes to `TelemetryTopic::new(format!("scalar.tap.{tag}"))` |
| **`NotificationEvent::AnomalyFlagged` + validated constructor** (the Observer emit vehicle) | ✅ PRESENT | `crates/maos-domain/src/notification.rs:50-65` (variant; doc = "full Observer wiring at Story 8.3"), `:78-103` (`anomaly_flagged(observer, subject, summary, confidence)`; NaN/empty/range-validated), `:68-76` (`NotificationEventError`) |
| **`NotificationDispatcher` + `NotificationChannel` + surfaces/levels** | ✅ PRESENT | `crates/maos-director-surface/src/notification.rs:23-24` (`NotificationChannel::surface`/`dispatch`), `:63-81` (`NotificationDispatcher::dispatch(event, level)`); `crates/maos-domain/src/notification.rs:9-21` (`NotificationLevel::{Immediate,Queue,Digest}`, `NotificationSurface::{Terminal,AcpEditor,MobilePush}`) |
| **`FrameKind::SandboxBlock = 8`** (the real structural alarm shaped in fixtures; v2.0 live source) | ✅ PRESENT | `crates/maos-spirit-abi/src/identity.rs` (`FrameKind::SandboxBlock = 8`); kernel emit `crates/maos-kernel-core/src/security/mod.rs:408-424` (`emit_sandbox_block`), `…/security/sandbox/t3/cap_audit_bridge.rs:14-28` (`emit_t3_escape_block`) |
| `FramePayload::TelemetryEvent(TelemetryEventPayload{event_type,data})` (alt carrier if needed) | ✅ PRESENT | `crates/maos-domain/src/frame.rs:68,277-281` |
| `LogRecallPort` + `FrameKindLabel::SandboxBlock` (participant-scoped read; note scope wall, Decision E) | ✅ PRESENT | `crates/maos-domain/src/ports/log_recall.rs:14-28`; `crates/maos-domain/src/log_recall.rs` (`FrameKindLabel::SandboxBlock`) |
| Posture / manifest validators (`deny_unknown_fields`) | ✅ PRESENT | `crates/maos-manifest/src/manifest.rs` — `[class]`/`[capabilities.required]`/`[posture]`(Cautious/Assistive/AutonomousWithHalt/Autonomous)/`[output_shape]`/`[budget]`/`[resources]`/`[sandbox]`(T0–T4)/`[epistemic_policy]`(`on_value_above/below/within/outside`) |
| Butler + Researcher reference crates (structure to mirror) | ✅ PRESENT | `spirits/butler/{Cargo.toml,manifest.toml,src/lib.rs,tests/}`; `spirits/researcher/{…}` (the closer template: dev-dep kernel adapters, SHA-pinned fixtures) |
| Workspace count gate + authoritative sentinel | ✅ PRESENT (=32) | root `Cargo.toml` members (32 incl. `spirits/butler`,`spirits/researcher`); `xtask` `check-workspace-count`; sentinel `<!-- workspace-count-authoritative -->` in `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` (declares **32** post-8.2) |
| Kernel-API surface invariant (Story 0.2) — `check-empty-kernel` / `check-service-boundary` / `kloc-check` | ✅ PRESENT | `.github/workflows/discipline.yml` (`check-service-boundary` classifies new kernel symbols; class `other` → build-break); `xtask/src/check_service_boundary.rs` |
| CI new-spirit wiring (job + aggregate `needs:`) | ✅ PRESENT | `.github/workflows/discipline.yml` — `butler-tests`, `researcher-tests` (+`researcher-bench`) jobs wired into `aggregate` `needs:` list |
| **`spirits/observer/` Spirit + fixtures** | ❌ **ABSENT** — **this story creates them** | no `spirits/observer/` today |
| coverage-matrix `observer` slot | ❌ ABSENT (fresh ADD) | `tests/coverage-matrix.yaml` `reference_spirits` has hello/example/butler/researcher; no `observer` (no wrong-path to fix, unlike 8.2) |

## Acceptance Criteria

### AC1 — Prerequisites & scope classified mechanically before Observer work opens

**Given** the prerequisite table above
**When** AC1 runs first
**Then** the dev confirms each ✅ path/symbol still exists (`TelemetryStreamPort`/`TelemetryStreamAdapter` + `scalar_tap_subscriber.rs`, `NotificationEvent::anomaly_flagged` + `NotificationDispatcher`, `FrameKind::SandboxBlock = 8`, the manifest validators, the workspace-count sentinel = 32, the kernel-API surface gate) and records the result in the Dev Agent Record
**And** the Observer absence is confirmed (no `spirits/observer/`, no `observer` coverage-matrix slot) and Decisions A–H are recorded as the chosen resolutions, not silently re-decided
**And** `dev_model_used` is recorded/confirmed in the story frontmatter (§A2 hard-fail gate).

### AC2 — Observer Spirit ships with a broad `scalar.tap` subscription + client-side principal-namespace filter (FR31)

**Given** the Observer reference Spirit in `spirits/observer/`
**When** Observer is loaded
**Then** the manifest declares `[class]` (abi=1.0, manifest_schema_version=2, `forms=["rust-inproc"]`, trust_tier="local"), `[posture] default = allowed_max = "cautious"` (Decision G), an `[output_shape]` aligned to the anomaly surface (e.g. `required_fields = ["subject","summary","confidence"]` — verify the exact set against `RawOutputShape`), `[budget]`/`[resources]` (small read-only envelope), `[sandbox]` tier, **no `[epistemic_policy]`**, and **no MCP servers** (verify whether `[capabilities.required]`/`provider.complete` is mandatory — if optional, omit; Decision G); the manifest passes `maos-manifest` validation (each section verified against the authoritative validators before authoring — `deny_unknown_fields`)
**And** Observer subscribes **broadly** to the Telemetry Stream including `scalar.tap` via `TelemetryStreamPort::subscribe_topic(spirit_id, &TelemetryTopic::new("scalar.tap.<metric>"))` and **receives** peer `ScalarTapEvent`s (proven by a test mirroring `crates/maos-kernel-core/tests/scalar_tap_subscriber.rs`, driving the real `TelemetryStreamAdapter` as a dev-dep: subscribe → peer publishes → Observer receives within the ≥500ms test bound)
**And** the subscription is **filtered to events under Observer's principal namespace per FR31** — implemented **client-side** on `ScalarTapEvent.spirit_id` (Decision C): events from emitters outside Observer's configured principal scope are dropped (proven by a positive + a negative test: an out-of-namespace emitter's scalar is NOT surfaced).

### AC3 — Pre-halt scalar-drift watchdog surfaces an early-warning BEFORE the halt fires (the v0.5 headline demo)

**Given** Observer watches `scalar.tap` for drift with Observer-side watch thresholds (Decision D)
**When** a peer Spirit's scalar value climbs toward (but has not yet crossed) its `[epistemic_policy]` halt threshold — a realistic runup published as a sequence of `ScalarTapEvent`s over the real `TelemetryStreamAdapter`
**Then** Observer detects the trajectory **entering the watch-band approaching the threshold** and surfaces a **drift early-warning via the real `NotificationEvent::anomaly_flagged(observer="observer", subject=<peer spirit_id>, summary="drift early-warning: …", confidence)`** dispatched through the real `maos-director-surface` `NotificationDispatcher` into a test `NotificationChannel` that captures it (Decision B)
**And** the early-warning is surfaced **before** the value would cross the threshold (the test asserts the warning fires on a pre-threshold value, i.e. the operator can intervene before the halt) — realizing the epic's v0.5 acceptance demo "Observer surfaces scalar.tap drift event before halt fires" and the I7 "witness the runup, not just the alarm" obligation
**And** the emitted `confidence` is in `[0.0, 1.0]` (the constructor rejects NaN / empty summary / out-of-range — negative tests exercise each `NotificationEventError`), and a non-drifting trajectory (value flat / falling away from the band) produces **no** early-warning (negative).

### AC4 — Structural-anomaly suspects detected Spirit-side; surfaced; interpretation never kernel-side (§4.0.7)

**Given** the three NFR-Sec-3 structural-anomaly inputs — syscall-pattern divergence from manifest declaration, fd-table growth, unexpected outbound IAC connections — **fixture-replayed** and shaped as the real `FrameKind::SandboxBlock = 8` discriminator (Decision E; NFR-Sec-3 is v2.0, so live sources are out of scope)
**When** Observer processes such an input
**Then** Observer **classifies it Spirit-side** and surfaces a **`structural_anomaly_suspect`** via `NotificationEvent::anomaly_flagged(observer="observer", subject=<offending pid/spirit_id>, summary="structural_anomaly_suspect: <divergence_kind> …", confidence)` dispatched through the real `NotificationDispatcher` (the epic's "emits a `structural_anomaly_suspect` IAC frame" realized as the operator-notification surface §6.5 grants Observer — Decision B)
**And** at least one scenario per divergence kind (syscall divergence / fd-table growth / unexpected outbound IAC) is covered, plus a **benign** scenario that produces **no** suspect (negative — avoids false-positive flooding)
**And** the **interpretation of malice is Observer-/operator-side, never kernel-side (§4.0.7)** — the classification thresholds, the `confidence`, and the "is this malice?" decision live entirely in `spirits/observer/`; the kernel only ever carried the structural signal (`SandboxBlock`), never a verdict.

### AC5 — Emission vehicle is the validated `AnomalyFlagged` notification; ABI stays frozen

**Given** Decision B (drift + structural anomalies both surface via `AnomalyFlagged`)
**When** any Observer surface is emitted
**Then** it is constructed via `NotificationEvent::anomaly_flagged(...)` (NOT a struct literal — the constructor enforces NaN / empty-summary / `[0.0,1.0]` validation; struct literals bypass it) and dispatched through the real `NotificationDispatcher::dispatch(event, level)` with a defensible `NotificationLevel` (drift early-warning ⇒ `Immediate` or `Queue`; record the choice)
**And** **no new `FrameKind`, no new `FramePayload`, no new public symbol** is added to `maos-spirit-abi`/`maos-domain` for this — `abi-diff` is **Added-only with `removed=[]`** and the frozen ABI is unchanged (if any ABI delta appears, STOP and flag — the story is mis-scoped)
**And** the `observer`/`subject` fields stay `String` at v0.5 (the doc's typed-`SpiritId` promotion is recorded as carry-forward, not done here).

### AC6 — Zero kernel KLOC; kernel-API invariant holds; workspace count reconciled; manifest conforms

**Given** Observer is a rust-inproc Spirit (zero kernel KLOC)
**When** Observer's subscription, drift-trajectory, and anomaly-classification logic is added
**Then** the logic lives entirely in `spirits/observer/`, **not** in `maos-kernel-core` — the Story 0.2 kernel-API surface invariant stays GREEN (no new kernel public fn; **the kernel-API gains no anomaly-classification / drift-detection / threshold-comparison-against-peer-policy function** — any such addition would be class `other` → build-break, per epic AC4)
**And** `spirits/observer/Cargo.toml` keeps kernel/director adapters (`maos-kernel-core`, `maos-director-surface`) in `[dev-dependencies]` only (the 8.2 pattern: Spirit-side deps = `maos-spirit-sdk`+`maos-spirit-abi`+`maos-domain`+serde; integration proven via dev-dep adapters)
**And** `check-workspace-count` is reconciled to **33** (Decision A): root `Cargo.toml` members + the `<!-- workspace-count-authoritative -->` sentinel in `4-kernel-design.md` both updated 32→33
**And** the manifest passes `maos-manifest` validation with every declared section verified against the authoritative validators before authoring (do NOT invent fields — `deny_unknown_fields`; confirm `[posture]=cautious`, no `[epistemic_policy]`, and the `provider.complete` requirement per Decision G).

### AC7 — Observer fixtures authored, SHA-pinned, and registered (Story 0.3)

**Given** Observer's deterministic test inputs (no live telemetry source, no live syscall monitor in CI)
**When** the fixtures are authored under `spirits/observer/tests/fixtures/`
**Then** they include (a) **drift-trajectory scenarios** (per-`(spirit_id,tag)` value sequences with a labeled watch-band + expected fire/no-fire) and (b) **structural-anomaly scenarios** (≥1 per divergence kind shaped as `SandboxBlock`, plus benign negatives), in a stable serialized form (JSONL/JSON), generated deterministically (seeded/pure — no live LLM, no RNG; NFR-Testability-1 bit-identical) if a generator is used
**And** both fixture sets are **SHA-256-pinned per Story 0.3** (a pin test mirroring `spirits/researcher`'s `corpus_pin`/`distillate_corpus_quarterly_pin` pattern) and **registered** in the corpus-staleness / `tests/coverage-matrix.yaml` surfaces so silent edits fail loud
**And** the new `observer` slot is added to `tests/coverage-matrix.yaml` `reference_spirits` with `path: "spirits/observer"`, `ships_at: "v0.5"` (fresh ADD — no wrong path to correct).

### AC8 — CI / discipline wiring green end-to-end

**Given** the discipline gates
**When** CI runs at HEAD
**Then** an `observer-tests` job is added to `.github/workflows/discipline.yml` (`cargo test -p observer --locked`, with `timeout-minutes`) and wired into the final gate-aggregation `needs:` list (mirrors `researcher-tests`; **no** bench job — Decision F, no latency journey)
**And** `xtask check-service-boundary` (0 new violations), `check-empty-kernel` (0 violations — no new kernel public fn), `check-workspace-count` (33/33), `coverage-matrix`, `corpus-staleness`, `abi-diff` (Observer ABI-neutral, Added-only/removed=[]), `kloc-check`, and the §A2 `check-dev-model-used-populated` gate are all GREEN at HEAD — **no flipped-while-red** (the Epic 7 §A2 trap)
**And** the full `cargo test -p observer --locked` suite passes, and the Butler + Researcher regressions stay clean (0 failures)
**And** the Dev Agent Record lists every file created/modified, and any pre-existing RED is verified Observer-neutral (identical clean-HEAD-vs-changes) and flagged, not introduced.

## Tasks / Subtasks

- [x] **T1 — Prerequisite + scope pre-check (AC1)**
  - [x] Re-verify every ✅ row (paths + key symbols), especially `TelemetryStreamPort::{publish_event,subscribe_topic}` + `TelemetryStreamAdapter.subscribe`, `NotificationEvent::anomaly_flagged` + `NotificationDispatcher::dispatch`, `FrameKind::SandboxBlock = 8`, the workspace-count sentinel (=32), the manifest validators; record in Dev Agent Record
  - [x] Confirm `spirits/observer/` absent and no `observer` coverage-matrix slot; record Decisions A–H as chosen resolutions
  - [x] Confirm/set `dev_model_used` frontmatter (§A2 gate)
- [x] **T2 — Scaffold the Observer crate (AC2, AC6, Decision A/G)**
  - [x] Create `spirits/observer/` (`Cargo.toml`, `manifest.toml`, `src/lib.rs`, `tests/`, `tests/fixtures/`) mirroring `spirits/researcher/` shape. **Spirit-side deps = `maos-spirit-sdk[local_runner]` + `maos-spirit-abi` + `maos-domain` + serde/serde_json** (NO kernel/director crates in `[dependencies]`). **Dev-deps = `maos-spirit-sdk[local_runner,mock,spirit_test]` + `maos-kernel-core` (TelemetryStreamAdapter) + `maos-director-surface` (NotificationDispatcher) + `maos-manifest` + tokio/tempfile/sha2/toml.**
  - [x] Add `spirits/observer` to root `Cargo.toml` members (→33); bump the `<!-- workspace-count-authoritative -->` sentinel in `4-kernel-design.md` 32→33; run `check-workspace-count` (expect 33/33)
  - [x] Author `manifest.toml`: `[class]` (observer, 0.5.0, abi=1.0, schema=2, min_substrate_version, forms=["rust-inproc"], trust_tier="local"); `[posture] default=allowed_max="cautious"`; `[output_shape]` (anomaly surface fields — verify); `[budget]`+`[resources]` (small); `[sandbox]` tier (T2 for the testable reference form; production read-only could tighten to T1 — record + flag); **no `[epistemic_policy]`, no MCP servers**; resolve the `provider.complete` mandatory-vs-optional question against the validator (Decision G). Verify all sections via `manifest_self_check` / the `Raw*Section::from_toml_str` validators in a smoke test (`tests/spirit_smoke.rs`).
- [x] **T3 — Broad `scalar.tap` subscription + client-side namespace filter (AC2, Decision C)**
  - [x] Implement Observer's subscription over the `&dyn TelemetryStreamPort` domain trait; `subscribe_topic` to the watched `scalar.tap.<metric>` topics; receive `ScalarTapEvent`s; apply principal-namespace filter on `spirit_id` (Observer-configured scope)
  - [x] Integration test `tests/scalar_tap_subscribe.rs` driving the real `TelemetryStreamAdapter` (dev-dep), mirroring `scalar_tap_subscriber.rs`: subscribe → peer publishes → Observer receives within ≥500ms; out-of-namespace emitter dropped (negative)
- [x] **T4 — Pre-halt drift watchdog (AC3) — the headline demo**
  - [x] Implement the rolling per-`(spirit_id,tag)` trajectory + watch-band detector (Observer-side watch thresholds, Decision D); fire when the value enters the band approaching (not yet crossing) the threshold
  - [x] Integration test `tests/drift_watchdog.rs`: publish a realistic runup over the real `TelemetryStreamAdapter`; assert a `NotificationEvent::AnomalyFlagged` "drift early-warning" is dispatched through the real `NotificationDispatcher` into a capturing test `NotificationChannel` **before** the threshold-crossing value; non-drifting trajectory ⇒ no warning (negative); confidence ∈ [0,1]; constructor negatives (NaN/empty/out-of-range)
- [x] **T5 — Structural-anomaly suspect path (AC4, Decision E)**
  - [x] Implement Spirit-side classification of fixture-replayed divergence inputs (shaped as `FrameKind::SandboxBlock`); emit `structural_anomaly_suspect` via `anomaly_flagged`
  - [x] Integration test `tests/structural_anomaly.rs`: ≥1 scenario per divergence kind (syscall divergence / fd-table growth / unexpected outbound IAC) ⇒ suspect surfaced; benign scenario ⇒ no suspect (negative); assert the malice interpretation + thresholds live Spirit-side (no kernel call classifies) — §4.0.7
- [x] **T6 — Emission-vehicle + ABI-frozen proof (AC5)**
  - [x] Confirm all surfaces go through `anomaly_flagged` + real `NotificationDispatcher` with a recorded `NotificationLevel`; no struct-literal bypass in production code
  - [x] Run `abi-diff`; confirm Observer is ABI-neutral (Added-only, removed=[]); confirm no new public symbol in `maos-spirit-abi`/`maos-domain`
- [x] **T7 — Fixtures: author, SHA-pin, register (AC7)**
  - [x] Author `spirits/observer/tests/fixtures/` (drift-trajectory + structural-anomaly scenarios; deterministic generator if used, env-gated, bit-identical)
  - [x] Add a SHA-256 pin test (mirror researcher's pin); register fixtures + the new `observer` slot (`path: spirits/observer`, `ships_at: v0.5`) in `tests/coverage-matrix.yaml`; run `coverage-matrix` + `corpus-staleness` (expect PASS)
- [x] **T8 — Zero-kernel-KLOC / kernel-API invariant (AC6)**
  - [x] Confirm no `maos-kernel-core` edits; run `check-empty-kernel` + `check-service-boundary` (0 violations); confirm the kernel-API gained no anomaly/drift/threshold function
- [x] **T9 — CI / discipline green (AC8)**
  - [x] Add `observer-tests` job (`cargo test -p observer --locked`, `timeout-minutes`) + wire into the `aggregate` `needs:` list (mirror `researcher-tests`; no bench job)
  - [x] Verify all AC8 gates GREEN at HEAD; pre-existing reds verified Observer-neutral; no flipped-while-red; File List complete

## Dev Notes

### Spirit form & scaffolding (mirror Researcher 8.2 — the closer template)
- **Form: rust-inproc.** `forms = ["rust-inproc"]`. Scaffold by copying `spirits/researcher/` shape: Spirit-side deps only in `[dependencies]`; the real kernel/director adapters in `[dev-dependencies]` so integration is PROVEN without violating Story 0.2. The `#[spirit]` macro is applied to an inherent `impl` block; it synthesizes no-op bodies for unused hooks. Keep state in `Arc<Mutex<...>>` (use `unwrap_or_else(|e| e.into_inner())` on lock — the 8.2 poison-safety review fix) so the Spirit stays `Sync`.
- **`Ctx` exposes only opaque handles** (`cancellation()`, `capability()`, `mailbox()`, `deprecation_warnings()`). A lifecycle hook cannot reach kernel services directly — the telemetry subscription + notification dispatch integrations are proven in tests that drive the real adapters as **dev-dependencies** (the resolved 8.1/8.2 pattern). This is the single most likely place to lose a review cycle — do NOT reach into `maos-kernel-core`/`maos-director-surface` from `spirits/observer`'s lib.

### Telemetry Stream subscription (I7) — the core read path
- Port `TelemetryStreamPort` ([crates/maos-domain/src/ports/telemetry.rs:12-25](crates/maos-domain/src/ports/telemetry.rs)): `publish_event(&self, topic: &TelemetryTopic, event: ScalarTapEvent)` + `subscribe_topic(&self, spirit_id: &str, topic: &TelemetryTopic) -> bool` (true = newly subscribed). **Exact-topic only — no namespace/prefix subscription (Decision C: filter client-side).**
- `TelemetryTopic::new("scalar.tap.<metric>")`; `ScalarTapEvent { spirit_id: String, tag: String, value: f64, timestamp: u64 }` ([crates/maos-domain/src/invariants/i7.rs:28-54](crates/maos-domain/src/invariants/i7.rs)). **No threshold field — Decision D: watch-band thresholds are Observer-side config.** I7 phasing: **v0.5 = `runtime`** (operational) — this story exercises the real stream.
- Concrete impl `TelemetryStreamAdapter` ([crates/maos-kernel-core/src/telemetry/mod.rs:78](crates/maos-kernel-core/src/telemetry/mod.rs)) — `tokio::sync::broadcast` per-topic channels (default cap 2048); `.subscribe(&topic) -> broadcast::Receiver<ScalarTapEvent>` for receiving. **Subscribe BEFORE publishing** (the broadcast channel must exist). Reference test to mirror exactly: [crates/maos-kernel-core/tests/scalar_tap_subscriber.rs](crates/maos-kernel-core/tests/scalar_tap_subscriber.rs) — subscribe → spawn publisher (`set_scalar`) → `tokio::time::timeout(…, rx.recv())` → assert. Use a **≥500ms** timeout in CI (8.2 raised 100ms→500ms to de-flake).
- Topic naming convention: peers publish via `set_scalar` → `scalar.tap.<tag>` ([crates/maos-kernel-core/src/capability/mod.rs:208](crates/maos-kernel-core/src/capability/mod.rs)).

### The emission vehicle — `NotificationEvent::AnomalyFlagged` (Decision B; the crux)
- The variant ([crates/maos-domain/src/notification.rs:50-65](crates/maos-domain/src/notification.rs)): `AnomalyFlagged { observer: String, subject: String, summary: String, confidence: f32 }` — doc: "Story 3.4 — anomaly surfaced to the director by an Observer-class Spirit (**full Observer wiring at Story 8.3**)". This is literally built for this story.
- Construct via `NotificationEvent::anomaly_flagged(observer, subject, summary, confidence) -> Result<_, NotificationEventError>` ([:78-103]) — rejects empty/whitespace summary (`EmptySummary`), NaN (`NanConfidence`), and out-of-range (`ConfidenceOutOfRange`). **Always use the constructor** (struct-literal bypass is the negative-test path only).
- Dispatch through `maos-director-surface` `NotificationDispatcher::dispatch(event, level)` ([crates/maos-director-surface/src/notification.rs:63-81](crates/maos-director-surface/src/notification.rs)); a test `NotificationChannel` (impl `surface()` + `dispatch()`, [:23-24]) captures the dispatched event. `NotificationLevel::{Immediate,Queue,Digest}` + `NotificationSurface::{Terminal,AcpEditor,MobilePush}` ([crates/maos-domain/src/notification.rs:7-21]).
- **Why not a new `FrameKind`:** the FrameKind enum ([crates/maos-spirit-abi/src/identity.rs]) has no `structural_anomaly_suspect`; the ABI is frozen at v1.0; §6.5 restricts Observer to the notification surface. The epic's "IAC frame" language is satisfied by the notification (Decision B). If a frame really were needed, the only ABI-neutral carrier is `FramePayload::TelemetryEvent(TelemetryEventPayload{event_type,data})` ([crates/maos-domain/src/frame.rs:68,277-281]) — but the notification is the correct, purpose-built, operator-actionable path; prefer it.

### Structural-anomaly sources (NFR-Sec-3 = v2.0; Decision E)
- NFR-Sec-3 ([prd/non-functional-requirements.md] — "Sandbox-escape **structural** anomaly detection … The kernel raises a structural alarm; the *interpretation* of whether the alarm constitutes malice is Spirit-side or operator-side. The kernel does not classify intent. **v2.0 (ADR-024)**"). At v0.5 the kernel observes *outcomes*, not syscalls.
- Real signal that exists today: `FrameKind::SandboxBlock = 8` ([crates/maos-spirit-abi/src/identity.rs]; kernel emit `crates/maos-kernel-core/src/security/mod.rs:408-424` `emit_sandbox_block`, `…/security/sandbox/t3/cap_audit_bridge.rs:14-28` `emit_t3_escape_block` → `container.escape.<category>.<vector>`). It is written to the **Transparency Log** (FrameOrigin::Kernel, about the *subject* pid), NOT broadcast on telemetry. A participant-scoped `LogRecallPort` read by Observer would `ScopeViolation` (Observer isn't the emitter); there is no broadcast topic — so **live cross-Spirit delivery to Observer is itself v2.0** (needs a broadcast topic or a broad-observer read capability).
- v0.5 resolution: **fixture-replay the three divergence inputs shaped as `SandboxBlock`** (Butler/Researcher fixture-replay precedent); prove Observer's real detect→classify→`anomaly_flagged`→dispatch path. §4.0.7: classification/thresholds/`confidence` are entirely Spirit-side.

### §4.0.7 — kernel non-interpretive (the AC4/AC6 spine)
- [Source: 4-kernel-design.md#4.0.7] "The kernel does NOT interpret tag semantics … Variance, entropy, … contradiction detection — all Spirit-side. The kernel performs universal arithmetic comparison only via four predicates." And: "The kernel does NOT author cognitive content … posture inference — all Spirit-side." Observer's anomaly-classification and drift-detection therefore MUST live in `spirits/observer/`; AC6 = epic AC4 (no anomaly-classification kernel fn; class `other` → build-break).

### FR31 / I7 (subscription scoping)
- [Source: prd/functional-requirements.md FR31] principal namespace `principal:<principal_id>:<schema>` — "the kernel allocates the namespace and enforces isolation; the kernel does not index or interpret content." [Source: 3-vocabulary-invariants.md I7] "Telemetry is broadcast; subscription is per-Spirit. Pre-halt scalar trajectory observable via the `scalar.tap` stream so Observer Spirits witness the runup, not just the alarm." Substrate reality: subscription is per-Spirit exact-topic; namespace scoping is applied client-side (Decision C). FR56 (self-telemetry) is adjacent but not this story's focus.

### Observer cognitive shape (architecture §6.5)
- [Source: 6-reference-spirits.md#6.5] Observer = "Read-only perceptual layer. Subscribes broadly to the Telemetry Stream … `scalar.tap` subscription to see pre-halt scalar drift across peer Spirits. **No write capabilities by default; the Observer cannot send IAC frames except `notification.surface`** (kernel-rendered to the user)." Posture: "`passive-observer` — silent allow on all reads; no exec; no mutating; no control-plane." Use case (J4, v1.5): "Observer colocated with Nash watches `scalar.tap` from Mira; surfaces pre-halt scalar drift before Mira's halt actually fires." ⇒ manifest `[posture]=cautious` (autonomy), `ObserverPosture` Spirit-side (cognitive), no `[epistemic_policy]` (Decision G).

### Latency (Decision F — no AC)
- [Source: 13-phased-roadmap.md#13.1] J4 "Mira-Nash Observer colocation < 10ms P95" — a **v1.5** budget (ships with Story 8.5). Epic Story 8.3 has no latency block; add no `maos-bench` journey and no `*-bench` CI job.

### Testing standards
- SDK spirit-test harness + v0.5 macros (`spirit_test_assert!`, `spirit_test_expect_frame!`, `assert_no_deprecations!`). Real `TelemetryStreamAdapter`/`NotificationDispatcher` integration via dev-deps (not spirit-test, which simulates). All inputs deterministic — fixtures, no live telemetry source / syscall monitor in CI. SHA-pin fixtures per Story 0.3; register in corpus-staleness/coverage-matrix.

### Project Structure Notes
- **New crate** `spirits/observer/`: `Cargo.toml`, `manifest.toml`, `src/lib.rs`, `tests/` (spirit_smoke, scalar_tap_subscribe, drift_watchdog, structural_anomaly, fixture pin), `tests/fixtures/`. Add to root `Cargo.toml` members (→33); bump the sentinel; ADD the coverage-matrix `observer` slot.
- **No edits** to `maos-kernel-core` / `maos-director-surface` (Story 0.2). Observer logic is Spirit-side only; the real adapters are reached as dev-deps in tests.

### References
- [Source: epics/epic-8-…miranash-v03-v15.md#Story 8.3] — story statement + 4 BDD AC blocks (subscribe+namespace / drift early-warning / structural anomaly / kernel-API invariant); v0.5 acceptance demo ("Observer surfaces scalar.tap drift event before halt fires")
- [Source: architecture-maos-minimal-opus/6-reference-spirits.md#6.5 Observer] — passive-observer posture, read-only, notification.surface-only emit, scalar.tap drift use case
- [Source: architecture-maos-minimal-opus/4-kernel-design.md#4.0.7] — kernel non-interpretive (structural alarm vs interpretation); workspace-count sentinel
- [Source: prd/non-functional-requirements.md#NFR-Sec-3] — structural anomaly detection (syscall divergence / fd-table growth / outbound IAC); v2.0/ADR-024; structural-not-semantic
- [Source: prd/functional-requirements.md] — FR31 (principal namespace), FR58 (per-phase reference Spirit), FR56 (self-telemetry); [3-vocabulary-invariants.md#I7] (telemetry broadcast + scalar.tap)
- [Source: crates/maos-domain/src/notification.rs] — `NotificationEvent::AnomalyFlagged` + `anomaly_flagged` constructor + `NotificationEventError`; `NotificationLevel`/`NotificationSurface`
- [Source: crates/maos-director-surface/src/notification.rs] — `NotificationDispatcher::dispatch`, `NotificationChannel`
- [Source: crates/maos-domain/src/ports/telemetry.rs + src/invariants/i7.rs + crates/maos-kernel-core/src/telemetry/mod.rs + tests/scalar_tap_subscriber.rs] — telemetry subscription substrate + reference test
- [Source: crates/maos-spirit-abi/src/identity.rs (FrameKind::SandboxBlock=8) + crates/maos-kernel-core/src/security/mod.rs:408-424 + .../sandbox/t3/cap_audit_bridge.rs] — the real structural-alarm source (v2.0 production target)
- [Source: crates/maos-manifest/src/manifest.rs] — manifest validators (`deny_unknown_fields`; posture enum; epistemic predicate keys)
- [Source: spirits/researcher/{Cargo.toml,manifest.toml,src/lib.rs,tests/} + spirits/butler/manifest.toml] — reference crate structure to mirror (dev-dep adapters, SHA-pinned fixtures, poison-safe locks)
- [Source: .github/workflows/discipline.yml] — `researcher-tests` job + aggregate `needs:` wiring pattern; [tests/coverage-matrix.yaml] — `reference_spirits` slots (ADD `observer`)
- [Source: _bmad-output/implementation-artifacts/8-2-…-log-recall-walker.md] — the in-proc Spirit→kernel-adapter-as-dev-dep bridge pattern; Decisions A/B mirrored here; review-fix lessons (poison-safe lock, ≥500ms telemetry timeout, no in-crate-only latency)

## Dev Agent Record

### Agent Model Used

`claude-opus-4-8` (Decision H) — recorded in frontmatter `dev_model_used` per §A2.

### Debug Log References

- `cargo test -p observer --locked` → **22 tests pass** across 6 files (lib 11 + spirit_smoke 4 + scalar_tap_subscribe 2 + drift_watchdog 2 + structural_anomaly 2 + fixtures_pin 1).
- `abi-diff --base abi-baseline/v1-pre-bump.txt --json` (the CI invocation) → `passed=true, removed=0`, Observer NOT in `added` → **Observer ABI-neutral**. (NOTE: the bare `cargo run -p xtask -- abi-diff` with NO `--base` is a different default-mode comparison and is NOT the gate — only the `--base` invocation is.)
- Clean-HEAD vs changes comparison for pre-existing reds: `git stash -u` to commit 83ab525 → `abi-diff` exit 0, `kloc-check` exit 1 (current=71644); with Observer → `abi-diff` passed, `kloc-check` exit 1 (current=71644, IDENTICAL). **kloc-check is a pre-existing RED, Observer-neutral** (Observer lives in `spirits/`, not counted in the kernel KLOC ceiling).

#### AC1 — Prerequisites & scope verified (2026-06-03)

All ✅ rows re-verified by direct source read: `TelemetryStreamPort::{publish_event, subscribe_topic}` (`crates/maos-domain/src/ports/telemetry.rs:12-25`) + `TelemetryStreamAdapter::subscribe` returning `broadcast::Receiver<ScalarTapEvent>` (`crates/maos-kernel-core/src/telemetry/mod.rs:78,121`); `ScalarTapEvent{spirit_id,tag,value:f64,timestamp}` carries **no threshold** (`…/invariants/i7.rs:45-54`) — confirms Decision D; `NotificationEvent::AnomalyFlagged` + validated `anomaly_flagged(observer,subject,summary,confidence)` (`crates/maos-domain/src/notification.rs:50-103`, doc = "full Observer wiring at Story 8.3") rendered by `TerminalChannel` + dispatched by `NotificationDispatcher::dispatch(event,level)` (`crates/maos-director-surface/src/notification.rs:63-81,206-225`); `FrameKind::SandboxBlock = 8` (`crates/maos-spirit-abi/src/identity.rs:27`); `manifest_self_check` proves `[capabilities.required]`/`[output_shape]`/`[budget]`/`[resources]` are OPTIONAL and `[epistemic_policy]` is not parsed (`crates/maos-spirit-sdk/src/spirit_test/manifest.rs:46-55`) → confirms Decision G (Observer omits both). `spirits/observer/` and the `observer` coverage-matrix slot confirmed ABSENT (fresh ADD). Decisions A–H recorded as chosen resolutions. `dev_model_used: claude-opus-4-8` set in frontmatter (§A2).

**Substrate-reality findings (flagged for Winston):**
- **No namespace-filtered telemetry subscription** — `TelemetryStreamPort` has only exact-topic `subscribe_topic`; FR31 scoping is implemented CLIENT-SIDE in `PrincipalScope::admits` on `ScalarTapEvent.spirit_id` (Decision C; mirrors Researcher's "`Scope::LogRecall` kernel-granted, documented" precedent). Q3.
- **`NotificationDispatcher::dispatch` takes `event` by value + `level`** and fans out to registered channels; the structural-anomaly emit is realized as `AnomalyFlagged` (Decision B), reconciling epic AC3's "structural_anomaly_suspect IAC frame" with §6.5's notification-only authority — zero ABI delta. Q2.
- **NFR-Sec-3 sources are not emitted at v0.5** — only `FrameKind::SandboxBlock` (kernel exit-time outcome) exists, written to the TL (not telemetry) about the subject pid; cross-Spirit delivery to Observer is itself v2.0. v0.5 fixture-replays the three divergence inputs shaped as SandboxBlock (Decision E). Q5.

#### AC2–AC8 — implementation evidence

- **AC2** (`tests/scalar_tap_subscribe.rs`, 2 pass): Observer subscribes broadly to `scalar.tap.belief_variance` on the real `TelemetryStreamAdapter`, receives a peer `ScalarTapEvent` within the 500ms bound; the FR31 client-side filter surfaces the in-namespace `mira` emitter and drops `stranger`. Manifest validated in `tests/spirit_smoke.rs` (4 pass): `posture=cautious/cautious`, output_shape `[subject,summary,confidence,anomaly_kind]`, `capabilities_required_count=0`, NO `[capabilities]`/`[epistemic_policy]`, T2 sandbox; `manifest_self_check` + `ClassSection`/`PostureSection`/`SandboxConfig::from_toml_str` all clean, zero warnings.
- **AC3** (`tests/drift_watchdog.rs`, 2 pass — the v0.5 headline): a peer runup published over the real `TelemetryStreamAdapter` (`[0.40, 0.66, 0.78]` vs `belief_variance on_value_above 0.7`, band `[0.55, 0.7)`); Observer surfaces ONE drift early-warning at value **0.66 < 0.7** (pre-threshold) via the real `NotificationDispatcher` into a capturing channel, dedups the episode, and does NOT warn on the crossed value 0.78 (the kernel halt's job). Flat/low trajectory and out-of-namespace emitter ⇒ no warning. A `below`-direction scenario (`user_preference_drift on_value_below 0.6`) warns falling-toward. Constructor negatives (empty/NaN/out-of-range) exercise each `NotificationEventError`.
- **AC4** (`tests/structural_anomaly.rs`, 2 pass): all three NFR-Sec-3 divergence kinds (syscall-pattern-divergence / fd-table-growth / unexpected-outbound-iac), fixture-replayed shaped as `FrameKind::SandboxBlock` (asserted `== FrameKind::SandboxBlock as u8`), classified Spirit-side and surfaced as `structural_anomaly_suspect` via the real `NotificationDispatcher`; benign (sub-floor) and out-of-namespace signals produce no suspect. The suspect verdict/threshold/confidence live entirely in `spirits/observer/` (§4.0.7).
- **AC5**: every surface is built via the validated `anomaly_flagged` constructor (no struct-literal bypass in lib) and dispatched as `AnomalyFlagged` at **`NotificationLevel::Immediate`** (chosen because drift early-warnings and structural suspects are time-sensitive operator interventions — the operator should see them before a halt fires or a structural anomaly escalates); `abi-diff` confirms NO new public symbol in `maos-spirit-abi`/`maos-domain` (removed=0, Observer absent from added). `observer`/`subject` kept `String` (typed-`SpiritId` promotion = carry-forward).
- **AC6**: `check-empty-kernel` PASSED (0 violations) + `check-service-boundary` PASSED (0 violations) — no kernel public fn added; the kernel-API gained no anomaly/drift/threshold-vs-peer-policy function (epic AC4). Observer's `Cargo.toml` keeps `maos-kernel-core`/`maos-director-surface` in `[dev-dependencies]` only. `check-workspace-count` PASSED **33/33** (members + `4-kernel-design.md` sentinel both 32→33).
- **AC7**: `tests/fixtures/{drift-scenarios.json, structural-scenarios.json}` authored; SHA-256 manifest-pinned by `tests/fixtures_pin.rs` (PIN `1742ac1f…bb6b9dd2`); the `observer` slot added to `tests/coverage-matrix.yaml` `reference_spirits` (`path: spirits/observer`, `ships_at: v0.5`); `coverage-matrix` + `corpus-staleness` exit 0.
- **AC8**: `observer-tests` job added to `.github/workflows/discipline.yml` (`cargo test -p observer --locked`, `timeout-minutes: 15`) + wired into the `aggregate` `needs:` list (no `*-bench` job — Decision F). All gates GREEN at HEAD: check-service-boundary, check-empty-kernel, check-workspace-count (33/33), coverage-matrix, corpus-staleness, abi-diff (Observer-neutral), check-dev-model-used-populated (PASS). Butler + Researcher regressions clean (0 failures). **Pre-existing reds verified Observer-neutral:** `kloc-check` exit 1 at clean HEAD == with-Observer (current=71644 identical). No flipped-while-red.

### Completion Notes List

- Observer is the **third reference Spirit** and the first **read-only perceptual** Spirit. The whole watchdog is pure Spirit-side cognition over `maos-domain` types; the real `TelemetryStreamAdapter` (scalar.tap) and real `NotificationDispatcher` (operator surface) are driven as **dev-deps** in `tests/` — the resolved 8.1/8.2 in-proc-bridge pattern, here with NO `maos-audit`/`maos-kernel-core` in production deps.
- **Decision B is the crux and it held:** `NotificationEvent::AnomalyFlagged` (shipped by Story 3.4 "for full Observer wiring at Story 8.3") is the single emit vehicle for both drift early-warnings and structural suspects — no new `FrameKind`/`FramePayload`, ABI frozen.
- Carried 8.1/8.2 lessons: poison-safe `Mutex` locks (`unwrap_or_else(|e| e.into_inner())`), ≥500ms telemetry test timeouts (the 8.2 CI-flake fix), kernel adapters as dev-deps only, fixtures SHA-pinned + deterministic.
- The `maos-manifest` crate has a pre-existing `clippy` issue surfaced under `cargo clippy -p observer --all-targets` (a dev-dep of Observer); it is NOT in Observer's code and there is no clippy gate in `discipline.yml`. The `observer-tests` job runs `cargo test`, which is clean.

### File List

**Created — Observer crate (`spirits/observer/`, workspace member #33):**
- `spirits/observer/Cargo.toml` — pure Spirit-side deps (spirit-sdk/abi + maos-domain + serde); kernel/director adapters dev-deps only; NO `[epistemic_policy]`/MCP/inference deps.
- `spirits/observer/manifest.toml` — read-only v0.5 envelope (`posture=cautious`, output-shape anomaly surface, T2, no capabilities/epistemic_policy — Decision G).
- `spirits/observer/src/lib.rs` — `Observer` Spirit (`#[spirit] on_idle`), `ObserverPosture`, `PrincipalScope` (FR31), `WatchThreshold`/`DriftDirection` (Decision D), `DivergenceKind`/`StructuralSignal`, `ObserverSurface`/`AnomalyKind`, `observe_scalar` drift watchdog, `classify_signal` structural path, `SANDBOX_BLOCK_FRAME_KIND` bound to the ABI; 11 lib unit tests.
- `spirits/observer/tests/spirit_smoke.rs` — on_idle watchdog pass + manifest validators (AC2/AC6).
- `spirits/observer/tests/scalar_tap_subscribe.rs` — broad subscription + FR31 client-side filter vs real `TelemetryStreamAdapter` (AC2).
- `spirits/observer/tests/drift_watchdog.rs` — pre-halt drift early-warning before the halt via real `NotificationDispatcher` + constructor negatives (AC3/AC5).
- `spirits/observer/tests/structural_anomaly.rs` — 3 divergence kinds + benign/FR31 negatives + SandboxBlock-discriminator binding (AC4/AC5).
- `spirits/observer/tests/fixtures_pin.rs` — SHA-256 fixture pin (AC7).
- `spirits/observer/tests/fixtures/drift-scenarios.json` — 4 drift trajectories (AC7).
- `spirits/observer/tests/fixtures/structural-scenarios.json` — 5 structural signals (AC7).

**Modified:**
- `Cargo.toml` — workspace members += `spirits/observer` (→33).
- `Cargo.lock` — new member resolved.
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` — workspace-count sentinel 32→33.
- `tests/coverage-matrix.yaml` — `observer` reference-spirits slot (`path: spirits/observer`, ships_at v0.5) + fixture-PIN note.
- `.github/workflows/discipline.yml` — `observer-tests` job + wired into the `aggregate` `needs:` list (no bench job, Decision F).
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — 8.3 ready-for-dev → in-progress → review.
- `_bmad-output/implementation-artifacts/8-3-…-watchdog.md` — Tasks, Dev Agent Record, Change Log, Status.

### Change Log

| Date | Change |
|---|---|
| 2026-06-03 | Story 8.3 created (AC1–AC8). Observer v0.5 = third reference Spirit + first read-only perceptual Spirit at `spirits/observer/` (workspace 32→33). 8 LOCKED decisions A–H: (B) drift + structural anomalies both emit via the purpose-built `NotificationEvent::AnomalyFlagged` (no new FrameKind; ABI frozen); (C) FR31 namespace filtering client-side (no port method exists); (D) Observer-side watch thresholds (ScalarTapEvent carries no threshold); (E) NFR-Sec-3 sources fixture-replayed (v2.0/ADR-024 deferred; SandboxBlock=8 is the real production target); (F) no §13.1 latency AC (J4 <10ms is v1.5); (G) no `[epistemic_policy]`, `[posture]=cautious`, no-inference manifest. Headline demo = pre-halt scalar-drift early-warning over the real `scalar.tap` stream (I7 runtime at v0.5) surfaced before the halt fires. |
| 2026-06-03 | Story 8.3 implemented (AC1–AC8) and shipped to review. NEW `observer` crate at `spirits/observer/` (workspace 32→33): `PrincipalScope` FR31 client-side filter, `WatchThreshold`/`observe_scalar` pre-halt drift watchdog (warns in-band before the halt), `classify_signal` structural-anomaly path over fixture-replayed SandboxBlock-shaped inputs, all surfaced via the existing `NotificationEvent::AnomalyFlagged` through the real `NotificationDispatcher` (Decision B; ABI frozen). 22 observer tests pass (11 lib + 4 smoke + 2 scalar_tap + 2 drift + 2 structural + 1 pin); proven against the real `TelemetryStreamAdapter` + `NotificationDispatcher` dev-deps. `observer-tests` CI job added + wired into aggregate. All AC8 gates GREEN at HEAD (service-boundary/empty-kernel 0 violations, workspace-count 33/33, coverage-matrix, corpus-staleness, abi-diff Observer-neutral, dev-model-used PASS); Butler+Researcher regressions clean. Pre-existing RED `kloc-check` verified Observer-neutral (current=71644 identical at clean HEAD). 8 LOCKED decisions A–H honored; 3 substrate-reality findings flagged for Winston (no namespace-filtered subscription; AnomalyFlagged-as-emit-vehicle; NFR-Sec-3 v2.0 fixture-replay). |

### Review Findings

**Decision-needed:**

- [x] [Review][Patch] NaN scalar values silently coerced to 0.0 — changed to return `None` (drop silently, no substitution). `spirits/observer/src/lib.rs` (blind+edge) — resolved: user chose "drop silently"

**Patch:**

- [x] [Review][Patch] Doc-comment says seeded inputs are "drained" but `on_idle` only borrows `&self` — fixed to "evaluates." `spirits/observer/src/lib.rs`
- [x] [Review][Patch] `below_band()` renamed to `outside_band()` for clarity. `spirits/observer/src/lib.rs` (blind+edge)
- [x] [Review][Patch] Added `principal_scope_prefix_wildcard_matches` test. `spirits/observer/src/lib.rs` (edge)
- [x] [Review][Patch] Added `unexpected_frame_kind_surfaces_as_suspect` test. `spirits/observer/src/lib.rs` (edge)
- [x] [Review][Patch] Added `#[serde(deny_unknown_fields)]` to `DriftScenario`, `EventFixture`, `StructuralScenario`. `spirits/observer/tests/drift_watchdog.rs`, `structural_anomaly.rs` (edge)
- [x] [Review][Patch] Added `drift_below_direction_reset_allows_rewarning` test. `spirits/observer/src/lib.rs` (edge)
- [x] [Review][Patch] Added `drift_below_direction_confidence_value` test asserting proximity. `spirits/observer/src/lib.rs` (edge)
- [x] [Review][Patch] Added `with_id_overrides_observer_id` test. `spirits/observer/src/lib.rs` (edge)
- [x] [Review][Patch] Recorded `NotificationLevel::Immediate` choice + rationale in AC5 evidence. `8-3-observer-…-watchdog.md` (auditor)

**Deferred:**

- [x] [Review][Defer] `WatchThreshold::new` accepts NaN/Inf `threshold` — all comparisons return false, silently disabling drift detection. Code-constructed config (not user input); a NaN threshold is a programming error. `spirits/observer/src/lib.rs:742-758` — deferred, not user-facing
- [x] [Review][Defer] `CapturingChannel` in tests uses `.lock().unwrap()` — inconsistent with production poison-safe pattern. Test-only code; pre-existing pattern across all Spirit test doubles. `spirits/observer/tests/drift_watchdog.rs:1386` — deferred, test-only
- [x] [Review][Defer] `StructuralSignal` has no constructor validation — `magnitude` outside `[0.0, 1.0]` silently clamped. Fixture-controlled at v0.5; the clamp is defensive. `spirits/observer/src/lib.rs:249-263` — deferred, fixture-controlled
- [x] [Review][Defer] Multiple watches for same tag — only first `find()` match is used. Observer is always constructed with unique tags; configuration correctness issue. `spirits/observer/src/lib.rs:458` — deferred, config correctness
- [x] [Review][Defer] Empty `PrincipalScope` silently drops all events. Observer is always constructed with at least one pattern. `spirits/observer/src/lib.rs:89-125` — deferred, config correctness
- [x] [Review][Defer] Empty `observer`/`subject` strings not validated by `anomaly_flagged` constructor — pre-existing gap in `maos-domain`, not introduced by this story. `crates/maos-domain/src/notification.rs:81-103` — deferred, pre-existing
- [x] [Review][Defer] `NotificationEvent` `#[non_exhaustive]` wildcard branch in `TerminalChannel` is dead code — pre-existing, not introduced by this story. `crates/maos-director-surface/src/notification.rs:226-228` — deferred, pre-existing

## Questions / Clarifications for the Architect (Winston)

1. **Decision A (workspace count → 33):** confirm Observer is a workspace member at `spirits/observer` (mirrors Butler/Researcher) — bumps `check-workspace-count` 32→33 and ADDs a fresh `observer` slot to the coverage-matrix `reference_spirits` (no wrong path to correct, unlike 8.2's researcher slot).
2. **Decision B (emit vehicle):** confirm both the drift early-warning AND the structural-anomaly suspect are surfaced via `NotificationEvent::AnomalyFlagged` (kernel-rendered, operator-actionable — the variant Story 3.4 shipped "for full Observer wiring at Story 8.3"), reconciling the epic's "emits a `structural_anomaly_suspect` IAC frame" with §6.5's notification-only authority and keeping the v1.0 ABI frozen (no new `FrameKind`/`FramePayload`). Keep `observer`/`subject` as `String` at v0.5 (typed-`SpiritId` promotion = carry-forward)?
3. **Decision C (FR31 namespace filtering):** confirm client-side filtering on `ScalarTapEvent.spirit_id` for v0.5, since `TelemetryStreamPort` has only exact-topic `subscribe_topic` (no `subscribe_namespace`/prefix). A port method would be its own ABI/port change.
4. **Decision D (drift thresholds):** confirm Observer-side watch thresholds + scalar trajectory (the event carries value only), vs. a future kernel API exposing peer `[epistemic_policy]` thresholds to colocated Observers.
5. **Decision E (NFR-Sec-3 sources):** confirm v0.5 satisfies the NFR-Sec-3-shaped obligation via fixture-replayed divergence inputs (shaped as the real `SandboxBlock=8`) + the real classify/emit path — with live syscall/fd/outbound monitors AND broad-observer SandboxBlock delivery deferred to v2.0 (ADR-024). Cross-Spirit SandboxBlock today would `ScopeViolation` under the participant-scoped `LogRecallPort` / isn't on telemetry.
6. **Decision G (manifest shape):** confirm `[posture]=cautious` + NO `[epistemic_policy]` + NO MCP servers for the read-only Observer, and whether `[capabilities.required]`/`provider.complete` is mandatory-non-empty (if optional, Observer omits it — it does no inference at v0.5). Sandbox tier: T2 for the testable rust-inproc reference form, or tighten to T1 given read-only posture?
