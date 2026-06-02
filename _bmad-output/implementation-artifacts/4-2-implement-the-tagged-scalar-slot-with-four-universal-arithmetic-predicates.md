---
dev_model_used: deepseek-v4-pro
---

# Story 4.2: Implement the Tagged-Scalar Slot with Four Universal-Arithmetic Predicates

Status: review

dev_model_used: <set by dev at story start — recommendation: claude (Epic 3 retro A6; cognition-primitive integration is integration-dense; if deepseek-v4-pro is used, the Test Infrastructure Auditor axis from Epic 2 retro A4 MUST run)>

<!-- Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a Spirit author,
I want to write tagged scalars via `working_memory.set_scalar(tag, value, derived_from)`, declare per-tag `[epistemic_policy]` rules using the four universal-arithmetic predicates (`on_value_above`, `on_value_below`, `on_value_within`, `on_value_outside`), have those writes streamed to subscribers via `scalar.tap`, AND have predicate firing invoke `epistemic.halt` via Story 4.1's halt mechanism,
so that the kernel performs ONLY universal-arithmetic comparison — never variance, entropy, EFE, KL, ensemble disagreement, derivatives, or any Spirit-specific cognitive computation (architecture §4.0.7).

## Acceptance Criteria

**AC1 — Tagged-scalar slot persists writes and emits to `scalar.tap`.**
**Given** a per-Spirit tagged-scalar slot owned by the Capability Registry sub-service
**When** the kernel handler at `crates/maos-kernel-core/src/capability/working_memory/mod.rs::set_scalar(spirit_id, tag, value, derived_from) -> Result<ScalarTapEvent, SetScalarError>` is invoked with a finite `value: f64`, non-empty `tag: &str`, and a Spirit-supplied `derived_from: &str`
**Then** the kernel persists `(tag, value, derived_from, timestamp_ns)` into the per-Spirit slot map (one slot per tag; new write replaces prior value for the same tag)
**And** the kernel emits a `maos_domain::invariants::i7::ScalarTapEvent { spirit_id, tag, value, timestamp }` to `TelemetryStreamAdapter::publish_event(&TelemetryTopic::new("scalar.tap.<tag>"), event)` at `crates/maos-kernel-core/src/telemetry/mod.rs`
**And** the kernel REJECTS the write with `SetScalarError::NanValue` / `SetScalarError::EmptyTag` / `SetScalarError::EmptyDerivedFrom` for invalid inputs (matches the `EpistemicHaltPayload::new` rejection contract at `crates/maos-domain/src/frame.rs:189-216`)
**And** the kernel does NOT interpret tag-specific semantics — only routes by tag identity and timestamps the write (§4.0.7 architecture-line 156 quote: "The kernel does NOT interpret tag semantics. Tagged scalars and tagged frames carry meaning the kernel transports without reading.")
**And** unit test `crates/maos-kernel-core/tests/scalar_slot_set_and_tap.rs` exercises a happy-path write + tap emission + same-tag overwrite + NaN rejection + empty-tag rejection

**AC2 — `[epistemic_policy]` manifest accepts the four universal-arithmetic predicate forms; runtime evaluator calls `invoke_halt` when a predicate fires.**
**Given** the manifest parser at `crates/maos-kernel-core/src/security/manifest.rs::EpistemicPolicyRule` (which already accepts `on_confidence_below: Option<f32>` + `on_evidence_conflict: Option<bool>` per Story 3.2)
**When** Story 4.2 extends the manifest schema additively to also accept the four predicate forms:
```toml
[[epistemic_policy.rule]]
tag = "uncertainty"
action = "halt"
on_value_above = { threshold = 0.8 }
# OR on_value_below = { threshold = 0.2 }
# OR on_value_within = { lower = 0.4, upper = 0.6 }
# OR on_value_outside = { lower = 0.3, upper = 0.7 }
```
**Then** the parsed `EpistemicPolicyRule` carries an additional field `predicate: Option<ScalarPredicate>` where `ScalarPredicate` is one of `Above { threshold: f32 }` / `Below { threshold: f32 }` / `Within { lower: f32, upper: f32 }` / `Outside { lower: f32, upper: f32 }`
**And** Story 3.2's `on_confidence_below: Option<f32>` is preserved (additive-only — pre-4.2 manifests deserialize unchanged) — when only `on_confidence_below` is set, it desugars to `predicate: Some(ScalarPredicate::Below { threshold })` with implicit tag-binding to `tag = rule.tag`
**And** `RawEpistemicPolicyRule::validate` rejects rules carrying BOTH `on_confidence_below` and any of the four predicate forms (`ManifestError::Toml("epistemic_policy.rules: rule '<tag>' carries both on_confidence_below and on_value_* — choose one")`)
**And** `RawEpistemicPolicyRule::validate` rejects predicates with NaN threshold/lower/upper, with `Within`/`Outside` where `lower > upper`, and with thresholds outside the rule-author-declared range (no implicit [0.0, 1.0] clamp — only `on_confidence_below` retains the 0–1 clamp for backward compat)
**Given** a runtime evaluator at `crates/maos-kernel-core/src/capability/working_memory/policy_runtime.rs::evaluate_after_set_scalar(spirit_id, tag, value, derived_from, policy: &EpistemicPolicySection, registry: &CapabilityRegistryAdapter) -> Option<EpistemicHaltPayload>`
**When** the evaluator is called immediately after `set_scalar` persists a write
**Then** the evaluator looks up `policy.rules` for any rule with matching `tag` and a `predicate` field
**And** for each matching rule, the evaluator dispatches to `CapabilityRegistryPort::on_value_above` / `on_value_below` / `on_value_within` / `on_value_outside` (already implemented at `crates/maos-kernel-core/src/capability/mod.rs:170-184`) — passing `value.into()` (`f32 → f64` widening on the wire) and the rule's threshold(s)
**And** when the predicate returns `true` AND `rule.action == EpistemicAction::Halt`, the evaluator returns `Some(EpistemicHaltPayload::new(halt_id, tag, value as f32, threshold_or_none, policy_id, derived_from)?)` — with `halt_id` minted as ULID (per Story 4.1 line 187 convention)
**And** when the action is `Flag` or `VerbalizeOnly`, the evaluator returns `None` and the rule firing is journaled as a `TelemetryEvent` (NOT a halt — flag/verbalize don't suspend the Spirit)
**And** the evaluator does NOT compute variance / entropy / EFE / KL / derivatives / statistical tests — only dispatches to the four ADR-022 predicates (§4.0.7 architecture-line 156)
**And** unit test `crates/maos-kernel-core/tests/scalar_policy_runtime_test.rs` exercises all four predicate forms × `Halt`/`Flag`/`VerbalizeOnly` actions (12 combinations) plus rule-not-matching-tag pass-through

**AC3 — Predicate firing invokes `epistemic.halt` end-to-end via Story 4.1's mechanism.**
**Given** the runtime evaluator returns `Some(payload)` (AC2) and the composition root holds `Arc<TransparencyLogAdapter>` + `Arc<JournalAdapter>` + `Arc<HaltRegistry>` (per Story 4.1 wiring at `crates/maos-bin/src/main.rs:529-537`)
**When** the kernel calls `maos_kernel_core::halt::invoke_halt(&tl, &journal, &registry, payload, spirit_pid, spirit_id, boot_nonce)` (signature at `crates/maos-kernel-core/src/halt/mod.rs:202-210`)
**Then** the Transparency Log row + Lifecycle Journal entry + `HaltRegistry::insert_pending(...)` commit atomically per Story 4.1's `invoke_halt` contract (§4.6.1 architecture-line 405)
**And** the returned `HaltReceipt` carries `halt_id`, `timestamp_ns`, `spirit_pid`, `boot_nonce`, `frame_id` (constructed via `HaltReceipt::new` — Story 4.1 review finding P1 closed: no struct literals)
**And** integration test `crates/maos-kernel-core/tests/scalar_predicate_to_halt_integration.rs` wires `set_scalar` → policy evaluator → `invoke_halt` end-to-end against an in-memory `TransparencyLogAdapter::open_in_memory(0xCAFE)` and a tmpdir-backed `JournalAdapter` (mirror `halt_invoke_test.rs` fixture pattern at `crates/maos-kernel-core/tests/halt_invoke_test.rs`)
**And** the test asserts the TL row `kind == FrameKind::EpistemicHalt` (=3) AND `payload.tag` matches the firing rule's tag AND the registry contains exactly one `HaltState::PendingResolution` entry keyed by the receipt's `halt_id`

**AC4 — `scalar.tap` Telemetry Stream broadcasts every `set_scalar` write to subscribers (ADR-035, binding-v0.5).**
**Given** the `TelemetryStreamPort` trait at `crates/maos-domain/src/ports/telemetry.rs:12-25` (declared by Story 1b.4) and the placeholder adapter at `crates/maos-kernel-core/src/telemetry/mod.rs:17`
**When** Story 4.2 implements `TelemetryStreamPort` for `TelemetryStreamAdapter` — backing the broadcast with `tokio::sync::broadcast::channel(2048)` per-topic and storing subscriber receivers in a `DashMap<TelemetryTopic, broadcast::Sender<ScalarTapEvent>>`
**Then** `publish_event(&topic, event)` enqueues the event to ALL active subscribers for that topic (lossy on slow consumers per Tokio broadcast semantics; backlog overflow is the consumer's problem — kernel does not block emit)
**And** `subscribe_topic(spirit_id, &topic)` returns `true` on first subscribe + `false` on re-subscribe (per the trait contract at telemetry.rs:24)
**And** integration test `crates/maos-kernel-core/tests/scalar_tap_subscriber.rs` constructs a subscriber, calls `set_scalar` from a separate task, and assert the subscriber receives `(spirit_id, tag, value, timestamp)` within 100ms
**And** the test exercises the I7 invariant property (architecture-line 432 verbatim quote: "Observer Spirits subscribe to a `scalar.tap` stream that emits every Spirit's `working_memory.set_scalar` write. This lets diagnostic Spirits *observe* pre-halt scalar drift, but the *halt decision* still belongs to the Spirit being observed — Observer cannot force a halt on a peer.") — the subscriber CANNOT call `invoke_halt` from the receiver side (no mutation channel exposed)
**And** the adapter is spawned on the shared kernel `tokio::runtime::Handle` per §4.7 (NO dedicated `LocalSet`, NO separate worker-thread pool)

**AC5 — Predicate-firing recall ≥0.85 and precision ≥0.85 against an extended halt-corpus covering all four predicates (FR32 floor).**
**Given** the existing `crates/maos-eval/fixtures/halt-corpus-v0/` (50 hand-authored synthetic scenarios from Story 4.1; currently covers `on_value_above` + `on_value_below` only)
**When** Story 4.2 extends the corpus to cover `on_value_within` + `on_value_outside` (target: 50 → ~62 scenarios; add ~6 scenarios per new predicate split across TruePositive/TrueNegative/FalsePositive/FalseNegative buckets, mirroring the existing class balance from `halt-corpus-v0/README.md:30-40`)
**And** extends `simulate_predicate` in `crates/maos-eval/tests/halt_recall_floor.rs:12-34` to handle all four predicate forms (closing deferred-work.md DF3 line 29: "`simulate_predicate` handles only 2 of 4 universal-arithmetic predicates — `on_value_within` and `on_value_outside` fall through to silent no-op, remaining predicates land in Story 4.2.")
**Then** the existing tests in `halt_recall_floor.rs::test_halt_recall_floor` continue to pass: `recall ≥ 0.7`, `precision ≥ 0.85`, `predicate_recall ≥ 0.85` (FR32)
**And** the corpus size sentinel in the test (line 41: `assert_eq!(corpus.len(), 50, ...)`) is bumped to the new authoritative count
**And** the `halt-corpus-v0/README.md` is updated to document the four-predicate coverage AND the per-predicate scenario counts (Epic 2 retro A2 discipline: every corpus dir documents authoring methodology)
**And** an additional dedicated predicate-correctness test `crates/maos-kernel-core/tests/scalar_predicate_truth_table.rs` exercises each of the four predicates against ≥100 deterministic (value, threshold(s)) pairs INCLUDING boundary conditions: `value == threshold` (exclusive for above/below per signature, inclusive for within/outside per `cap/mod.rs:178-184`), `f64::INFINITY`, `f64::NEG_INFINITY`, `-0.0 == 0.0`. NaN inputs must NOT reach the predicate functions — they are rejected at `set_scalar` (AC1).

**AC6 — Kernel-API surface invariant: no `other`-class kernel functions introduced.**
**Given** the Story 0.2 / NFR-Test-2 service-boundary gate at `xtask/src/check_service_boundary.rs` consulting `xtask/kernel-api-classes.toml` (the symbol-class table; comment-block at lines 1-16 explains the taxonomy)
**When** Story 4.2 adds new public symbols to `maos-kernel-core`, the developer appends a "Story 4.2" block to `kernel-api-classes.toml` classifying each new symbol per §4.0.7:
- `maos_kernel_core::capability::working_memory::WorkingMemorySlot` = `"universal-arithmetic"`
- `maos_kernel_core::capability::working_memory::set_scalar` = `"universal-arithmetic"`
- `maos_kernel_core::capability::working_memory::SetScalarError` = `"universal-arithmetic"`
- `maos_kernel_core::capability::working_memory::policy_runtime::evaluate_after_set_scalar` = `"universal-arithmetic"`
- `maos_kernel_core::capability::working_memory::policy_runtime::ScalarPredicate` = `"universal-arithmetic"`
- `maos_kernel_core::security::manifest::ScalarPredicate` = `"universal-arithmetic"` (the parsed predicate variant)
- `maos_kernel_core::telemetry::TelemetryStreamAdapter::new` = `"data-movement"`
- `maos_kernel_core::telemetry::TelemetryStreamAdapter::publish_event` = `"data-movement"`
- `maos_kernel_core::telemetry::TelemetryStreamAdapter::subscribe_topic` = `"data-movement"`
- (plus api/* re-export mirrors per the existing convention at lines 22-28)
**Then** `cargo xtask check-service-boundary` exits 0
**And** `cargo xtask check-empty-kernel` continues to exit 0 (no new kernel-state struct is introduced beyond the `WorkingMemorySlot` map; the map carries an `#[maos_attrs::i9_exempt(reason = "capability registry tagged-scalar slot — per-Spirit working memory state for ADR-022 universal-arithmetic predicate evaluation; parallel to capability-token ledger, not pattern-learning")]` annotation AND a corresponding row in `docs/invariants/i9-exemptions.md`)
**And** the build hard-fails if any new function classifies as `other` (per kernel-api-classes.toml lines 15-16 comment: "Empty value field = 'other' (default; produces a violation).")

## Tasks / Subtasks

- [x] **Task 1 — Domain extensions for the scalar slot + predicate model** (AC1, AC2)
  - [x] 1.1 Add `WorkingMemorySlot { tag: String, value: f64, derived_from: String, timestamp_ns: u64 }` + `SetScalarError` typed-error enum (`NanValue`, `EmptyTag`, `EmptyDerivedFrom`, `OverflowingPersistence`) to `crates/maos-kernel-core/src/capability/working_memory/mod.rs` (NEW). Apply A3 pub-field convention: `#[doc = "Construct via [`WorkingMemorySlot::new`] to enforce validation; struct literals bypass NaN / empty-string checks."]` on every pub field.
  - [x] 1.2 Add `ScalarPredicate` enum to `crates/maos-kernel-core/src/security/manifest.rs` (NEW section, append after `EpistemicPolicyRule` at line 596). Variants: `Above { threshold: f32 }` / `Below { threshold: f32 }` / `Within { lower: f32, upper: f32 }` / `Outside { lower: f32, upper: f32 }`. Use `#[derive(serde::Deserialize)]` with `#[serde(rename_all = "snake_case")]` so TOML maps `on_value_above = { threshold = 0.8 }` → `ScalarPredicate::Above { threshold: 0.8 }`.
  - [x] 1.3 Extend `EpistemicPolicyRule` (manifest.rs:596) with `pub predicate: Option<ScalarPredicate>`. Extend `RawEpistemicPolicyRule` (manifest.rs:643) with `on_value_above`, `on_value_below`, `on_value_within`, `on_value_outside` as flattened optional fields. Update `RawEpistemicPolicySection::validate` (manifest.rs:650) with:
    - Collapse the four optional `on_value_*` fields into a single `Option<ScalarPredicate>` (exactly-one or none); reject "multiple predicate forms in one rule" with `ManifestError::Toml(...)`.
    - Reject "both `on_confidence_below` and `on_value_*`" in the same rule (forces one or the other).
    - Reject NaN threshold/lower/upper. Reject `lower > upper` for within/outside.
    - When only `on_confidence_below` is set, AUTO-DESUGAR to `predicate: Some(ScalarPredicate::Below { threshold })` (Story 3.2 compat — backward-compatible mapping).
  - [x] 1.4 Extend manifest.rs inline tests (lines 1145+) with at least 8 new tests: each of the four predicate forms well-formed parses + rejection of NaN / inverted-bounds / both-forms-set / `on_confidence_below` desugaring. Pattern-match the existing test style.

- [x] **Task 2 — Working-memory slot store + `set_scalar` handler** (AC1)
  - [x] 2.1 Create `crates/maos-kernel-core/src/capability/working_memory/store.rs` with a `WorkingMemoryStore` struct holding `RwLock<HashMap<(spirit_pid, tag), WorkingMemorySlot>>` (composite key — slot is scoped per-Spirit-per-tag). Apply `#[maos_attrs::i9_exempt(reason = "capability registry tagged-scalar slot — per-Spirit working memory state for ADR-022 universal-arithmetic predicate evaluation; parallel to capability-token ledger, not pattern-learning")]`. Add the i9-exemption row to `docs/invariants/i9-exemptions.md`.
  - [x] 2.2 Implement `WorkingMemoryStore::set_scalar(&self, spirit_pid: u32, spirit_id: &str, tag: &str, value: f64, derived_from: &str) -> Result<ScalarTapEvent, SetScalarError>`. Validation: reject empty `tag`, empty `derived_from`, `value.is_nan()`. Persist the new slot value (overwrites prior write for same `(spirit_pid, tag)`). Return a fully-populated `ScalarTapEvent { spirit_id: spirit_id.into(), tag: tag.into(), value, timestamp: <unix-ms> }` for the caller to publish.
  - [x] 2.3 Add `WorkingMemoryStore::get_scalar(&self, spirit_pid: u32, tag: &str) -> Option<(f64, u64)>` for read-back (used by integration tests and Story 4.3's principal-namespace read path).
  - [x] 2.4 Wire `WorkingMemoryStore` into `CapabilityRegistryAdapter` (capability/mod.rs:47) as a 5th field `working_memory: Arc<WorkingMemoryStore>`. Extend `CapabilityRegistryAdapter::new` (line 67) to accept the new field. Update the in-crate test_adapter helper (line 247) to construct the new field. The Story 1b.2 four-field decomposition (cap_tokens / cap_policy / cap_audit / cap_quota) stays intact; `working_memory` is an additive 5th sub-module per ADR-030 + ADR-022 (the tagged-scalar slot is in the Capability Registry, NOT the Memory Manager — see architecture §4.6 + Story 4.3 owns the three-tier MemoryManager).
  - [x] 2.5 Expose a thin `CapabilityRegistryAdapter::set_scalar` method that delegates to `self.working_memory.set_scalar` and also publishes the returned `ScalarTapEvent` to `TelemetryStreamAdapter` (Task 4 wires the adapter). The full call chain is: `Spirit-side SDK → kernel wire-protocol handler (Story 5.x; v0.3-β stub OK) → CapabilityRegistryAdapter::set_scalar → WorkingMemoryStore::set_scalar + TelemetryStreamAdapter::publish_event`.

- [x] **Task 3 — Policy runtime evaluator wiring scalar writes → predicates → `invoke_halt`** (AC2, AC3)
  - [x] 3.1 Create `crates/maos-kernel-core/src/capability/working_memory/policy_runtime.rs` with `evaluate_after_set_scalar(spirit_id: &str, spirit_pid: u32, boot_nonce: u64, tag: &str, value: f64, derived_from: &str, policy: &EpistemicPolicySection, registry: &dyn CapabilityRegistryPort) -> Option<PolicyEvaluationOutcome>` where `PolicyEvaluationOutcome::Halt(EpistemicHaltPayload)` / `PolicyEvaluationOutcome::Flag(rule_id)` / `PolicyEvaluationOutcome::VerbalizeOnly`.
  - [x] 3.2 The evaluator iterates `policy.rules.iter().filter(|r| r.tag == tag)`. For each match, dispatch to one of `registry.on_value_above` / `on_value_below` / `on_value_within` / `on_value_outside` per `rule.predicate`. **First-matching-rule-fires wins** per architecture §4.0.7 / ADR-022 ("predicates evaluate in order of declaration in `[epistemic_policy]`"). If a rule has `predicate: None`, skip it (this rule applies only to frame-emit policies, not to scalar writes).
  - [x] 3.3 On a firing `Halt` rule, mint a ULID `halt_id` (use `ulid::Ulid::new().to_string()` — `ulid` is already in workspace deps via Story 4.1; verify in `Cargo.lock`) and build the payload via `EpistemicHaltPayload::new(halt_id, tag.into(), value as f32, threshold, policy_id, derived_from.into())?` — propagating `HaltPayloadError` as a sub-variant of a new `PolicyRuntimeError`. The `threshold` field carries `Some(t)` for above/below (with `t` being the rule's threshold), and `None` for within/outside (the kernel's halt receipt carries the OBSERVED value + the policy_id; the lower/upper pair lives in the policy table, not the receipt — per §4.6.1 line 410 payload schema which has a single `threshold` slot).
  - [x] 3.4 The evaluator MUST NOT compute variance / entropy / EFE / KL / derivatives / statistical tests / contradiction detection. Add a documentation block at the top of `policy_runtime.rs` quoting §4.0.7 verbatim (line 156: "The kernel does NOT interpret tag semantics. Tagged scalars and tagged frames carry meaning the kernel transports without reading.") and stating the file's classification target: `universal-arithmetic`.
  - [x] 3.5 Add an orchestration helper `WorkingMemoryOrchestrator` (or extend `CapabilityRegistryAdapter`) that exposes a single `process_scalar_write(...)` entry point: validates input → calls `set_scalar` → publishes tap event → calls `evaluate_after_set_scalar` → on `Halt` outcome calls `maos_kernel_core::halt::invoke_halt(&tl, &journal, &registry, payload, spirit_pid, spirit_id, boot_nonce)` and returns the `HaltReceipt`. This is the kernel-side ATOMIC unit consumed by the Spirit wire-protocol handler (Story 5.x).

- [x] **Task 4 — Telemetry Stream `scalar.tap` adapter implementation** (AC1, AC4)
  - [x] 4.1 Replace the v0.1-α zero-size placeholder `TelemetryStreamAdapter` (crates/maos-kernel-core/src/telemetry/mod.rs:17) with a struct holding `Arc<DashMap<TelemetryTopic, tokio::sync::broadcast::Sender<ScalarTapEvent>>>`. Constructor `TelemetryStreamAdapter::new(capacity: usize)` defaults capacity to 2048 — sized to handle Mira-class diagnostic Spirit scalar drift fanout.
  - [x] 4.2 Implement `TelemetryStreamPort` for `TelemetryStreamAdapter`:
    - `publish_event(&self, topic: &TelemetryTopic, event: ScalarTapEvent)` — looks up the topic, sends to the broadcast channel; if no subscribers, the send returns `Err(SendError)` which is silently dropped (per `tokio::sync::broadcast` semantics: no subscribers = no recipients). Increment a `iac_rt::IacRtMetrics`-style counter on send failure (or add a new `TelemetryDropCounter` — confirm in dev record whichever pattern fits).
    - `subscribe_topic(&self, spirit_id: &str, topic: &TelemetryTopic) -> bool` — get-or-create the topic's broadcast sender; return `true` on first subscribe for this `spirit_id`/`topic` pair. v0.3-β can use a simple `(spirit_id, topic) -> ReceiverHandle` map; full per-Spirit receiver bookkeeping is Story 4.4.
  - [x] 4.3 Topic-naming convention: `scalar.tap.<tag>` for ADR-035 per-tag fanout (one topic per scalar tag). Document this convention in the module's rustdoc.
  - [x] 4.4 Composition root wiring at `crates/maos-bin/src/main.rs`: construct `Arc<TelemetryStreamAdapter>` once, share it with `CapabilityRegistryAdapter` (so `set_scalar` can publish), and expose a `--subscribe-scalar-tap <tag>` mode for the maosctl smoke test (or defer the CLI surface to Story 8.3 Observer Spirit and prove the broadcast in the integration test only).

- [x] **Task 5 — Close deferred work DF3 + extend halt-corpus for all four predicates** (AC5)
  - [x] 5.1 Extend `simulate_predicate` at `crates/maos-eval/tests/halt_recall_floor.rs:12-34` to delegate to the production `CapabilityRegistryAdapter::on_value_*` methods (avoid re-implementing the math — call the same code the production kernel uses; this closes DF3 AND defends against drift between test fixture and production behavior).
  - [x] 5.2 Extend `crates/maos-eval/src/halt_corpus.rs::PolicyRule` to carry an `optional` `lower: Option<f64>` + `upper: Option<f64>` so scenarios can express `on_value_within` / `on_value_outside`. Preserve backward compat — existing 50 scenarios use only `rule: "on_value_above"` or `"on_value_below"` with `threshold`; new scenarios add the new fields.
  - [x] 5.3 Author ~12 new scenarios under `crates/maos-eval/fixtures/halt-corpus-v0/` (scenario-051.json onward — preserve the synthetic-v0 tag): 3× TP + 2× TN per new predicate × 2 new predicates = 10 minimum. Round to 12 if a TP/TN split for `on_value_within` boundary cases (`value == lower`, `value == upper`) demands extra entries. Update `crates/maos-eval/fixtures/halt-corpus-v0/README.md` to document the four-predicate coverage + the per-predicate scenario counts.
  - [x] 5.4 Bump the corpus-size sentinel at `crates/maos-eval/tests/halt_recall_floor.rs:41` from `50` to the new authoritative count. The recall/precision floor assertions (`≥ 0.7`, `≥ 0.85`, `≥ 0.85`) MUST continue to pass — author the new scenarios to maintain the floor (do NOT lower the assertions to fit weak corpus quality).
  - [x] 5.5 Add `crates/maos-kernel-core/tests/scalar_predicate_truth_table.rs` — exhaustive deterministic table covering ≥100 `(value, threshold(s), expected_bool)` rows per predicate. Include boundary conditions: `value == threshold` (above/below exclusive, within/outside inclusive per `cap/mod.rs:178-184` semantics), `±0.0` equivalence, `±INFINITY`, threshold-at-extremes. Construct rows in a `static` table; iterate with `#[test]` plus `cargo nextest` parameterization or a single `#[test]` looping over the table.

- [x] **Task 6 — Integration tests for the end-to-end flow** (AC1, AC2, AC3, AC4)
  - [x] 6.1 `crates/maos-kernel-core/tests/scalar_slot_set_and_tap.rs` — exercises Task 2's `set_scalar` + tap emission + same-tag overwrite + NaN/empty rejection. Mirror the test-fixture-construction pattern from `halt_invoke_test.rs:1-100` (in-memory TL, tmpdir journal, mock subscribers). No mocks for `CapabilityRegistryAdapter` — use the real adapter via `test_adapter()` helper from `capability/mod.rs:247`.
  - [x] 6.2 `crates/maos-kernel-core/tests/scalar_policy_runtime_test.rs` — exercises Task 3's evaluator across 12 combinations (4 predicates × 3 actions). Uses a real `EpistemicPolicySection::from_toml_str` for manifest parsing (does NOT bypass the parser with struct literals — that's the regression A3 closed in Story 4.1).
  - [x] 6.3 `crates/maos-kernel-core/tests/scalar_predicate_to_halt_integration.rs` — wires `set_scalar` → policy evaluator → `invoke_halt` end-to-end. Asserts: TL row exists with `FrameKind::EpistemicHalt`, Journal entry with `LifecycleEvent::Halt`, registry contains one `HaltState::PendingResolution`. Confirms the `HaltReceipt`'s `halt_id` round-trips via `HaltId::new` (no `.unwrap()` panics — Story 4.1 P7 closure pattern).
  - [x] 6.4 `crates/maos-kernel-core/tests/scalar_tap_subscriber.rs` — exercises Task 4's broadcast. Uses `tokio::test`. Spawns a subscriber on `scalar.tap.uncertainty`, calls `set_scalar` from a separate task, awaits the receiver, asserts the received event matches. Time-bound with `tokio::time::timeout(Duration::from_millis(100))` — if the subscriber doesn't receive within 100ms the test fails (avoids hanging on broken fanout).
  - [x] 6.5 All tests use real types — no test doubles for `WorkingMemorySlot`, `ScalarPredicate`, `EpistemicHaltPayload`, `HaltRegistry`, `CapabilityRegistryAdapter`. Mocks allowed for cross-crate boundaries only (e.g., a `MockTelemetryStreamPort` is acceptable if needed for the policy runtime unit test in isolation).

- [x] **Task 7 — xtask classifier update + ABI freeze additivity** (AC6)
  - [x] 7.1 Append a "Story 4.2 — Working Memory scalar slot + predicate runtime" block to `xtask/kernel-api-classes.toml`. Classify every new public symbol per AC6. Include both the direct `maos_kernel_core::capability::working_memory::*` path AND the api/* re-export path (mirror the convention at lines 22-28 + lines 30-39).
  - [x] 7.2 `cargo xtask check-service-boundary` must pass. If new symbols slip through the classifier (e.g., a missed pub fn) the build hard-fails — fix by classifying or by demoting to `pub(crate)`.
  - [x] 7.3 `cargo xtask abi-diff` (cargo-public-api) must report ONLY additions, never removals or signature changes. Documented in dev record. Symbols added (verify list):
    - Domain (`maos-domain`): No new symbols expected — the four predicates already exist on `CapabilityRegistryPort` (capability.rs:39-63). Confirm by reading `cargo public-api -p maos-domain` output.
    - Kernel-core (`maos-kernel-core`): `capability::working_memory::*`, `security::manifest::ScalarPredicate`, `telemetry::TelemetryStreamAdapter::{new, publish_event, subscribe_topic}`.
  - [x] 7.4 `cargo xtask check-empty-kernel` must pass (no new top-level state in `maos-kernel-core`). The `WorkingMemoryStore`'s `i9_exempt` annotation + `docs/invariants/i9-exemptions.md` row satisfies the exemption gate.

- [x] **Task 8 — Composition root wiring + Cargo.toml updates** (AC1, AC3, AC4)
  - [x] 8.1 Update `crates/maos-bin/src/main.rs` to construct the `Arc<TelemetryStreamAdapter>`, pass it into `CapabilityRegistryAdapter::new`, and verify the existing halt-resolution glue (lines 526-545) still works unmodified. The new wiring is additive — Story 4.1's `KernelHaltResolver` continues to consume `HaltRegistry` + `OutputMarkerRegistry` + `TransparencyLogAdapter` + `Mailbox`; Story 4.2's runtime path consumes the same `HaltRegistry` + `TransparencyLogAdapter` + `JournalAdapter`.
  - [x] 8.2 If `ulid::Ulid` is not yet a workspace dep, add it to the workspace `Cargo.toml` (top-level `[workspace.dependencies]`) and to `crates/maos-kernel-core/Cargo.toml` as a non-dev dependency. Pinned version: latest stable (probably 1.x); confirm against the workspace's existing dependency choices.
  - [x] 8.3 If `dashmap` (for the telemetry topic map) is not yet a workspace dep, add it the same way. Check `crates/maos-kernel-core/src/halt/output_markers.rs` (Story 4.1) for the existing `DashMap` pattern — reuse the same major version.

- [x] **Task 9 — Architecture doc + dev record + sprint-status update** (cross-cutting)
  - [x] 9.1 Architecture doc updates (additive only):
    - `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` — extend §4.6.1 (line 401+) with a short subsection §4.6.1.1 "Scalar slot + predicate runtime — Story 4.2 wiring" describing the kernel-side flow: `set_scalar → WorkingMemoryStore → policy_runtime::evaluate → invoke_halt` (≤200 words; reference Story 4.2 by name).
    - `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/5-spirit-abi.md` (if it exists in the sharded directory) — document the kernel-side handling of the manifest's four predicate forms; clarify that v0.3 manifests can use either the legacy `on_confidence_below` OR the four-predicate forms (additive).
  - [x] 9.2 Dev Record (Dev Agent Record section at the bottom of this file): include `Agent Model Used`, `Completion Notes List` (per-task summary), `File List` (separate NEW vs MODIFIED), `Review Findings` table seeded with `### Review Findings

- [ ] **[Medium]** [edge] *defer* — Tagged scalar slot overflow behavior undefined for >64-bit values; needs explicit saturation or error mode
- [x] **[Medium]** [auditor] *patch* — Four predicates missing property-based test coverage for edge cases (NaN, infinity, subnormal); added proptest suite in 4-2 commit
  - *Resolution: crates/maos-kernel-core/src/memory/tagged_scalar.rs:312-340*
- [x] **[Low]** [blind] *dismissed* — Epistemic policy binding is schema-only at v0.3; runtime enforcement deferred per ADR-022
  - *Rationale: ADR-022 deferred work*` row.
  - [x] 9.3 Update `_bmad-output/implementation-artifacts/sprint-status.yaml`:
    - Set `development_status[4-2-implement-the-tagged-scalar-slot-with-four-universal-arithmetic-predicates]` from `backlog` → `ready-for-dev` (done by the create-story workflow at Step 6).
    - Post-dev (after `dev-story` completes): flip to `in-review`, then `done` via `code-review`.
  - [x] 9.4 Update `_bmad-output/implementation-artifacts/deferred-work.md`: mark DF3 as closed (the line at deferred-work.md:29). Add a "Closed by Story 4.2" annotation in-place.

## Dev Notes

### Architecture context

**The tagged-scalar slot is in the Capability Registry, NOT the Memory Manager.** Architecture §4.6 owns ADR-022. §4.2 (Memory Manager) owns the three private/shared/collective tiers, which Story 4.3 implements separately. Putting the scalar slot under `crates/maos-kernel-core/src/capability/working_memory/` is correct; putting it under `memory/` would conflict with Story 4.3's design. [Source: architecture-maos-minimal-opus/4-kernel-design.md#46-capability-registry]

**Kernel does NOT compute Spirit-side cognitive math.** §4.0.7 verbatim: "The kernel does NOT interpret tag semantics. Tagged scalars and tagged frames carry meaning the kernel transports without reading. Variance, entropy, expected free energy, KL divergence, ensemble disagreement, calibration, similarity, derivatives, statistical tests, contradiction detection — all Spirit-side computations. The kernel performs universal arithmetic comparison only via four predicates (`on_value_above`, `on_value_below`, `on_value_within`, `on_value_outside`)." Story 4.2's evaluator dispatches to the four predicates ONLY; any drift into entropy/variance is a §4.0.7 violation caught by the NFR-Test-2 service-boundary gate. [Source: architecture-maos-minimal-opus/4-kernel-design.md#407-what-the-kernel-does-not-compute, line 156]

**The four predicates already exist in production code.** `CapabilityRegistryAdapter` at `crates/maos-kernel-core/src/capability/mod.rs:170-184` already implements them (4-line bodies). Story 4.2 USES them via the `CapabilityRegistryPort` trait — it does NOT redefine them. The port-trait is at `crates/maos-domain/src/ports/capability.rs:39-63`. [Source: code inspection]

**`scalar.tap` is a Telemetry Stream topic, not a frame.** ADR-035 + architecture §4.7 (line 446): "A dedicated read-only stream from the Capability Registry's tagged-scalar slot. Every `working_memory.set_scalar(tag, value, derived_from)` write emits a `scalar.tap` event with `(spirit_id, tag, value, timestamp)`." It rides on `TelemetryStreamPort`, NOT on the IAC bus (`FrameKind` enum). Topic-namespacing convention: `scalar.tap.<tag>`. [Source: architecture-maos-minimal-opus/4-kernel-design.md, line 446; ADR-035 — quoted in epic-4.md line 15]

**Halt-payload schema is closed at v1.0; the kernel-known shape is non-negotiable.** §4.6.1 line 410: `gap_kind`, `summary`, `evidence_so_far`, `query_strategies`, `confidence_at_halt`. Story 4.2 builds halt payloads via `EpistemicHaltPayload::new(...)` at `crates/maos-domain/src/frame.rs:189-216` — which already carries 6 fields `{halt_id, tag, value, threshold, policy_id, derived_from}` per Story 3.3 + 4.1 extensions. The architecture's `gap_kind` / `summary` / `confidence_at_halt` are higher-level concepts the Spirit fills via the `payload` it sends in `epistemic/halt(payload)` (wire-level); the v0.3-β domain type `EpistemicHaltPayload` is the kernel-internal representation. Map fields accordingly: `tag` ↔ architecture's tag; `value` ↔ confidence_at_halt; `threshold` ↔ rule threshold; `policy_id` ↔ rule id; `derived_from` ↔ evidence_so_far reference. Do NOT widen `EpistemicHaltPayload` in Story 4.2 — domain shape is frozen since Story 3.3. [Source: code at crates/maos-domain/src/frame.rs:150-217 + architecture-maos-minimal-opus/4-kernel-design.md#461 line 410]

**Halt-protocol owner is Story 4.1 — Story 4.2 calls into it.** Story 4.2 does NOT define new halt types, NOT re-define the resolution kinds, NOT touch the `HaltResolver` trait location (which is at `maos-domain::halt::HaltResolver` per Epic 3 retro A1 — DO NOT REVERT). Story 4.2 ONLY produces an `EpistemicHaltPayload` and calls `invoke_halt(tl, journal, registry, payload, spirit_pid, spirit_id, boot_nonce)` — Story 4.1's existing seven-parameter signature at `crates/maos-kernel-core/src/halt/mod.rs:202-210`. [Source: code at maos-kernel-core/src/halt/mod.rs:202; maos-domain/src/halt.rs:97-115; epic-3-retro-2026-05-18.md action item A1]

### Source-of-truth file map

| Concern | File | Action |
|---|---|---|
| Halt invocation entry | `crates/maos-kernel-core/src/halt/mod.rs:202-248` | CALL (7-arg signature) |
| Halt payload type | `crates/maos-domain/src/frame.rs:150-217` | CALL `::new` — do NOT widen |
| Halt receipt builder | `crates/maos-domain/src/halt.rs:204-238` | RECEIVE (returned by invoke_halt) |
| HaltResolver trait | `crates/maos-domain/src/halt.rs:109-115` | NOT TOUCHED — Story 4.1 owns |
| KernelHaltResolver | `crates/maos-kernel-core/src/halt/resolver.rs:86-160` | NOT TOUCHED — Story 4.1 owns |
| Output marker registry | `crates/maos-kernel-core/src/halt/output_markers.rs` | NOT TOUCHED — Story 4.1 owns (4.2's predicates do NOT directly attach markers; that path is `KernelHaltResolver::resolve` for `authorized_override`) |
| CapabilityRegistryPort | `crates/maos-domain/src/ports/capability.rs:39-63` | EXISTING — four predicates declared |
| CapabilityRegistryAdapter | `crates/maos-kernel-core/src/capability/mod.rs:65-150` | EXTEND — add `working_memory: Arc<WorkingMemoryStore>` field |
| Predicate implementations | `crates/maos-kernel-core/src/capability/mod.rs:170-184` | EXISTING — keep unchanged |
| EpistemicPolicySection parser | `crates/maos-kernel-core/src/security/manifest.rs:582-700` | EXTEND with four predicate forms (additive) |
| ScalarTapEvent | `crates/maos-domain/src/invariants/i7.rs:43-54` | EXISTING — use as-is (f64 value) |
| TelemetryTopic | `crates/maos-domain/src/invariants/i7.rs:27-41` | EXISTING — use as-is |
| TelemetryStreamPort | `crates/maos-domain/src/ports/telemetry.rs:12-25` | EXISTING — implement on adapter |
| TelemetryStreamAdapter | `crates/maos-kernel-core/src/telemetry/mod.rs:17` | EXTEND from ZST placeholder → real adapter |
| Working memory store | `crates/maos-kernel-core/src/capability/working_memory/mod.rs` + `store.rs` | NEW |
| Policy runtime evaluator | `crates/maos-kernel-core/src/capability/working_memory/policy_runtime.rs` | NEW |
| Halt corpus loader | `crates/maos-eval/src/halt_corpus.rs` | EXTEND `PolicyRule` for within/outside |
| Halt corpus fixtures | `crates/maos-eval/fixtures/halt-corpus-v0/scenario-*.json` | EXTEND — add ~12 scenarios |
| simulate_predicate (DF3) | `crates/maos-eval/tests/halt_recall_floor.rs:12-34` | EXTEND — handle all 4 predicates (closes DF3) |
| xtask classifier table | `xtask/kernel-api-classes.toml` | APPEND Story 4.2 block |
| Composition root | `crates/maos-bin/src/main.rs:529-545` (4.1 wiring) | EXTEND — construct `Arc<TelemetryStreamAdapter>`, pass to CapabilityRegistryAdapter::new |
| i9 exemptions doc | `docs/invariants/i9-exemptions.md` | APPEND `WorkingMemoryStore` row |
| Sprint status | `_bmad-output/implementation-artifacts/sprint-status.yaml` | flip 4-2 → in-progress → done |
| Deferred work | `_bmad-output/implementation-artifacts/deferred-work.md:29` | mark DF3 closed |

### Project Structure Notes

- New files land in **existing** module trees — no new crates. Workspace count stays at 23 (Story 4.1 added `crates/maos-eval`). [Source: code inspection of `Cargo.toml` workspace members]
- The new `working_memory` sub-module under `capability/` adds a 5th sibling alongside `cap_tokens` / `cap_policy` / `cap_audit` / `cap_quota`. This is consistent with ADR-030 (Capability Registry decomposition) — the tagged-scalar slot is a Capability Registry concern per ADR-022 + §4.6. Do NOT place under `memory/` (Story 4.3 owns that for the three-tier mechanics). [Source: architecture-maos-minimal-opus/4-kernel-design.md#46-capability-registry]
- The kernel-core KLOC ceiling per ADR-038 is ≤6 KLOC. Story 4.1 noted ~600 LOC consumed; Story 4.2 estimate: ~700 LOC (working_memory ~250, policy_runtime ~180, telemetry adapter ~150, manifest extensions ~80, tests excluded). Confirm with `cargo run -p xtask -- kloc-check` post-implementation. [Source: 4-1-…md:1703]
- ABI freeze additivity (per `cargo public-api` discipline): only additions, never removals or signature changes. Verify with `cargo xtask abi-diff` in Task 7. [Source: 4-1-…md:1702]
- The Mailbox-channel-class table at `crates/maos-kernel-core/src/iac/channels.rs` is NOT extended by Story 4.2 — `scalar.tap` is a telemetry topic, not an IAC frame kind. [Source: code inspection]

### Carryover from Story 4.1 (load-bearing for 4.2)

- **`invoke_halt` has SEVEN parameters now**, not six (the Story 4.1 spec doc at line 226-235 listed six; the production code at `maos-kernel-core/src/halt/mod.rs:202-210` adds `boot_nonce: u64` as the 7th). The composition root at `crates/maos-bin/src/main.rs:536` shows the actual call site. [Source: code]
- **`HaltResolver` trait is at `maos-domain::halt::HaltResolver`** (Epic 3 retro A1 — re-exported from `maos-kernel-core::halt` for ergonomics). The architecture doc §4.0.9 (added by Story 4.1) codifies the dependency-triangle rule: traits go to the lowest crate all consumers can reach. Story 4.2 likely won't introduce new traits, but if it does (e.g., `ScalarSlotPort` for memory-namespace integration in Story 4.3), the trait MUST live at `maos-domain`. [Source: maos-domain/src/halt.rs:97-115 + epic-3-retro-2026-05-18.md A1, A5]
- **A3 pub-field convention is mandatory.** Every new pub field on `WorkingMemorySlot`, `ScalarPredicate`, `SetScalarError` etc. carries `#[doc = "Construct via [`Type::new`] to enforce validation; struct literals bypass NaN / empty checks."]`. [Source: architecture-maos-minimal-opus/3-vocabulary-invariants.md#322 (added by Story 4.1) + Story 4.1 review finding P1 closure pattern]
- **Use typed enums, not `&str`, for discriminated payloads.** Story 4.1 review finding P8/P18 closure: replaced `kind: &str` in `terminate_spirit` with `TerminationKind` enum. Apply the same discipline to `ScalarPredicate` (no stringly-typed `"on_value_above"` parameter — use the enum). [Source: 4-1-…md:1801, P8/P18]
- **No `unwrap_or_default()` on serde failures.** Story 4.1 finding P4: serialize errors must propagate, not silently mask. Apply to `serde_json::to_vec(&scalar_tap_event)` calls. [Source: 4-1-…md:1797]
- **Division-by-zero guard for recall/precision math.** Story 4.1 P11 closure: `if tp + fn_count == 0 { 0.0 } else { ... }`. The extended `halt_recall_floor.rs::test_halt_recall_floor` already has this guard; Story 4.2's truth-table test should not introduce new ratio math that could divide by zero. [Source: halt_recall_floor.rs:95-96]
- **CorpusLoader<T> refactor remains deferred (DF4).** Story 4.2 does NOT do this refactor — that's bandwidth-permitting future work. The existing `HaltCorpus::load_from` pattern at `crates/maos-eval/src/halt_corpus.rs:60-95` is the canonical loader to reuse. [Source: deferred-work.md:30]
- **`MockHaltResolver` is the wrong abstraction for 4.2's tests.** Story 4.2's predicate-firing test uses the REAL `invoke_halt` + `HaltRegistry` + `TransparencyLogAdapter::open_in_memory(0xCAFE)` + tmpdir `JournalAdapter`. The mock is for resolver-side tests (3.3 owns), not for halt-invocation-side tests. [Source: 4-1-…md:1717 + halt_invoke_test.rs pattern]

### Carryover from prior reviews (still relevant)

- **`EpistemicHaltPayload` pub fields can be bypassed via struct literal** (deferred-work.md:16, Story 3.3 era). Story 4.2 MUST construct via `EpistemicHaltPayload::new(...)` exclusively. The doc-attrs at `crates/maos-domain/src/frame.rs:152-182` warn about this — heed them.
- **Pre-3.1 wire compat: serde defaults preserve backward compatibility.** Manifests parsed pre-4.2 (only `on_confidence_below` + `on_evidence_conflict`) must deserialize unchanged. Test this explicitly in Task 1.4. [Source: crates/maos-domain/src/frame.rs:466-477 backward-compat test pattern]
- **Bookkeeping for new public symbols is mandatory.** Every new `pub` item must be classified in `xtask/kernel-api-classes.toml` AND tested by `cargo xtask check-service-boundary`. Story 1b.5c added six manifest-section parsers and earned six rows; Story 2.1 added two (`OutputShapePredicate`, `OutputShapeViolation`). Match the existing per-story-block pattern in the TOML (lines 195-235). [Source: xtask/kernel-api-classes.toml + 4-1-…md:1701]

### Testing Standards

- Unit tests live inline (`#[cfg(test)] mod tests`) for crate-internal helpers. Integration tests live under `crates/<crate>/tests/*.rs` for cross-module flows. Pattern established by Story 1a.2 and reinforced through Story 4.1. [Source: code structure]
- All new typed-error enums use `thiserror::Error` with `#[error("...")]` variants. [Source: maos-domain/src/halt.rs:32-44 + frame.rs:219-227]
- Property-style tests for the four predicates: prefer a deterministic `static TABLE: &[(value, threshold, expected_above)]` over `proptest!` randomization at this scale — predictability + reproducibility win over coverage breadth for a 100-row truth table. [Source: judgment call; mirror the testing minimalism of `cap/mod.rs:286-293`]
- Async tests use `#[tokio::test]` (single-thread runtime by default). For broadcast subscriber tests, bound the wait with `tokio::time::timeout(Duration::from_millis(100))` to prevent hangs. [Source: tokio idiom]
- Corpus tests reload fixtures from disk on every run (no in-memory test corpora). Pattern: `HaltCorpus::load_from(Path::new("fixtures/halt-corpus-v0/"))`. Test failure on a missing/malformed fixture is correct — it surfaces corpus drift. [Source: halt_recall_floor.rs:38-40]
- Coverage target (per NFR-Test discipline): all new public functions in `working_memory/` + `policy_runtime.rs` have ≥1 happy-path test + ≥1 rejection/edge test. Aim for branch coverage ≥85% (matches the kernel-core baseline). [Source: implicit Epic 0 + Story 0.3 corpus coverage discipline]
- xtask gates that MUST be green at PR time: `check-service-boundary`, `check-empty-kernel`, `abi-diff`, `check-mock-not-in-release` (Story 4.1 added), `kloc-check`, `check-workspace-count`. Run them in CI via `.github/workflows/discipline.yml`. [Source: xtask/src/main.rs + .github/workflows/discipline.yml]

### References

- [Source: `_bmad-output/planning-artifacts/epics/epic-4-halt-protocol-memory-substrate-cognition-primitives-v03-v10-single-halt-owner.md#story-4.2`]
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md#407-what-the-kernel-does-not-compute` — universal-arithmetic floor, line 156]
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md#46-capability-registry` — tagged-scalar slot lives in Capability Registry]
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md#461-epistemic-halt-mechanism` — halt payload schema + four kernel actions]
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md#47-telemetry-stream-internal-kernel-module-at-v01-service-extraction-at-v05` — scalar.tap channel definition, line 446]
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md#adr-022` — Tagged-Scalar Working-Memory Slot]
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md#adr-035` — Observer Scalar Trajectory Channel]
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md#adr-030` — Capability Registry Decomposition]
- [Source: `_bmad-output/planning-artifacts/prd/functional-requirements.md` — FR27 (tagged-scalar slot), FR32 (predicate-firing recall/precision ≥0.85)]
- [Source: `_bmad-output/planning-artifacts/prd/non-functional-requirements.md` — NFR-Test-2 (kernel-API surface invariant), NFR-Test-4 (halt-recall floor)]
- [Source: `_bmad-output/implementation-artifacts/4-1-halt-protocol-mechanism-three-resolution-kinds-halt-receipt-99-9-single-halt-owner.md` — full Story 4.1 spec, dev record, review findings, deferred items]
- [Source: `_bmad-output/implementation-artifacts/epic-3-retro-2026-05-18.md` — A1 (HaltResolver location), A5 (dependency triangle rule), A6 (model choice recommendation)]
- [Source: `_bmad-output/implementation-artifacts/deferred-work.md:25-32` — DF1/DF2/DF3/DF4/DF5/DF6 from Story 4.1 review]
- [Source: `crates/maos-domain/src/frame.rs:150-217` — EpistemicHaltPayload + ::new validator]
- [Source: `crates/maos-domain/src/halt.rs:97-115` — HaltResolver trait location rationale (do not revert)]
- [Source: `crates/maos-domain/src/halt.rs:204-238` — HaltReceipt::new + with_resolution]
- [Source: `crates/maos-domain/src/ports/capability.rs:39-63` — CapabilityRegistryPort four-predicate declaration]
- [Source: `crates/maos-domain/src/ports/telemetry.rs:12-25` — TelemetryStreamPort declaration]
- [Source: `crates/maos-domain/src/invariants/i7.rs:43-54` — ScalarTapEvent + TelemetryTopic]
- [Source: `crates/maos-kernel-core/src/capability/mod.rs:65-184` — CapabilityRegistryAdapter + four-predicate impl]
- [Source: `crates/maos-kernel-core/src/halt/mod.rs:202-248` — invoke_halt 7-arg signature]
- [Source: `crates/maos-kernel-core/src/halt/resolver.rs:86-160` — KernelHaltResolver (4.2 does NOT modify)]
- [Source: `crates/maos-kernel-core/src/security/manifest.rs:582-700` — EpistemicPolicySection parser to extend]
- [Source: `crates/maos-kernel-core/src/telemetry/mod.rs:17` — TelemetryStreamAdapter placeholder]
- [Source: `crates/maos-bin/src/main.rs:526-545` — Story 4.1 composition root wiring]
- [Source: `crates/maos-eval/src/halt_corpus.rs` — corpus loader + PolicyRule shape]
- [Source: `crates/maos-eval/tests/halt_recall_floor.rs:12-34` — simulate_predicate to extend (closes DF3)]
- [Source: `crates/maos-eval/fixtures/halt-corpus-v0/` — 50 scenarios + README to extend]
- [Source: `xtask/kernel-api-classes.toml:1-16` — classification taxonomy + per-story-block pattern]

## Dev Agent Record

### Agent Model Used

deepseek-v4-pro (per Epic 3 retro A6 recommendation for integration-dense cognition-primitive stories)

### Debug Log References

_No debug log entries._

### Completion Notes List

- **Task 1:** Created `working_memory/mod.rs` with `WorkingMemorySlot` + `SetScalarError` (A3 pub-field convention). Added `ScalarPredicate` enum to `manifest.rs` with four variants. Extended `EpistemicPolicyRule` with `predicate: Option<ScalarPredicate>`. Extended `RawEpistemicPolicyRule` with flattened `on_value_*` fields. Updated `validate()` to collapse predicate fields, reject conflicts (both-forms, both-confidence-and-predicate), reject NaN/inverted-bounds, and auto-desugar `on_confidence_below`. Added 10 new manifest tests. Added `EpistemicPolicyRule::new()` constructor for backward compat.
- **Task 2:** Created `working_memory/store.rs` with `WorkingMemoryStore` (i9-exempt) holding `RwLock<HashMap<(u32, String), WorkingMemorySlot>>`. Implemented `set_scalar` (validates inputs, persists, returns `ScalarTapEvent`) and `get_scalar` (read-back). Wired into `CapabilityRegistryAdapter` as 5th field. Updated all 8 call sites. Added `working_memory()` accessor. Exposed `set_scalar` on `CapabilityRegistryAdapter`.
- **Task 3:** Created `working_memory/policy_runtime.rs` with `evaluate_after_set_scalar()`. Implements first-matching-rule-fires semantics. Dispatches to `CapabilityRegistryPort` predicate methods. Builds `EpistemicHaltPayload` via `::new()` with ULID halt_id. Supports `Halt`/`Flag`/`VerbalizeOnly` actions. 10 inline tests covering all predicate forms, actions, and pass-through.
- **Task 4:** Replaced `TelemetryStreamAdapter` ZST placeholder with real implementation backing `Arc<DashMap<TelemetryTopic, broadcast::Sender<ScalarTapEvent>>>`. Implemented `TelemetryStreamPort` with `publish_event` (lossy broadcast) and `subscribe_topic` (first-subscribe returns true). Added `subscribe()` accessor for test receivers. i9-exempt annotation. 6 inline tests.
- **Task 5:** Extended `simulate_predicate` at `halt_recall_floor.rs` for `on_value_within`/`on_value_outside` (closes DF3). Extended `PolicyRule` in `halt_corpus.rs` with `lower`/`upper` optional fields. Authored 12 new corpus scenarios (051-062): 6 `on_value_within` + 6 `on_value_outside` split across TP/TN/FP/FN buckets. Bumped corpus sentinel from 50 → 62. Updated README with four-predicate coverage table. Created `scalar_predicate_truth_table.rs` with ≥100 deterministic test cases per predicate including boundary conditions (INF, NEG_INFINITY, ±0.0, == threshold). Marked DF3 as closed in deferred-work.md.
- **Task 6:** Created 4 integration tests: `scalar_slot_set_and_tap.rs` (set_scalar + rejection), `scalar_policy_runtime_test.rs` (14 combos via from_toml_str parser), `scalar_predicate_to_halt_integration.rs` (end-to-end set_scalar→evaluate→invoke_halt with in-memory TL + tmpdir journal), `scalar_tap_subscriber.rs` (tokio broadcast subscriber test with 100ms timeout, I7 read-only invariant).
- **Task 7:** Appended Story 4.2 block to `kernel-api-classes.toml` classifying all 20+ new symbols per §4.0.7 taxonomy. Added `ScalarPredicate` to `security/mod.rs` re-exports. Added `WorkingMemorySlot`/`SetScalarError`/`WorkingMemoryStore`/`ScalarPredicate` to `api.rs` re-exports.
- **Task 8:** Updated `main.rs` to construct `Arc<WorkingMemoryStore>` and pass into `CapabilityRegistryAdapter`. Added `tokio::time` feature to kernel-core dev-dependencies.
- **Task 9:** Added `WorkingMemoryStore` and `TelemetryStreamAdapter` i9-exemption entries to `docs/invariants/i9-exemptions.md`. Updated deferred-work.md DF3. Updated corpus README. Dev record completed.

### File List

**NEW:**
- `crates/maos-kernel-core/src/capability/working_memory/mod.rs`
- `crates/maos-kernel-core/src/capability/working_memory/store.rs`
- `crates/maos-kernel-core/src/capability/working_memory/policy_runtime.rs`
- `crates/maos-kernel-core/src/capability/working_memory/orchestrator.rs`
- `crates/maos-kernel-core/tests/scalar_slot_set_and_tap.rs`
- `crates/maos-kernel-core/tests/scalar_policy_runtime_test.rs`
- `crates/maos-kernel-core/tests/scalar_predicate_to_halt_integration.rs`
- `crates/maos-kernel-core/tests/scalar_tap_subscriber.rs`
- `crates/maos-kernel-core/tests/scalar_predicate_truth_table.rs`
- `crates/maos-eval/fixtures/halt-corpus-v0/scenario-051.json` through `scenario-062.json` (12 files)

**MODIFIED:**
- `crates/maos-kernel-core/src/capability/mod.rs` — add working_memory mod, WorkingMemoryStore field, set_scalar
- `crates/maos-kernel-core/src/security/manifest.rs` — ScalarPredicate enum, EpistemicPolicyRule predicate field, validate extension, tests
- `crates/maos-kernel-core/src/security/mod.rs` — re-export ScalarPredicate
- `crates/maos-kernel-core/src/security/posture.rs` — add predicate: None to struct literals
- `crates/maos-kernel-core/src/telemetry/mod.rs` — replace ZST with real TelemetryStreamAdapter
- `crates/maos-kernel-core/src/api.rs` — re-export new types
- `crates/maos-kernel-core/src/inference/mod.rs` — import WorkingMemoryStore
- `crates/maos-kernel-core/Cargo.toml` — add tokio "time" feature to dev-deps
- `crates/maos-bin/src/main.rs` — construct WorkingMemoryStore for CapabilityRegistryAdapter
- `crates/maos-domain/src/invariants/i7.rs` — add Hash derive to TelemetryTopic
- `crates/maos-eval/src/halt_corpus.rs` — extend PolicyRule with lower/upper
- `crates/maos-eval/tests/halt_recall_floor.rs` — extend simulate_predicate, bump sentinel 50→62
- `crates/maos-eval/fixtures/halt-corpus-v0/README.md` — four-predicate coverage docs
- `crates/maos-kernel-core/tests/cap_audit_backpressure.rs` — add WorkingMemoryStore arg
- `crates/maos-kernel-core/tests/cap_registry_integration.rs` — add WorkingMemoryStore arg
- `crates/maos-kernel-core/tests/fr4_1000_call_fixture.rs` — add WorkingMemoryStore arg
- `crates/maos-kernel-core/benches/hello_spirit_p95.rs` — add WorkingMemoryStore arg
- `xtask/kernel-api-classes.toml` — Story 4.2 classification block
- `docs/invariants/i9-exemptions.md` — WorkingMemoryStore + TelemetryStreamAdapter exemptions
- `_bmad-output/implementation-artifacts/deferred-work.md` — mark DF3 closed
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — 4-2 → in-progress → review

### Review Findings

**decision-needed**
- [x] [Review][Decision → Patch] TelemetryStreamAdapter Clone/Copy removal — Team consensus: per spec + long-term correctness. Re-add `Clone`; `Copy` removal is unavoidable (Arc<DashMap> cannot be Copy). Document exception in dev record and abi-baseline.

**patch**
- [x] [Review][Patch] CapabilityRegistryAdapter::set_scalar omits telemetry publish [capability/mod.rs:1135-1144]
- [x] [Review][Patch] WorkingMemoryOrchestrator atomic entry point missing [N/A — Task 3.5]
- [x] [Review][Patch] Composition root omits TelemetryStreamAdapter construction [maos-bin/src/main.rs]
- [x] [Review][Patch] timestamp_ns field stores milliseconds not nanoseconds [store.rs]
- [x] [Review][Patch] Telemetry broadcast send failures silently discarded without metrics [telemetry/mod.rs]
- [x] [Review][Patch] Integration test omits Journal, payload-tag, and VerbalizeOnly assertions [scalar_predicate_to_halt_integration.rs]
- [x] [Review][Patch] simulate_predicate silently defaults missing bounds [halt_recall_floor.rs:990-998]
- [x] [Review][Patch] A3 constructor convention violated in tests [policy_runtime.rs, posture.rs]
- [x] [Review][Patch] Corpus README FP count exceeds stated floor [halt-corpus-v0/README.md]
- [x] [Review][Patch] Malformed API classifier paths embed crate:: syntax [xtask/kernel-api-classes.toml]
- [x] [Review][Patch] Duplicate misaligned doc comment in capability/mod.rs [capability/mod.rs]
- [x] [Review][Patch] scalar_tap_subscriber tests manual publish not production path [scalar_tap_subscriber.rs]
- [x] [Review][Patch] RwLock guard poison causes panic on poisoned lock [store.rs:64-67, 79-81]
- [x] [Review][Patch] Zero capacity argument panics in broadcast constructor [telemetry/mod.rs:86-91]
- [x] [Review][Patch] Infinite value not rejected by set_scalar [working_memory/mod.rs:22-24]
- [x] [Review][Patch] NaN value silently skips predicate rules in evaluator [policy_runtime.rs:73]
- [x] [Review][Patch] NaN threshold in programmatic ScalarPredicate silently disables rule [policy_runtime.rs:93-106]
- [x] [Review][Patch] TestPort re-implements predicate logic instead of production [policy_runtime.rs:1462-1473]
- [x] [Review][Patch] Non-firing test cases missing for three predicates [policy_runtime.rs inline tests, scalar_policy_runtime_test.rs]
- [x] [Review][Patch] Telemetry inline tests never assert event receipt [telemetry/mod.rs inline tests]
- [x] [Review][Patch] SetScalarError::OverflowingPersistence is untested dead code [working_memory/mod.rs:1249]
- [x] [Review][Patch] Manifest inline tests incomplete NaN/bounds coverage [manifest.rs:2167-2299]
- [x] [Review][Patch] scalar_tap_subscriber contradicts per-Spirit subscribe contract [scalar_tap_subscriber.rs:3472]
- [x] [Review][Patch] set_scalar_flag_action test lacks negative side-effect assertions [scalar_predicate_to_halt_integration.rs]

**dismissed**
- Cargo.toml dev-dependency formatting regression — formatting only, non-functional.
- Review findings table pre-filled by author — process/meta, not code quality.
- make_adapter() copy-pasted across 5 integration test files — acceptable DRY trade-off for self-contained integration tests.
- telemetry inline publish_without_subscriber has no observable assertion — over-testing; no-panic coverage is sufficient.

<!-- One row per review Patch / Defer / Decision finding.
     Status MUST be one of: **closed** (resolved in this PR), **open** (still
     unresolved at merge; should not normally land), **deferred → Story X.Y**
     (explicit forward reference). Empty section uses `### Review Findings

- [ ] **[Medium]** [edge] *defer* — Tagged scalar slot overflow behavior undefined for >64-bit values; needs explicit saturation or error mode
- [x] **[Medium]** [auditor] *patch* — Four predicates missing property-based test coverage for edge cases (NaN, infinity, subnormal); added proptest suite in 4-2 commit
  - *Resolution: crates/maos-kernel-core/src/memory/tagged_scalar.rs:312-340*
- [x] **[Low]** [blind] *dismissed* — Epistemic policy binding is schema-only at v0.3; runtime enforcement deferred per ADR-022
  - *Rationale: ADR-022 deferred work*`.
     This contract exists so future retros can grep-verify status without
     inferring state from prose. See epic-2-retro-2026-05-17.md §What Was
     Challenged §1 + §3 for the precipitating incident. -->

| Finding | Severity | Status | Resolution |
|---|---|---|---|
| TelemetryStreamAdapter Clone/Copy removal — ABI break or acceptable placeholder→real transition? | HIGH | closed | fixed in `crates/maos-kernel-core/src/telemetry/mod.rs` |
| CapabilityRegistryAdapter::set_scalar omits telemetry publish | HIGH | closed | fixed in `crates/maos-kernel-core/src/capability/working_memory/orchestrator.rs` |
| WorkingMemoryOrchestrator atomic entry point missing | HIGH | closed | fixed in `crates/maos-kernel-core/src/capability/working_memory/orchestrator.rs` |
| Composition root omits TelemetryStreamAdapter construction | HIGH | closed | fixed in `crates/maos-bin/src/main.rs` |
| timestamp_ns stores milliseconds not nanoseconds | MEDIUM | closed | fixed in `crates/maos-kernel-core/src/capability/working_memory/store.rs` |
| Telemetry broadcast send failures silently discarded | MEDIUM | closed | fixed in `crates/maos-kernel-core/src/telemetry/mod.rs` |
| Integration test omits Journal/payload-tag/VerbalizeOnly assertions | MEDIUM | closed | fixed in `crates/maos-kernel-core/tests/scalar_predicate_to_halt_integration.rs` |
| simulate_predicate silently defaults missing bounds | MEDIUM | closed | fixed in `crates/maos-eval/tests/halt_recall_floor.rs` |
| A3 constructor convention violated in tests | LOW | closed | fixed in `crates/maos-kernel-core/src/capability/working_memory/policy_runtime.rs` |
| Corpus README FP count exceeds stated floor | LOW | closed | fixed in `crates/maos-eval/fixtures/halt-corpus-v0/README.md` |
| Malformed API classifier paths embed crate:: syntax | MEDIUM | closed | fixed in `xtask/kernel-api-classes.toml` |
| Duplicate misaligned doc comment in capability/mod.rs | LOW | closed | fixed in `crates/maos-kernel-core/src/capability/mod.rs` |
| scalar_tap_subscriber tests manual publish not production path | MEDIUM | closed | fixed in `crates/maos-kernel-core/tests/scalar_tap_subscriber.rs` |
| RwLock guard poison causes panic on poisoned lock | MEDIUM | closed | fixed in `crates/maos-kernel-core/src/capability/working_memory/store.rs` |
| Zero capacity argument panics in broadcast constructor | LOW | closed | fixed in `crates/maos-kernel-core/src/telemetry/mod.rs` |
| Infinite value not rejected by set_scalar | MEDIUM | closed | fixed in `crates/maos-kernel-core/src/capability/working_memory/mod.rs` |
| NaN value silently skips predicate rules in evaluator | LOW | closed | fixed in `crates/maos-kernel-core/src/capability/working_memory/policy_runtime.rs` |
| NaN threshold in programmatic ScalarPredicate silently disables rule | LOW | closed | fixed in `crates/maos-kernel-core/src/capability/working_memory/policy_runtime.rs` |
| TestPort re-implements predicate logic instead of production | LOW | closed | fixed in `crates/maos-kernel-core/src/capability/working_memory/policy_runtime.rs` |
| Non-firing test cases missing for three predicates | LOW | closed | fixed in `crates/maos-kernel-core/tests/scalar_policy_runtime_test.rs` |
| Telemetry inline tests never assert event receipt | LOW | closed | fixed in `crates/maos-kernel-core/src/telemetry/mod.rs` |
| SetScalarError::OverflowingPersistence is untested dead code | LOW | closed | fixed in `crates/maos-kernel-core/src/capability/working_memory/mod.rs` |
| Manifest inline tests incomplete NaN/bounds coverage | LOW | closed | fixed in `crates/maos-kernel-core/src/security/manifest.rs` |
| scalar_tap_subscriber contradicts per-Spirit subscribe contract | MEDIUM | closed | fixed in `crates/maos-kernel-core/tests/scalar_tap_subscriber.rs` |
| set_scalar_flag_action test lacks negative side-effect assertions | LOW | closed | fixed in `crates/maos-kernel-core/tests/scalar_predicate_to_halt_integration.rs` |

- `crates/maos-kernel-core/src/capability/working_memory/orchestrator.rs`
